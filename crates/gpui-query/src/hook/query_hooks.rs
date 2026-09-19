//! Plain-query fetch tasks are deliberately detached: stale writes are guarded
//! by the two-phase `accept_current_request` protocol, and each task holds
//! only a `WeakEntity`, so it self-terminates on entity drop.

use gpui::{BorrowAppContext as _, Context, Entity, Subscription};

use crate::client::{QueryClient, QueryObserver};
use crate::core::{Fetched, QueryFetchMode, QueryKey, QueryResource, QuerySignal, QueryStatus};

use super::current_time_ms;
use super::fetch_retry::{
    FetchedLike, begin_request_on_entity, fetch_signal_with_retry, fetch_with_retry,
};

/// Creates or reuses the resource in the global [`QueryClient`] and spawns a
/// fetch if it is idle; call it in a constructor, never in `render`.
pub fn use_query<T, E, C, F, Fut>(
    options: impl Into<crate::hook::QueryOptions>,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(QuerySignal) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    use_query_impl(options.into(), fetcher, cx)
}

/// A fetcher returning [`Fetched::with_policy`](crate::core::Fetched::with_policy)
/// overrides the resource's stored policy right after success (server wins).
pub fn use_query_with_policy<T, E, C, F, Fut>(
    options: impl Into<crate::hook::QueryOptions>,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(QuerySignal) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Fetched<T>, E>> + Send + 'static,
{
    use_query_impl(options.into(), fetcher, cx)
}

fn use_query_impl<T, E, C, F, Fut, Out>(
    options: crate::hook::QueryOptions,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    Out: FetchedLike<T> + Send + 'static,
    F: Fn(QuerySignal) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Out, E>> + Send + 'static,
{
    let crate::hook::QueryOptions {
        key,
        cache_policy,
        request_policy,
        retry_policy,
        force_fetch,
        ..
    } = options;
    let (entity, subscription) = use_query_manual(key.clone(), cache_policy, request_policy, cx);

    entity.update(cx, |r, _| r.set_retry_policy(retry_policy.clone()));

    if entity.read_with(cx, |r, _| r.status() == QueryStatus::Idle) {
        let fetch_mode = if force_fetch {
            QueryFetchMode::Force
        } else {
            QueryFetchMode::Normal
        };
        if let (Some(request_id), signal) =
            begin_request_on_entity(&entity, cx, fetch_mode, Some(key))
        {
            let signal = signal.unwrap_or_else(QuerySignal::new);
            let weak = entity.downgrade();
            let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
                fetch_signal_with_retry(fetcher, signal, request_id, &retry_policy, &weak, cx)
                    .await;
            });
            task.detach();
        }
    }

    (entity, subscription)
}

/// Signal-free fetcher variant; prefer the signal-accepting [`use_query`].
pub fn use_query_unsignalled<T, E, C, F, Fut>(
    key: QueryKey,
    cache_policy: crate::core::CachePolicy,
    request_policy: crate::core::RequestPolicy,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let (entity, subscription) = use_query_manual(key.clone(), cache_policy, request_policy, cx);

    if entity.read_with(cx, |r, _| r.status() == QueryStatus::Idle)
        && let (Some(request_id), _signal) =
            begin_request_on_entity(&entity, cx, QueryFetchMode::Normal, Some(key))
    {
        let weak = entity.downgrade();
        let retry_policy = entity.read_with(cx, |r, _| r.retry_policy().clone());
        let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
            fetch_with_retry(fetcher, request_id, &retry_policy, &weak, cx).await;
        });
        task.detach();
    }

    (entity, subscription)
}

/// Consumes only `key`, `cache_policy`, and `request_policy`; use
/// [`use_query`] to honor the rest.
pub fn use_query_manual_opts<T, E, C>(
    options: impl Into<crate::hook::QueryOptions>,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    let opts = options.into();
    use_query_manual(opts.key, opts.cache_policy, opts.request_policy, cx)
}

pub fn use_query_unsignalled_opts<T, E, C, F, Fut>(
    options: impl Into<crate::hook::QueryOptions>,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let opts = options.into();
    use_query_unsignalled(
        opts.key,
        opts.cache_policy,
        opts.request_policy,
        fetcher,
        cx,
    )
}

/// Entity + observer without starting a fetch; panics in debug builds when no [`QueryClient`] global is set (release falls back to a standalone entity).
pub fn use_query_manual<T, E, C>(
    key: QueryKey,
    cache_policy: crate::core::CachePolicy,
    request_policy: crate::core::RequestPolicy,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    let entity = if cx.has_global::<QueryClient>() {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.resource_with_policies::<T, E>(key, cache_policy, request_policy, cx)
        })
    } else {
        #[cfg(debug_assertions)]
        {
            eprintln!(
                "use_query_manual: no QueryClient set via cx.set_global(). \
                 Falling back to standalone entity (no shared caching, no GC). \
                 Call cx.set_global(QueryClient::new()) in your app setup."
            );
            panic!(
                "use_query_manual: QueryClient is not initialized. \
                 Call cx.set_global(QueryClient::new()) before using query hooks."
            );
        }
        #[cfg(not(debug_assertions))]
        {
            cx.new(|_| QueryResource::new(key, cache_policy, request_policy))
        }
    };

    let observer = QueryObserver::new(&entity);
    let Some(subscription) = observer.observe(cx) else {
        #[cfg(debug_assertions)]
        panic!(
            "QueryObserver::observe failed: entity was just created and cannot be dropped. \
             This indicates a GPUI internal regression."
        );
        #[cfg(not(debug_assertions))]
        {
            return (entity, Subscription::new(|| {}));
        }
    };

    (entity, subscription)
}

/// No-op when the cache is fresh or a fetch is already loading; otherwise
/// spawns a retry-aware fetch.
pub fn fetch_query<T, E, C, F, Fut>(
    entity: &Entity<QueryResource<T, E>>,
    fetcher: F,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    fetch_query_impl(entity, fetcher, cx);
}

/// [`fetch_query`] whose fetcher may return [`Fetched<T>`](crate::core::Fetched)
/// to override the resource's policy on success.
pub fn fetch_query_with_policy<T, E, C, F, Fut>(
    entity: &Entity<QueryResource<T, E>>,
    fetcher: F,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Fetched<T>, E>> + Send + 'static,
{
    fetch_query_impl(entity, fetcher, cx);
}

fn fetch_query_impl<T, E, C, F, Fut, Out>(
    entity: &Entity<QueryResource<T, E>>,
    fetcher: F,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    Out: FetchedLike<T> + Send + 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Out, E>> + Send + 'static,
{
    let (Some(request_id), _signal) =
        begin_request_on_entity(entity, cx, QueryFetchMode::Normal, None)
    else {
        return;
    };
    let weak = entity.downgrade();
    let retry_policy = entity.read_with(cx, |r, _| r.retry_policy().clone());
    let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
        fetch_with_retry(fetcher, request_id, &retry_policy, &weak, cx).await;
    });
    task.detach();
}

/// `FnOnce` fetcher, so no retries; staleness is guarded by
/// `accept_current_request`, not a post-fetch `is_cancelled()` check (racy).
pub fn fetch_query_with_signal<T, E, C, F, Fut>(
    entity: &Entity<QueryResource<T, E>>,
    fetcher: F,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: FnOnce(QuerySignal) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let (Some(request_id), signal) =
        begin_request_on_entity(entity, cx, QueryFetchMode::Normal, None)
    else {
        return;
    };
    let signal = signal.unwrap_or_else(QuerySignal::new);
    let weak = entity.downgrade();

    let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
        let result = fetcher(signal).await;

        let now_ms = current_time_ms();
        let Some(entity) = weak.upgrade() else { return };

        let _ = entity.update(cx, |resource, cx| {
            if let Some(guard) = resource.accept_current_request(request_id) {
                match result {
                    Ok(data) => {
                        resource.complete_success(guard, data, now_ms);
                    }
                    Err(error) => {
                        resource.complete_failure(guard, error, now_ms);
                    }
                }
                cx.notify();
            } else {
                #[cfg(debug_assertions)]
                eprintln!(
                    "DEBUG: fetch_query_with_signal: request {} no longer active, result discarded",
                    request_id.label()
                );
            }
        });
    });
    task.detach();
}
