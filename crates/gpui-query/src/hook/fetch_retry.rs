//! Request lifecycle helpers and retry-aware fetch functions for query resources.

use gpui::{BorrowAppContext as _, Context, Entity};

use crate::client::QueryClient;
use crate::core::{
    CachePolicy, Fetched, QueryBeginResult, QueryFetchMode, QueryKey, QueryResource, QuerySignal,
    RequestId, RetryPolicy,
};

use super::{current_time_ms, read_entity};

/// Decomposed fetcher success: the data, an optional server-derived
/// [`CachePolicy`], and (under `persist`) optional opaque metadata.
pub(crate) struct FetchParts<T> {
    pub data: T,
    pub server_policy: Option<CachePolicy>,
    #[cfg(feature = "persist")]
    pub meta: Option<serde_json::Value>,
}

/// Adapt a fetcher success payload into [`FetchParts`]. Plain `T` yields no
/// server policy; [`Fetched<T>`] carries both optional extras.
pub(crate) trait FetchedLike<T> {
    fn into_parts(self) -> FetchParts<T>;
}

impl<T> FetchedLike<T> for T {
    fn into_parts(self) -> FetchParts<T> {
        FetchParts {
            data: self,
            server_policy: None,
            #[cfg(feature = "persist")]
            meta: None,
        }
    }
}

impl<T> FetchedLike<T> for Fetched<T> {
    fn into_parts(self) -> FetchParts<T> {
        FetchParts {
            data: self.data,
            server_policy: self.cache_policy,
            #[cfg(feature = "persist")]
            meta: self.meta,
        }
    }
}

/// Begin a request on a query entity: runs the cache-freshness /
/// `IgnoreWhileLoading` check and the `Loading` transition atomically in one
/// `entity.update`, and reads the freshly created signal in the same pass.
///
/// Returns `(Some(request_id), Some(signal))` when a fetch should be spawned;
/// `(None, None)` on `CacheHit` / `IgnoredWhileLoading` (skip the fetch).
///
/// When a [`QueryClient`] global is present, the bucket's co-located sequencer
/// mints the `RequestId` (shared with the imperative `prepare_fetch_query`
/// path so the two never collide for the same key); otherwise
/// `begin_request_with_id` falls back to the resource's own monotonic
/// sequencer. `known_key` spares callers that already hold the key a re-read.
pub(crate) fn begin_request_on_entity<T, E, C>(
    entity: &Entity<QueryResource<T, E>>,
    cx: &mut Context<C>,
    fetch_mode: QueryFetchMode,
    known_key: Option<QueryKey>,
) -> (Option<RequestId>, Option<QuerySignal>)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    let now_ms = current_time_ms();

    let maybe_request_id = if cx.has_global::<QueryClient>() {
        let key = known_key.unwrap_or_else(|| entity.read_with(cx, |r, _| r.key().clone()));
        cx.update_global::<QueryClient, _>(|client, _cx| {
            client.next_request_id_for_key::<T, E>(&key)
        })
    } else {
        None
    };

    entity.update(cx, |resource, _cx| {
        match resource.begin_request_with_id(maybe_request_id, now_ms, fetch_mode) {
            QueryBeginResult::Started { request_id, .. }
            | QueryBeginResult::StaleCacheHit { request_id, .. } => {
                let signal = resource.signal().cloned();
                (Some(request_id), signal)
            }
            QueryBeginResult::CacheHit | QueryBeginResult::IgnoredWhileLoading { .. } => {
                (None, None)
            }
        }
    })
}

/// Single retry loop shared by every query fetch shape.
///
/// `signal` is `None` for signal-less fetchers; `Some(initial)` re-reads a
/// fresh signal from the resource after each retry delay. After each delay the
/// loop checks `is_current_request` and stops early if a newer request has
/// superseded this one. `cx.notify()` fires only when a result is actually
/// accepted, and `entity.update` results are discarded because `update`
/// returns `Result<R>` under `AsyncApp`.
async fn run_query_retry_loop<T, E, Out, F, Fut>(
    fetcher: F,
    request_id: RequestId,
    retry_policy: &RetryPolicy,
    entity: &gpui::WeakEntity<QueryResource<T, E>>,
    cx: &mut gpui::AsyncApp,
    mut signal: Option<QuerySignal>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    Out: FetchedLike<T> + Send + 'static,
    F: Fn(Option<QuerySignal>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Out, E>> + Send + 'static,
{
    let mut attempt: u32 = 0;

    loop {
        let result = fetcher(signal.clone()).await;

        match result {
            Ok(out) => {
                let parts = out.into_parts();
                #[cfg(feature = "persist")]
                let meta = parts.meta;
                let now_ms = current_time_ms();
                let Some(e) = entity.upgrade() else {
                    // Owning component unmounted: result is silently discarded.
                    return;
                };
                let _ = e.update(cx, |resource, cx| {
                    resource.reset_retry_count();
                    if let Some(guard) = resource.accept_current_request(request_id) {
                        resource.complete_success(guard, parts.data, now_ms);
                        // Server wins: a fetcher-supplied policy overrides the
                        // resource's stored one.
                        if let Some(policy) = parts.server_policy {
                            resource.set_cache_policy(policy);
                        }
                        cx.notify();
                        #[cfg(feature = "persist")]
                        cx.default_global::<crate::client::CacheMutation>();
                        #[cfg(feature = "persist")]
                        if let Some(meta) = meta {
                            let key = resource.key().clone();
                            cx.update_global::<QueryClient, _>(|client, _| {
                                client.record_meta(key, meta);
                            });
                        }
                    } else {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "DEBUG: run_query_retry_loop: request {} no longer active on success, result discarded",
                            request_id.label()
                        );
                    }
                });
                return;
            }
            Err(error) => {
                if retry_policy.should_retry(attempt) {
                    let delay_ms = retry_policy.delay_for_attempt(attempt);
                    let Some(e) = entity.upgrade() else { return };
                    // No notify: retry counters do not change status (stays
                    // Loading), and the observer dedupes on status.
                    let _ = e.update(cx, |resource, _cx| {
                        resource.increment_retry();
                    });
                    attempt += 1;

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    let Some(e) = entity.upgrade() else { return };
                    let (request_still_active, fresh_signal) = read_entity(&e, cx, |r, _| {
                        (
                            r.is_current_request(request_id),
                            r.signal().cloned().unwrap_or_else(QuerySignal::new),
                        )
                    })
                    .unwrap_or_else(|| (false, QuerySignal::new()));
                    if !request_still_active {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "DEBUG: run_query_retry_loop: request {} no longer active after retry delay, aborting retry",
                            request_id.label()
                        );
                        return;
                    }
                    if let Some(ref mut sig) = signal {
                        *sig = fresh_signal;
                    }
                } else {
                    let Some(e) = entity.upgrade() else { return };
                    let failure_now_ms = current_time_ms();
                    let _ = e.update(cx, |resource, cx| {
                        if let Some(guard) = resource.accept_current_request(request_id) {
                            resource.complete_failure(guard, error, failure_now_ms);
                            // Reset so the next begin_request starts clean.
                            resource.reset_retry_count();
                            cx.notify();
                            #[cfg(feature = "persist")]
                            cx.default_global::<crate::client::CacheMutation>();
                        } else {
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "DEBUG: run_query_retry_loop: request {} no longer active on failure, result discarded",
                                request_id.label()
                            );
                        }
                    });
                    return;
                }
            }
        }
    }
}

/// Fetch with retry for a query resource (no-signal fetcher): a thin wrapper
/// over [`run_query_retry_loop`] with `signal = None`.
pub(crate) async fn fetch_with_retry<T, E, Out, F, Fut>(
    fetcher: F,
    request_id: RequestId,
    retry_policy: &RetryPolicy,
    entity: &gpui::WeakEntity<QueryResource<T, E>>,
    cx: &mut gpui::AsyncApp,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    Out: FetchedLike<T> + Send + 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Out, E>> + Send + 'static,
{
    let wrapper = move |_: Option<QuerySignal>| fetcher();
    run_query_retry_loop::<T, E, Out, _, _>(wrapper, request_id, retry_policy, entity, cx, None)
        .await;
}

/// Like [`fetch_with_retry`] but for fetchers that take a [`QuerySignal`].
/// On retry, a fresh signal is read from the resource and handed to the
/// fetcher.
pub(crate) async fn fetch_signal_with_retry<T, E, Out, F, Fut>(
    fetcher: F,
    initial_signal: QuerySignal,
    request_id: RequestId,
    retry_policy: &RetryPolicy,
    entity: &gpui::WeakEntity<QueryResource<T, E>>,
    cx: &mut gpui::AsyncApp,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    Out: FetchedLike<T> + Send + 'static,
    F: Fn(QuerySignal) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Out, E>> + Send + 'static,
{
    let wrapper = move |sig: Option<QuerySignal>| fetcher(sig.unwrap_or_default());
    run_query_retry_loop::<T, E, Out, _, _>(
        wrapper,
        request_id,
        retry_policy,
        entity,
        cx,
        Some(initial_signal),
    )
    .await;
}
