//! The `use_infinite_query` hook: infinite scrolling / pagination for GPUI
//! components.
//!
//! # Usage
//!
//! ```no_run
//! use gpui_query::hook::{use_infinite_query, fetch_next_page_infinite, InfiniteQueryOptions};
//! use gpui_query::QueryKey;
//! # #[derive(Clone)]
//! # struct Post { id: u64 }
//! # #[derive(Clone, Debug)]
//! # struct MyError;
//!
//! struct FeedView {
//!     feed: gpui::Entity<gpui_query::InfiniteQueryResource<Vec<Post>, MyError>>,
//!     _subscription: gpui::Subscription,
//! }
//!
//! impl FeedView {
//!     fn new(cx: &mut gpui::Context<Self>) -> Self {
//!         let (entity, _subscription) = use_infinite_query(
//!             InfiniteQueryOptions::new(QueryKey::from(["feed"])),
//!             |last_page| async move {
//!                 // Your async fetcher here
//!                 Ok((vec![], false))
//!             },
//!             cx,
//!         );
//!         Self { feed: entity, _subscription }
//!     }
//!
//!     fn on_scroll_to_bottom(&mut self, cx: &mut gpui::Context<Self>) {
//!         fetch_next_page_infinite(
//!             &self.feed,
//!             |last_page| async move {
//!                 // Your async fetcher here
//!                 Ok((vec![], false))
//!             },
//!             cx,
//!         );
//!     }
//! }
//! ```

use gpui::{AppContext as _, BorrowAppContext as _, Context, Entity, Subscription};

use crate::client::{InfiniteQueryObserver, QueryClient};
use crate::core::{InfiniteQueryResource, QueryStatus};

use super::fetch_runners::run_fetch_next_page_with_id;
use crate::hook::current_time_ms;
use crate::hook::options::InfiniteQueryOptions;

/// Hook for infinite scrolling / pagination.
///
/// Creates an [`InfiniteQueryResource`] entity (registered with
/// [`QueryClient`] for shared caching, GC, and bulk invalidation) and
/// subscribes via an observer that dedupes on status, so intermediate retry
/// updates do not re-render. Returns the entity and the subscription; store
/// both. The retry policy from options is stored on the entity and applied to
/// every page fetch.
///
/// The fetcher receives `Option<&T>` (the last page, if any) and returns
/// `Result<(T, bool), E>` where the bool says whether more pages exist.
pub fn use_infinite_query<T, E, C, FNext, Fut>(
    options: InfiniteQueryOptions,
    fetch_next: FNext,
    cx: &mut Context<C>,
) -> (Entity<InfiniteQueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    FNext: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    let InfiniteQueryOptions {
        key,
        cache_policy,
        request_policy,
        max_pages,
        retry_policy,
        ..
    } = options;

    let entity = if cx.has_global::<QueryClient>() {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.infinite_resource_with_policies::<T, E>(key, cache_policy, request_policy, cx)
        })
    } else {
        #[cfg(debug_assertions)]
        {
            eprintln!(
                "use_infinite_query: no QueryClient set via cx.set_global(). \
                 Falling back to standalone entity (no shared caching, no GC). \
                 Call cx.set_global(QueryClient::new()) in your app setup."
            );
        }
        // max_pages and retry_policy are applied unconditionally below for
        // both paths.
        cx.new(|_| InfiniteQueryResource::new(key, cache_policy, request_policy))
    };

    // QueryClient-created entities don't set max_pages; apply it and store
    // the retry policy in one pass.
    entity.update(cx, |resource, cx| {
        if let Some(max) = max_pages {
            resource.set_max_pages(Some(max));
        }
        resource.set_retry_policy(retry_policy.clone());
        cx.notify();
    });

    let observer = InfiniteQueryObserver::new(&entity);

    let Some(subscription) = observer.observe(cx) else {
        #[cfg(debug_assertions)]
        panic!(
            "InfiniteQueryObserver::observe failed: entity was just created and \
             cannot be dropped. This indicates a GPUI internal regression."
        );
        #[cfg(not(debug_assertions))]
        {
            return (entity, Subscription::new(|| {}));
        }
    };

    // Start the initial fetch if idle
    if entity.read_with(cx, |r, _| r.status() == QueryStatus::Idle) {
        // The initial fetch also mints from the bucket's sequencer, so later
        // fetch_next/previous_page ids continue the same sequence.
        let maybe_request_id = if cx.has_global::<QueryClient>() {
            let key = entity.read_with(cx, |r, _| r.key().clone());
            cx.update_global::<QueryClient, _>(|client, _| {
                client.next_request_id_for_infinite_key::<T, E>(&key)
            })
        } else {
            None
        };

        let request_id = entity.update(cx, |resource, _| {
            let now_ms = current_time_ms();
            resource.begin_fetch_next_with_id(maybe_request_id, now_ms)
        });

        if let Some(request_id) = request_id {
            let weak = entity.downgrade();
            let fetcher = fetch_next;
            let retry = retry_policy.clone();
            let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
                run_fetch_next_page_with_id(&weak, &fetcher, request_id, &retry, cx).await;
            });
            entity.update(cx, |r, _| r.set_current_task(task));
        }
    }

    (entity, subscription)
}
