//! Public fetch helpers for infinite query entities, called from event
//! handlers (scroll-to-bottom, button click) to fetch the next or previous
//! page.

use gpui::{BorrowAppContext as _, Context, Entity};

use crate::client::QueryClient;
use crate::core::InfiniteQueryResource;

use super::fetch_runners::{
    PageDirection, run_fetch_next_page_with_id, run_fetch_previous_page_with_id,
};
use crate::hook::current_time_ms;

/// Initiate a fetch of the next page on an existing infinite query entity.
///
/// Reads the last page from the entity and passes it to the fetcher. Applies
/// the retry policy stored on the entity; if a fetch is already in flight the
/// old signal is cancelled and the new request supersedes it.
///
/// # Example
///
/// ```no_run
/// use gpui_query::hook::fetch_next_page_infinite;
/// # #[derive(Clone)]
/// # struct Item;
/// # #[derive(Clone, Debug)]
/// # struct MyError;
/// # fn _doc(entity: &gpui::Entity<gpui_query::InfiniteQueryResource<Vec<Item>, MyError>>, cx: &mut gpui::Context<()>) {
///
/// fetch_next_page_infinite(entity, |last_page| async move {
///     Ok((vec![], false))
/// }, cx);
/// # }
/// ```
pub fn fetch_next_page_infinite<T, E, C, FNext, Fut>(
    entity: &Entity<InfiniteQueryResource<T, E>>,
    fetcher: FNext,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    FNext: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    fetch_page_infinite(entity, fetcher, cx, PageDirection::Next);
}

/// Initiate a fetch of the previous page on an existing infinite query entity.
///
/// Like [`fetch_next_page_infinite`] but backward: the fetcher receives the
/// first page (not the last) so it can determine the cursor for the previous
/// page.
pub fn fetch_previous_page_infinite<T, E, C, FPrev, Fut>(
    entity: &Entity<InfiniteQueryResource<T, E>>,
    fetcher: FPrev,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    FPrev: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    fetch_page_infinite(entity, fetcher, cx, PageDirection::Previous);
}

/// Shared body of [`fetch_next_page_infinite`] / [`fetch_previous_page_infinite`];
/// only `direction` differs (which `begin_fetch_*` call and which runner).
fn fetch_page_infinite<T, E, C, F, Fut>(
    entity: &Entity<InfiniteQueryResource<T, E>>,
    fetcher: F,
    cx: &mut Context<C>,
    direction: PageDirection,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    let weak = entity.downgrade();

    // Mint the RequestId from the bucket's persistent sequencer so ids stay
    // monotonic; pass it into begin_fetch_*_with_id so the resource's
    // active_request_id matches the bucket's counter. Falls back to None
    // (transient sequencer) when no QueryClient is available.
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
        match direction {
            PageDirection::Next => resource.begin_fetch_next_with_id(maybe_request_id, now_ms),
            PageDirection::Previous => {
                resource.begin_fetch_previous_with_id(maybe_request_id, now_ms)
            }
        }
    });

    if let Some(request_id) = request_id {
        let retry_policy = entity.read_with(cx, |r, _| r.retry_policy().clone());
        // Stored on the resource so a replacement fetch or unmount aborts the
        // prior in-flight task.
        let task: gpui::Task<()> = cx.spawn(async move |_this, cx| match direction {
            PageDirection::Next => {
                run_fetch_next_page_with_id(&weak, &fetcher, request_id, &retry_policy, cx).await;
            }
            PageDirection::Previous => {
                run_fetch_previous_page_with_id(&weak, &fetcher, request_id, &retry_policy, cx)
                    .await;
            }
        });
        entity.update(cx, |r, _| r.set_current_task(task));
    }
}
