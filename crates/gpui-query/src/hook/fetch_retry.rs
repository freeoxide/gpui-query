//! Request lifecycle helpers and retry-aware fetch functions for query resources.

use gpui::{BorrowAppContext as _, Context, Entity};

use crate::client::QueryClient;
use crate::core::{
    CachePolicy, Fetched, QueryBeginResult, QueryFetchMode, QueryKey, QueryResource, QuerySignal,
    RequestId, RetryPolicy,
};

use super::{current_time_ms, read_entity};

// ── Fetcher-success adapter ("server wins") ─────────────────────────────
//
// Lets the single retry loop below serve both fetcher shapes:
// - `Result<T, E>`         (plain)        → no policy override
// - `Result<Fetched<T>, E>` (`*_with_policy`) → optional server-derived policy
//
// The loop is generic over `Out: FetchedLike<T>`; the success arm extracts
// `(data, server_policy)` from the fetcher result and, when a server policy is
// present, applies it to the resource after `complete_success`. A plain `T`
// yields `(self, None)`, so the existing fetch paths are behaviorally identical.

/// Decomposed fetcher success: the underlying data, an optional server-derived
/// [`CachePolicy`], and (under `persist`) optional opaque metadata sourced from
/// [`Fetched::meta`](crate::core::Fetched).
pub(crate) struct FetchParts<T> {
    pub data: T,
    pub server_policy: Option<CachePolicy>,
    #[cfg(feature = "persist")]
    pub meta: Option<serde_json::Value>,
}

/// Adapt a fetcher success payload into its decomposed [`FetchParts`].
pub(crate) trait FetchedLike<T> {
    /// Consume `self` into [`FetchParts`]; `server_policy` (and `meta` under
    /// `persist`) are `None` for plain fetcher results.
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

// ── Request lifecycle helpers ───────────────────────────────────────────

/// Call `begin_request` on a query entity.
///
/// This transitions the resource to a Loading status, creates a fresh signal,
/// and returns `Some(RequestId)` that must be used for completion.
///
/// Returns `None` when the resource does not need fetching (cache hit, ignored
/// while loading). The caller should skip spawning the async fetch task when
/// this returns `None`.
///
/// Audit fix #62: For `QueryFetchMode::Normal`, uses the new
/// `QueryResource::try_begin_request(now_ms)` (core contract #4) to perform
/// the cache-freshness / `IgnoreWhileLoading` check and the `Loading`
/// transition atomically inside a single `entity.update`. This closes the
/// read-then-update race window where a concurrent caller could change state
/// between the check and the begin. The `CacheHit` / `Started` /
/// `StaleCacheHit` / `IgnoredWhileLoading` distinction is preserved via
/// [`QueryBeginResult`].
///
/// Audit fix #2: Accepts an optional `known_key` so callers that already hold
/// the key avoid re-reading it from the entity; otherwise it is read once here.
///
/// Audit H3: the result now also carries the `Option<QuerySignal>` that
/// `begin_request` just created, read from the SAME `entity.update` closure.
/// Callers previously did a SEPARATE `entity.read_with(|r, _| r.signal()…)`
/// afterwards — this fuses those two reads into one. The returned signal is the
/// one `begin_request` just created (Started/StaleCacheHit); `CacheHit` /
/// `IgnoredWhileLoading` return `None` for both slots, matching the prior
/// "skip the fetch" semantics.
///
/// `RequestId` source: when a `QueryClient` global is available, the bucket's
/// co-located sequencer mints a monotonic `RequestId` (shared with the
/// imperative `prepare_fetch_query` path, so the two never collide); otherwise
/// `begin_request_with_id` falls back to the resource's own stored sequencer
/// (the N3 fix, monotonic per-resource). Done for BOTH fetch modes so
/// Normal-mode fetches also receive unique, monotonic ids.
///
/// Note (audit H5 reverted): an earlier pass minted lazily from the resource's
/// own sequencer even when a `QueryClient` was present. That split the ID space
/// from the imperative fetch path — both sequencers start at scope 1 — so a
/// hook fetch and an imperative `prepare_fetch_query` on the same key could
/// both mint `RequestId(1,1)` and defeat stale-request rejection. The bucket
/// sequencer is restored here; the minor cost is one consumed sequence number
/// on `CacheHit` / `IgnoredWhileLoading` paths, which is not worth the
/// correctness risk.
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

    // Use the bucket's co-located sequencer for a monotonic `RequestId` when a
    // QueryClient is available; otherwise fall back to the resource's stored
    // sequencer inside `begin_request_with_id` (N3). Shared with the imperative
    // fetch path, so ids never collide across hook/imperative for the same key.
    let maybe_request_id = if cx.has_global::<QueryClient>() {
        let key = known_key.unwrap_or_else(|| entity.read_with(cx, |r, _| r.key().clone()));
        cx.update_global::<QueryClient, _>(|client, _cx| {
            client.next_request_id_for_key::<T, E>(&key)
        })
    } else {
        None
    };

    // Audit fix #62: the cache-freshness / IgnoreWhileLoading check and the
    // Loading transition happen atomically inside this single `entity.update`
    // closure, so a concurrent caller cannot slip a state mutation in between
    // the check and the begin.
    //
    // Audit H3: ALSO read `resource.signal().cloned()` inside the SAME closure
    // for the Started / StaleCacheHit branches (the ones that return a real
    // `RequestId`), so callers do not need a second read pass to obtain the
    // signal. For `Started` this is the signal `begin_request` just created;
    // for the `IgnoreWhileLoading`-active `StaleCacheHit` sub-branch
    // `begin_request` returns early and reuses the active request, so this is
    // that existing signal — in both cases the correct one for the caller to
    // poll for cancellation.
    entity.update(cx, |resource, _cx| {
        match resource.begin_request_with_id(maybe_request_id, now_ms, fetch_mode) {
            QueryBeginResult::Started { request_id, .. } => {
                let signal = resource.signal().cloned();
                (Some(request_id), signal)
            }
            QueryBeginResult::StaleCacheHit { request_id, .. } => {
                let signal = resource.signal().cloned();
                (Some(request_id), signal)
            }
            QueryBeginResult::CacheHit => (None, None),
            QueryBeginResult::IgnoredWhileLoading { .. } => (None, None),
        }
    })
}

// ── Retry-aware fetch helpers ───────────────────────────────────────────

/// Unified retry loop for query fetches.
///
/// Audit fix #14: The previous `fetch_with_retry` and `fetch_signal_with_retry`
/// were ~90% duplicated (only signal-handling differed). Both now delegate to
/// this single loop parameterized by `signal: Option<QuerySignal>`:
/// - `None` → the no-signal variant (`Fn() -> Fut` fetcher wrapped to ignore
///   the signal slot; no fresh-signal re-read on retry).
/// - `Some(initial)` → the signal variant (`Fn(QuerySignal) -> Fut` fetcher
///   wrapped to unwrap the slot; fresh signal re-read after each retry delay).
///
/// Audit fix #6: After each retry delay, checks whether the request has been
/// cancelled (e.g., by a newer `begin_request` under `LatestWins`). If so,
/// breaks out of the retry loop immediately to avoid unnecessary work.
///
/// Audit fix #7: `cx.notify()` is only called when `accept_current_request`
/// succeeds (i.e., the result is actually accepted by the current request
/// slot). Discarded results do not trigger a re-render.
///
/// Audit fix #24: `unwrap_or_else(QuerySignal::new)` instead of the
/// redundant-closure `unwrap_or_else(|| QuerySignal::new())`.
///
/// Audit fix #27/#121: `entity.update` results are discarded via `let _ =`
/// because `update` returns `Result<R>` under `AsyncApp`.
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
                // Server wins: extract any server-derived policy (and, under
                // `persist`, opaque metadata) from a `Fetched<T>` result. Plain
                // `T` results yield `None` for both.
                let parts = out.into_parts();
                let data = parts.data;
                let server_policy = parts.server_policy;
                #[cfg(feature = "persist")]
                let meta = parts.meta;
                let now_ms = current_time_ms();
                let Some(e) = entity.upgrade() else {
                    // Documented behavior -- if the owning component
                    // was unmounted, the result is silently discarded.
                    return;
                };
                let _ = e.update(cx, |resource, cx| {
                    resource.reset_retry_count();
                    if let Some(guard) = resource.accept_current_request(request_id) {
                        resource.complete_success(guard, data, now_ms);
                        // Server wins: override the resource's policy only when
                        // the fetcher supplied one. No-op for plain `T` results.
                        if let Some(policy) = server_policy {
                            resource.set_cache_policy(policy);
                        }
                        // Audit fix #7: Only notify when the result was actually accepted.
                        cx.notify();
                        // B2: precise dirty signal for the persistence layer.
                        #[cfg(feature = "persist")]
                        cx.default_global::<crate::client::CacheMutation>();
                        // L1: carry fetcher-supplied opaque metadata (e.g. an
                        // HTTP CacheMeta) into the persistence layer's per-key
                        // map so it round-trips through PersistedEntry.meta.
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
                    let _ = e.update(cx, |resource, _cx| {
                        resource.increment_retry();
                        // No cx.notify() here -- increment_retry does not change
                        // status (stays Loading). The QueryObserver handles
                        // status-deduplication so this update does not trigger
                        // a re-render.
                    });
                    attempt += 1;

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    // Audit fix #6: After the retry delay, check whether the
                    // request has been cancelled (e.g., by a newer begin_request
                    // under LatestWins). If so, stop retrying immediately.
                    // Audit H6: fuse the cancellation check and the fresh-signal
                    // re-read into a single read_entity pass (was two sequential
                    // reads). `fresh_signal` is computed unconditionally but only
                    // used when `signal` is `Some` (audit fix #14).
                    let Some(e) = entity.upgrade() else { return };
                    let (request_still_active, fresh_signal) = read_entity(&e, cx, |r, _| {
                        (
                            r.is_current_request(request_id),
                            // Audit fix #24: `QuerySignal::new` directly instead
                            // of the redundant closure.
                            r.signal().cloned().unwrap_or_else(QuerySignal::new),
                        )
                    })
                    .unwrap_or((false, QuerySignal::new()));
                    if !request_still_active {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "DEBUG: run_query_retry_loop: request {} no longer active after retry delay, aborting retry",
                            request_id.label()
                        );
                        return;
                    }

                    // Audit fix #14: Only the signal variant re-reads a fresh
                    // signal after the retry-delay cancellation check. The
                    // no-signal variant leaves `signal` as `None` forever.
                    if let Some(ref mut sig) = signal {
                        *sig = fresh_signal;
                    }
                    // Loop to retry
                } else {
                    // No more retries -- complete with failure
                    let Some(e) = entity.upgrade() else { return };
                    let failure_now_ms = current_time_ms();
                    let _ = e.update(cx, |resource, cx| {
                        if let Some(guard) = resource.accept_current_request(request_id) {
                            resource.complete_failure(guard, error, failure_now_ms);
                            // Audit fix #4: Reset retry_count on terminal failure so the
                            // resource is clean for the next begin_request.
                            resource.reset_retry_count();
                            // Audit fix #7: Only notify when the result was actually accepted.
                            cx.notify();
                            // B2: precise dirty signal for the persistence layer.
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

/// Execute a fetch with retry logic for a query resource (no-signal variant).
///
/// Calls the fetcher. On failure, if the retry policy allows it, waits for the
/// configured delay and retries. Updates the entity state between attempts.
/// Resets the retry counter on success.
///
/// Takes an explicit `request_id` parameter obtained from
/// `begin_request_on_entity`. Callers should only invoke this when
/// `begin_request_on_entity` returns `Some(request_id)`.
///
/// Audit fix #14: Thin wrapper over [`run_query_retry_loop`] with
/// `signal = None`.
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
///
/// On retry, reads a fresh signal from the resource entity and passes it to
/// the fetcher. The signal is properly cancelled when a new request replaces
/// the current one (v2 fix).
///
/// Audit fix #14: Thin wrapper over [`run_query_retry_loop`] with
/// `signal = Some(initial_signal)`.
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
