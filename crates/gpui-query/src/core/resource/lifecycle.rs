//! Query resource lifecycle: begin, cancel, reset, optimistic updates.

use crate::core::{
    QueryBeginResult, QueryFetchMode, QuerySignal, QueryStatus, QueryTimestamp, RequestGuard,
    RequestId, RequestPolicy, RequestSequencer,
};

use super::QueryResource;

/// Source of the [`RequestId`] for the shared `begin_request_inner` helper.
///
/// Mirrors the `MaybeRequestId` pattern already used by
/// `InfiniteQueryResource` to dedup its four entry points. Keeping the two
/// public `begin_request` / `begin_request_with_id` entry points sharing one
/// implementation avoids the ~90% duplication flagged in N4, and threads the
/// stored per-resource sequencer (N3) through the `None` path so transient
/// callers no longer collide at `RequestId(1,1)`.
enum MaybeRequestId<'a> {
    FromSequencer(&'a mut RequestSequencer),
    Provided(Option<RequestId>),
}

impl<T, E> QueryResource<T, E> {
    /// Begin a new request on this resource.
    ///
    /// Respects the cache policy (may return `CacheHit`) and request policy
    /// (`IgnoreWhileLoading` or `LatestWins`). When replacing an existing
    /// request, the old signal is **cancelled** so the in-flight fetcher
    /// can observe it and abort early.
    pub fn begin_request(
        &mut self,
        sequencer: &mut RequestSequencer,
        now_ms: u64,
        fetch_mode: QueryFetchMode,
    ) -> QueryBeginResult {
        self.begin_request_inner(now_ms, fetch_mode, MaybeRequestId::FromSequencer(sequencer))
    }

    /// Like [`begin_request`](Self::begin_request) but accepts an optional
    /// pre-generated `RequestId` instead of using a `RequestSequencer`.
    ///
    /// When `maybe_request_id` is `Some`, uses that ID directly (useful when
    /// the bucket's co-located sequencer has already generated the ID).
    /// When `None`, falls back to the resource's own stored sequencer so the
    /// generated ids are monotonic and collision-free across calls (N3) rather
    /// than every call producing a colliding `RequestId(1,1)`.
    ///
    /// This is the preferred entry point for the hook layer (audit fixes
    /// #1/#5/#15/#18): it allows the bucket's persistent sequencer to provide
    /// globally unique, monotonically increasing RequestIds.
    pub fn begin_request_with_id(
        &mut self,
        maybe_request_id: Option<RequestId>,
        now_ms: u64,
        fetch_mode: QueryFetchMode,
    ) -> QueryBeginResult {
        self.begin_request_inner(
            now_ms,
            fetch_mode,
            MaybeRequestId::Provided(maybe_request_id),
        )
    }

    /// Shared implementation behind [`begin_request`](Self::begin_request) and
    /// [`begin_request_with_id`](Self::begin_request_with_id) (N4).
    ///
    /// `id_source` selects where the request id comes from: an external
    /// sequencer (for `begin_request`) or a pre-allocated id with a
    /// per-resource fallback (for `begin_request_with_id`). The fallback uses
    /// the resource's own stored sequencer (N3) instead of a fresh
    /// `RequestSequencer::new()`.
    fn begin_request_inner(
        &mut self,
        now_ms: u64,
        fetch_mode: QueryFetchMode,
        mut id_source: MaybeRequestId,
    ) -> QueryBeginResult {
        // Helper that resolves the next id from whichever source we were given,
        // evaluated lazily so early-return guards never consume a sequence
        // number (preserving the original counter-consumption behavior).
        macro_rules! next_id {
            () => {{
                match &mut id_source {
                    MaybeRequestId::FromSequencer(seq) => seq.next_request(),
                    MaybeRequestId::Provided(maybe_id) => maybe_id
                        .unwrap_or_else(|| self.transient_sequencer.next_request()),
                }
            }};
        }

        // 1. Fresh cache hit — no fetch needed at all.
        if fetch_mode == QueryFetchMode::Normal && self.should_short_circuit_cache(now_ms) {
            self.record_cache_hit();
            return QueryBeginResult::CacheHit;
        }

        // 2. Stale-while-revalidate: serve stale data immediately, trigger
        //    background refetch. This is checked before the IgnoreWhileLoading
        //    guard so we always revalidate stale data even if another request
        //    is in flight (the new request replaces it via LatestWins below).
        if fetch_mode == QueryFetchMode::Normal && self.should_serve_stale_and_revalidate(now_ms) {
            self.record_stale_cache_hit();

            // If IgnoreWhileLoading and a request is already active, skip the
            // background refetch — an in-flight request will refresh the data.
            if self.request_policy == RequestPolicy::IgnoreWhileLoading
                && let Some(active_request_id) = self.active_request_id
            {
                return QueryBeginResult::StaleCacheHit {
                    request_id: active_request_id,
                    status: self.status,
                    replaced_request_id: None,
                };
            }

            let replaced_request_id = self.active_request_id;
            if replaced_request_id.is_some() {
                self.cancelled_count = self.cancelled_count.saturating_add(1);
            }

            let request_id = next_id!();
            let status = self.begin_loading(request_id, now_ms);
            return QueryBeginResult::StaleCacheHit {
                request_id,
                status,
                replaced_request_id,
            };
        }

        // 3. IgnoreWhileLoading guard for normal (non-stale) requests.
        if self.request_policy == RequestPolicy::IgnoreWhileLoading
            && let Some(active_request_id) = self.active_request_id
        {
            return QueryBeginResult::IgnoredWhileLoading { active_request_id };
        }

        // 4. Normal fetch — start a new request.
        let replaced_request_id = self.active_request_id;
        if replaced_request_id.is_some() {
            self.cancelled_count = self.cancelled_count.saturating_add(1);
        }

        let request_id = next_id!();
        let status = self.begin_loading(request_id, now_ms);
        QueryBeginResult::Started {
            request_id,
            status,
            replaced_request_id,
        }
    }

    /// Internal: transition to a loading state.
    ///
    /// **v2 fix**: Cancels the old signal before creating a new one,
    /// so in-flight fetchers for replaced requests can abort early.
    ///
    /// Note: This method performs no guard against the current status. Under
    /// `LatestWins` policy, a second call while already `LoadingEmpty` is
    /// intentional — it cancels the old request and starts a new one. The old
    /// request's async task holds a stale `RequestId` and will be rejected by
    /// `accept_current_request()`.
    pub(crate) fn begin_loading(&mut self, request_id: RequestId, now_ms: u64) -> QueryStatus {
        let status = if self.has_data() {
            QueryStatus::LoadingWithData
        } else {
            QueryStatus::LoadingEmpty
        };
        self.status = status;
        self.active_request_id = Some(request_id);
        self.started_at = Some(QueryTimestamp::from(now_ms));
        self.error = None;

        // v2 fix: Cancel the OLD signal before replacing it.
        if let Some(old_signal) = self.signal.as_ref() {
            old_signal.cancel();
        }
        self.signal = Some(QuerySignal::new());

        status
    }

    /// Whether the given request id is the current active request.
    pub fn is_current_request(&self, request_id: RequestId) -> bool {
        self.active_request_id == Some(request_id)
    }

    /// Accept a request for completion.
    ///
    /// Returns a [`RequestGuard`] if the request is still active, or `None`
    /// if it was replaced or cancelled. The guard is a capability token for
    /// the two-phase protocol (validate → complete).
    pub fn accept_current_request(&mut self, request_id: RequestId) -> Option<RequestGuard> {
        if self.is_current_request(request_id) {
            self.active_request_id = None;
            Some(RequestGuard::new(request_id))
        } else {
            self.mark_ignored_result();
            None
        }
    }

    /// Cancel the active request.
    ///
    /// Returns `false` if there is no active request.
    /// The signal is cancelled so the in-flight fetcher can observe it.
    ///
    /// Data is preserved across cancellations. Current data (if any) is saved
    /// to `previous_data` before being cleared, allowing recovery via
    /// `rollback_to_previous()`. This matches TanStack Query behavior where
    /// cancelling a refetch does not destroy existing data.
    ///
    /// When the resource was in `LoadingEmpty` status (no prior data existed),
    /// both `data` and `previous_data` remain `None`. When the resource was in
    /// `LoadingWithData` status (a refetch with existing data), the prior data
    /// is saved to `previous_data` and `data` is set to `None`. Callers can use
    /// `rollback_to_previous()` to recover the data if needed.
    pub fn cancel(&mut self, error: E) -> bool {
        if self.active_request_id.is_none() {
            return false;
        }

        self.active_request_id = None;
        self.status = QueryStatus::Cancelled;
        self.error = Some(error);
        self.cancelled_count = self.cancelled_count.saturating_add(1);

        // Save current data to previous_data before clearing so
        // rollback_to_previous() can recover it.
        if self.data.is_some() {
            self.previous_data = self.data.take();
        }

        if let Some(signal) = self.signal.as_ref() {
            signal.cancel();
        }

        true
    }

    pub fn mark_ignored_result(&mut self) {
        self.ignored_results = self.ignored_results.saturating_add(1);
    }

    /// Whether the current data was served from stale cache (i.e., a
    /// stale-while-revalidate background refetch is in progress or failed).
    ///
    /// Returns `true` when the resource has data but the status indicates
    /// the most recent fetch attempt failed or was cancelled. Consumers can
    /// use this to distinguish "fresh success" from "stale data still being
    /// displayed after a background refetch failure".
    ///
    /// Note: This is a heuristic check. A `true` result means data exists but
    /// the last fetch did not succeed — the data may still be perfectly valid.
    pub fn is_data_stale(&self) -> bool {
        self.data.is_some()
            && matches!(
                self.status,
                QueryStatus::LoadingWithData | QueryStatus::Failure | QueryStatus::Cancelled
            )
    }

    /// Reset the resource back to idle, clearing state and diagnostic counters.
    ///
    /// **v2 fix**: Cancels the signal before clearing it.
    ///
    /// **Preserves**: `cache_policy`, `request_policy`, `retry_policy`, and `key`.
    /// These are considered configuration, not runtime state, and persist across
    /// resets. Use `QueryResource::new()` to create a fully fresh resource with
    /// default policies.
    ///
    /// Calling `reset()` on an already-Idle resource resets diagnostic counters
    /// (`cache_hits`, `cancelled_count`, `ignored_results`, `retry_count`) to zero.
    /// This is intentional — `reset()` always resets counters regardless of current
    /// state. If counter preservation is needed, read them before calling `reset()`.
    pub fn reset(&mut self) {
        // Cancel signal before dropping
        if let Some(signal) = self.signal.as_ref() {
            signal.cancel();
        }
        self.status = QueryStatus::Idle;
        self.data = None;
        self.error = None;
        self.active_request_id = None;
        self.started_at = None;
        self.last_updated_at = None;
        self.cache_hits = 0;
        self.cancelled_count = 0;
        self.ignored_results = 0;
        self.retry_count = 0;
        self.previous_data = None;
        self.signal = None;
    }

    /// Roll back to the previous data (optimistic update undo).
    ///
    /// Clears any stored error to maintain the invariant that `Success`
    /// implies `error is None` (mirroring `apply_success`).
    pub fn rollback_to_previous(&mut self) -> bool {
        if let Some(prev) = self.previous_data.take() {
            self.data = Some(prev);
            self.status = QueryStatus::Success;
            self.error = None;
            return true;
        }
        false
    }

    /// Apply an optimistic update. Current data is saved for rollback.
    pub fn set_data(&mut self, data: T) {
        self.previous_data = self.data.take();
        self.data = Some(data);
    }

    /// Clear data optimistically. Current data is saved for rollback.
    ///
    /// Transitions status to `Idle` to maintain the invariant that `Success`
    /// implies data is available (mirroring `apply_success_optional`'s `None`
    /// branch). Without this, a `Success` resource with `data = None` would
    /// panic on `data.unwrap()`.
    pub fn clear_data(&mut self) {
        self.previous_data = self.data.take();
        if self.status == QueryStatus::Success {
            self.status = QueryStatus::Idle;
        }
    }
}
