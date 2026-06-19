//! Lifecycle methods for [`InfiniteQueryResource`]: fetch, complete, reset,
//! invalidate, and two-phase protocol.

use std::sync::Arc;

use crate::core::{
    QuerySignal, QueryStatus, QueryTimestamp, RequestGuard, RequestId, RequestSequencer,
};

use super::FetchDirection;
use super::InfiniteQueryResource;

/// Direction of an infinite-query page fetch, used internally to share logic
/// between the four `begin_fetch_*` entry points (audit #12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageDirection {
    Next,
    Previous,
}

/// Source of the [`RequestId`] for [`InfiniteQueryResource::begin_fetch`] (audit #12).
///
/// Keeps the sequencer-based and pre-allocated-id entry points sharing one
/// implementation: the sequencer variant borrows the sequencer and only calls
/// `next_request()` after the early-return guards, preserving the original
/// counter-consumption behavior exactly.
enum MaybeRequestId<'a> {
    FromSequencer(&'a mut RequestSequencer),
    Provided(Option<RequestId>),
}

// ── Lifecycle ───────────────────────────────────────────────────────────

impl<T, E> InfiniteQueryResource<T, E> {
    /// Begin fetching the next page.
    ///
    /// **v2 fix**: Cancels the old signal before creating a new one.
    ///
    /// **Cross-direction replacement** (audit 2): When `RequestPolicy::LatestWins`
    /// is set and a `begin_fetch_previous` is currently active, calling this
    /// method will replace the previous-page request with this next-page request.
    /// The old signal is cancelled and the previous-page result will be silently
    /// discarded by `complete_page_success` (which returns `false` for stale IDs).
    /// This is intentional for `LatestWins` semantics — the most recent direction
    /// wins. Callers should check `is_fetching_next_page()` / `is_fetching_previous_page()`
    /// before completing if they need to detect direction changes.
    ///
    /// **Per-direction `IgnoreWhileLoading` semantics** (audit #87): the guard
    /// only applies within the same direction. Beginning a next-page fetch while
    /// `is_fetching_previous_page` is active bypasses the `IgnoreWhileLoading`
    /// guard and cancels/replaces the in-flight previous fetch — the reverse is
    /// also true for `begin_fetch_previous`. See [`begin_fetch_previous`] for the
    /// mirror case.
    pub fn begin_fetch_next(
        &mut self,
        sequencer: &mut RequestSequencer,
        now_ms: u128,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Next,
            MaybeRequestId::FromSequencer(sequencer),
            now_ms,
        )
    }

    /// Begin fetching the previous page.
    ///
    /// **v2 fix**: Cancels the old signal before creating a new one.
    ///
    /// **Cross-direction replacement** (audit 2): When `RequestPolicy::LatestWins`
    /// is set and a `begin_fetch_next` is currently active, calling this
    /// method will replace the next-page request with this previous-page request.
    /// The old signal is cancelled and the next-page result will be silently
    /// discarded by `complete_page_success` (which returns `false` for stale IDs).
    /// This is intentional for `LatestWins` semantics — the most recent direction
    /// wins. Callers should check `is_fetching_next_page()` / `is_fetching_previous_page()`
    /// before completing if they need to detect direction changes.
    ///
    /// **Per-direction `IgnoreWhileLoading` semantics** (audit #87): the guard
    /// only applies within the same direction. Beginning a previous-page fetch
    /// while `is_fetching_next_page` is active bypasses the `IgnoreWhileLoading`
    /// guard and cancels/replaces the in-flight next fetch — and vice versa for
    /// `begin_fetch_next`. This is intentional: a same-direction re-entrancy is
    /// suppressed under `IgnoreWhileLoading`, but an opposite-direction fetch is
    /// treated as a new request that supersedes the current one.
    pub fn begin_fetch_previous(
        &mut self,
        sequencer: &mut RequestSequencer,
        now_ms: u128,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Previous,
            MaybeRequestId::FromSequencer(sequencer),
            now_ms,
        )
    }

    /// Like [`begin_fetch_next`] but accepts an optional pre-generated `RequestId`
    /// instead of a `RequestSequencer`.
    ///
    /// When `maybe_request_id` is `Some`, uses that ID directly — this is the
    /// preferred call when the bucket's co-located sequencer has already
    /// pre-allocated an ID via `QueryClient::next_request_id_for_infinite_key`.
    /// When `None`, falls back to a transient `RequestSequencer::new()` for
    /// compatibility (e.g., when no `QueryClient` is available).
    ///
    /// Passing the pre-allocated ID through ensures the `RequestId` stored as
    /// the resource's `active_request_id` matches the one the bucket's sequencer
    /// already consumed, keeping the bucket's monotonic counter consistent with
    /// the resource's active request.
    pub fn begin_fetch_next_with_id(
        &mut self,
        maybe_request_id: Option<RequestId>,
        now_ms: u128,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Next,
            MaybeRequestId::Provided(maybe_request_id),
            now_ms,
        )
    }

    /// Like [`begin_fetch_previous`] but accepts an optional pre-generated
    /// `RequestId` instead of a `RequestSequencer`.
    ///
    /// When `maybe_request_id` is `Some`, uses that ID directly — this is the
    /// preferred call when the bucket's co-located sequencer has already
    /// pre-allocated an ID via `QueryClient::next_request_id_for_infinite_key`.
    /// When `None`, falls back to a transient `RequestSequencer::new()` for
    /// compatibility (e.g., when no `QueryClient` is available).
    ///
    /// Passing the pre-allocated ID through ensures the `RequestId` stored as
    /// the resource's `active_request_id` matches the one the bucket's sequencer
    /// already consumed, keeping the bucket's monotonic counter consistent with
    /// the resource's active request.
    pub fn begin_fetch_previous_with_id(
        &mut self,
        maybe_request_id: Option<RequestId>,
        now_ms: u128,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Previous,
            MaybeRequestId::Provided(maybe_request_id),
            now_ms,
        )
    }

    /// Shared implementation behind the four `begin_fetch_*` entry points
    /// (audit #12). Behavior-preserving: the public wrappers retain their
    /// original signatures.
    ///
    /// `id_source` is either a live sequencer (for the `_next`/`_previous`
    /// variants) or a pre-allocated id (for the `_with_id` variants).
    fn begin_fetch(
        &mut self,
        direction: PageDirection,
        id_source: MaybeRequestId,
        now_ms: u128,
    ) -> Option<RequestId> {
        let (has_page, is_fetching_same_direction) = match direction {
            PageDirection::Next => (self.has_next_page, self.is_fetching_next_page),
            PageDirection::Previous => {
                (self.has_previous_page, self.is_fetching_previous_page)
            }
        };

        if !has_page {
            return None;
        }

        if is_fetching_same_direction
            && self.request_policy == crate::core::RequestPolicy::IgnoreWhileLoading
        {
            return None;
        }

        if self.active_request_id.is_some() {
            self.cancelled_count += 1;
        }

        // v2 fix: Cancel old signal before replacing
        if let Some(old_signal) = self.signal.as_ref() {
            old_signal.cancel();
        }

        match direction {
            PageDirection::Next => {
                self.is_fetching_next_page = true;
                self.is_fetching_previous_page = false;
            }
            PageDirection::Previous => {
                self.is_fetching_previous_page = true;
                self.is_fetching_next_page = false;
            }
        }

        let request_id = match id_source {
            MaybeRequestId::FromSequencer(sequencer) => sequencer.next_request(),
            MaybeRequestId::Provided(maybe_id) => {
                maybe_id.unwrap_or_else(|| RequestSequencer::new().next_request())
            }
        };
        self.active_request_id = Some(request_id);
        self.status = if self.pages.is_empty() {
            QueryStatus::LoadingEmpty
        } else {
            QueryStatus::LoadingWithData
        };
        self.started_at = Some(QueryTimestamp::from(now_ms));
        self.error = None;
        self.signal = Some(QuerySignal::new());

        Some(request_id)
    }

    /// Accept the current request for two-phase completion.
    ///
    /// Returns a [`RequestGuard`] if the request is still active, or `None`
    /// if it was replaced or cancelled. The guard is a capability token for
    /// the two-phase protocol (validate then complete).
    ///
    /// This mirrors [`QueryResource::accept_current_request`] for consistency.
    /// Use this when you need to inspect or transform data between validation
    /// and completion, or when integrating with frameworks that prefer explicit
    /// acceptance.
    pub fn accept_current_request(&mut self, request_id: RequestId) -> Option<RequestGuard> {
        if self.is_current_request(request_id) {
            self.active_request_id = None;
            Some(RequestGuard::new(request_id))
        } else {
            None
        }
    }

    /// Complete a page fetch with success using a guard (two-phase protocol).
    ///
    /// The guard proves that `accept_current_request` already validated the
    /// request is current.
    ///
    /// **Audit 3**: Uses `VecDeque::push_back` for append and `VecDeque::push_front`
    /// for prepend — both O(1) amortized.
    pub fn complete_success_with_guard(
        &mut self,
        _guard: RequestGuard,
        page: T,
        has_more: bool,
        is_next: bool,
        now_ms: u128,
    ) {
        if is_next {
            self.pages.push_back(Arc::new(page));
            self.has_next_page = has_more;
            self.enforce_max_pages_remove_front();
        } else {
            self.pages.push_front(Arc::new(page));
            self.has_previous_page = has_more;
            self.enforce_max_pages_remove_back();
        }

        self.status = QueryStatus::Success;
        self.error = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
        self.is_fetching_next_page = false;
        self.is_fetching_previous_page = false;
        self.signal = None;
    }

    /// Complete a page fetch with failure using a guard (two-phase protocol).
    ///
    /// The guard proves that `accept_current_request` already validated the
    /// request is current.
    ///
    /// **Note**: This does NOT clear previously loaded pages. A `Failure` status
    /// means the last page fetch failed, but previously loaded pages remain
    /// accessible via [`pages`](Self::pages). Use
    /// [`is_page_data_valid`](Self::is_page_data_valid) to check whether page
    /// data can be relied upon.
    pub fn complete_failure_with_guard(&mut self, _guard: RequestGuard, error: E) {
        self.status = QueryStatus::Failure;
        self.error = Some(error);
        self.is_fetching_next_page = false;
        self.is_fetching_previous_page = false;
        self.signal = None;
    }

    /// Complete a page fetch with success.
    ///
    /// Convenience method that accepts and completes in one call.
    ///
    /// **Audit 3**: Uses `VecDeque::push_back` for append and `VecDeque::push_front`
    /// for prepend — both O(1) amortized.
    pub fn complete_page_success(
        &mut self,
        request_id: RequestId,
        page: T,
        has_more: bool,
        is_next: bool,
        now_ms: u128,
    ) -> bool {
        if self.active_request_id != Some(request_id) {
            self.ignored_results += 1;
            return false;
        }

        if is_next {
            self.pages.push_back(Arc::new(page));
            self.has_next_page = has_more;
            self.enforce_max_pages_remove_front();
        } else {
            self.pages.push_front(Arc::new(page));
            self.has_previous_page = has_more;
            self.enforce_max_pages_remove_back();
        }

        self.status = QueryStatus::Success;
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
        self.is_fetching_next_page = false;
        self.is_fetching_previous_page = false;
        self.signal = None;

        true
    }

    /// Complete a page fetch with failure.
    ///
    /// Convenience method that accepts and completes in one call.
    ///
    /// **Note**: This does NOT clear previously loaded pages. The `Failure` status
    /// applies to the most recent page fetch attempt only — previously loaded pages
    /// remain accessible via [`pages()`](Self::pages) and are still valid. Use
    /// [`is_page_data_valid()`](Self::is_page_data_valid) to check whether page
    /// data can be relied upon.
    pub fn complete_page_failure(&mut self, request_id: RequestId, error: E) -> bool {
        if self.active_request_id != Some(request_id) {
            self.ignored_results += 1;
            return false;
        }

        self.status = QueryStatus::Failure;
        self.error = Some(error);
        self.active_request_id = None;
        self.is_fetching_next_page = false;
        self.is_fetching_previous_page = false;
        self.signal = None;

        true
    }

    /// Whether the given request id is the current active request.
    pub fn is_current_request(&self, request_id: RequestId) -> bool {
        self.active_request_id == Some(request_id)
    }

    /// Reset to idle, clearing everything.
    ///
    /// **Note** (audit 2): `max_pages` is preserved across resets — if it was
    /// changed via `set_max_pages()`, that value persists.
    ///
    /// **Audit 3**: `has_next_page` and `has_previous_page` are reset according
    /// to the current [`FetchDirection`](self.direction):
    /// - `ForwardOnly`: `has_next_page = true`, `has_previous_page = false`
    /// - `Bidirectional`: both reset to `false`
    ///
    /// If the resource was previously exhausted, the caller should set the
    /// flags again after reset if the direction-based defaults are incorrect.
    pub fn reset(&mut self) {
        if let Some(signal) = self.signal.as_ref() {
            signal.cancel();
        }
        self.pages.clear();
        self.status = QueryStatus::Idle;
        self.error = None;
        self.active_request_id = None;
        self.started_at = None;
        self.last_updated_at = None;
        self.cache_hits = 0;
        self.cancelled_count = 0;
        self.ignored_results = 0;
        self.retry_count = 0;
        // max_pages is intentionally preserved across resets.
        // direction is intentionally preserved across resets.
        let (has_next, has_prev) = match self.direction {
            FetchDirection::ForwardOnly => (true, false),
            FetchDirection::Bidirectional => (false, false),
        };
        self.has_next_page = has_next;
        self.has_previous_page = has_prev;
        self.is_fetching_next_page = false;
        self.is_fetching_previous_page = false;
        self.signal = None;
    }

    /// Invalidate the cache (clear last-updated timestamp).
    pub fn invalidate(&mut self) {
        self.last_updated_at = None;
    }
}
