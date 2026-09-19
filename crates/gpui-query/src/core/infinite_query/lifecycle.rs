//! Lifecycle methods for [`InfiniteQueryResource`]: fetch, complete, reset,
//! invalidate, and two-phase protocol.

use std::sync::Arc;

use crate::core::{
    QuerySignal, QueryStatus, QueryTimestamp, RequestGuard, RequestId, RequestSequencer,
};

use super::FetchDirection;
use super::InfiniteQueryResource;

/// Direction of an infinite-query page fetch, used internally to share logic
/// between the four `begin_fetch_*` entry points.
///
/// `pub(super)` because it backs the serde-serialized `fetching_direction`
/// field on [`InfiniteQueryResource`]; a single `Option<PageDirection>` makes
/// the "only one direction in flight" invariant unrepresentable to violate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum PageDirection {
    Next,
    Previous,
}

/// Source of the [`RequestId`] for [`InfiniteQueryResource::begin_fetch`]:
/// a caller-supplied sequencer, or an optional pre-generated id with a
/// per-resource fallback. The sequencer variant only calls `next_request()`
/// after the early-return guards, so guards never consume a sequence number.
enum MaybeRequestId<'a> {
    FromSequencer(&'a mut RequestSequencer),
    Provided(Option<RequestId>),
}

impl<T, E> InfiniteQueryResource<T, E> {
    /// Begin fetching the next page.
    ///
    /// Cancels any in-flight request's signal before starting the new one.
    ///
    /// Under `RequestPolicy::LatestWins`, this replaces an active
    /// `begin_fetch_previous` request: the old signal is cancelled and the
    /// previous-page result is discarded by `complete_page_success` (it
    /// returns `false` for stale IDs). Callers can check
    /// `is_fetching_next_page()` / `is_fetching_previous_page()` before
    /// completing if they need to detect direction changes.
    ///
    /// The `IgnoreWhileLoading` guard only applies within the same direction:
    /// a next-page fetch while a previous-page fetch is active bypasses the
    /// guard and replaces it (and vice versa).
    pub fn begin_fetch_next(
        &mut self,
        sequencer: &mut RequestSequencer,
        now_ms: u64,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Next,
            MaybeRequestId::FromSequencer(sequencer),
            now_ms,
        )
    }

    /// Begin fetching the previous page.
    ///
    /// Cancels any in-flight request's signal before starting the new one.
    ///
    /// Under `RequestPolicy::LatestWins`, this replaces an active
    /// `begin_fetch_next` request: the old signal is cancelled and the
    /// next-page result is discarded by `complete_page_success` (it returns
    /// `false` for stale IDs). Callers can check
    /// `is_fetching_next_page()` / `is_fetching_previous_page()` before
    /// completing if they need to detect direction changes.
    ///
    /// The `IgnoreWhileLoading` guard only applies within the same direction:
    /// a previous-page fetch while a next-page fetch is active bypasses the
    /// guard and replaces it (and vice versa).
    pub fn begin_fetch_previous(
        &mut self,
        sequencer: &mut RequestSequencer,
        now_ms: u64,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Previous,
            MaybeRequestId::FromSequencer(sequencer),
            now_ms,
        )
    }

    /// Like [`begin_fetch_next`](Self::begin_fetch_next) but accepts an optional
    /// pre-generated `RequestId` instead of a `RequestSequencer`.
    ///
    /// When `maybe_request_id` is `Some`, uses that ID directly — the
    /// preferred call when the bucket's co-located sequencer has already
    /// pre-allocated an ID via `QueryClient::next_request_id_for_infinite_key`,
    /// so the resource's `active_request_id` matches the id the bucket already
    /// consumed. When `None`, falls back to the resource's own stored
    /// sequencer.
    pub fn begin_fetch_next_with_id(
        &mut self,
        maybe_request_id: Option<RequestId>,
        now_ms: u64,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Next,
            MaybeRequestId::Provided(maybe_request_id),
            now_ms,
        )
    }

    /// Like [`begin_fetch_previous`](Self::begin_fetch_previous) but accepts
    /// an optional pre-generated `RequestId` instead of a `RequestSequencer`.
    ///
    /// When `maybe_request_id` is `Some`, uses that ID directly — the
    /// preferred call when the bucket's co-located sequencer has already
    /// pre-allocated an ID via `QueryClient::next_request_id_for_infinite_key`,
    /// so the resource's `active_request_id` matches the id the bucket already
    /// consumed. When `None`, falls back to the resource's own stored
    /// sequencer.
    pub fn begin_fetch_previous_with_id(
        &mut self,
        maybe_request_id: Option<RequestId>,
        now_ms: u64,
    ) -> Option<RequestId> {
        self.begin_fetch(
            PageDirection::Previous,
            MaybeRequestId::Provided(maybe_request_id),
            now_ms,
        )
    }

    /// Shared implementation behind the four `begin_fetch_*` entry points.
    fn begin_fetch(
        &mut self,
        direction: PageDirection,
        id_source: MaybeRequestId,
        now_ms: u64,
    ) -> Option<RequestId> {
        let (has_page, is_fetching_same_direction) = match direction {
            PageDirection::Next => (self.has_next_page, self.is_fetching_next_page()),
            PageDirection::Previous => (self.has_previous_page, self.is_fetching_previous_page()),
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
            self.cancelled_count = self.cancelled_count.saturating_add(1);
        }

        // Cancel old signal before replacing.
        if let Some(old_signal) = self.signal.as_ref() {
            old_signal.cancel();
        }

        self.fetching_direction = Some(direction);

        let request_id = match id_source {
            MaybeRequestId::FromSequencer(sequencer) => sequencer.next_request(),
            MaybeRequestId::Provided(maybe_id) => {
                maybe_id.unwrap_or_else(|| self.transient_sequencer.next_request())
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
    /// if it was replaced or cancelled. A stale/replaced request's result is
    /// ignored, bumping `ignored_results`.
    pub fn accept_current_request(&mut self, request_id: RequestId) -> Option<RequestGuard> {
        if self.is_current_request(request_id) {
            self.active_request_id = None;
            Some(RequestGuard::new(request_id))
        } else {
            self.mark_ignored_result();
            None
        }
    }

    /// Complete a page fetch with success using a guard (two-phase protocol).
    ///
    /// Appends (`is_next`) or prepends the page in O(1) amortized. Pages
    /// evicted by the `max_pages` bound are dropped here (refcounts release,
    /// nothing leaks); the `append_page`/`prepend_page` methods are the
    /// variants that return evicted pages.
    pub fn complete_success_with_guard(
        &mut self,
        _guard: RequestGuard,
        page: T,
        has_more: bool,
        is_next: bool,
        now_ms: u64,
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
        self.fetching_direction = None;
        self.signal = None;
    }

    /// Complete a page fetch with failure using a guard (two-phase protocol).
    ///
    /// Does NOT clear previously loaded pages: `Failure` means the last page
    /// fetch failed, but loaded pages remain accessible via
    /// [`pages`](Self::pages). Use [`is_page_data_valid`](Self::is_page_data_valid)
    /// to check whether the page data can be relied upon.
    pub fn complete_failure_with_guard(&mut self, _guard: RequestGuard, error: E) {
        self.status = QueryStatus::Failure;
        self.error = Some(error);
        self.fetching_direction = None;
        self.signal = None;
    }

    /// Complete a page fetch with success.
    ///
    /// Convenience method that accepts and completes in one call. Appends
    /// (`is_next`) or prepends the page in O(1) amortized; pages evicted by
    /// the `max_pages` bound are dropped (see
    /// [`complete_success_with_guard`](Self::complete_success_with_guard)).
    pub fn complete_page_success(
        &mut self,
        request_id: RequestId,
        page: T,
        has_more: bool,
        is_next: bool,
        now_ms: u64,
    ) -> bool {
        if self.active_request_id != Some(request_id) {
            self.ignored_results = self.ignored_results.saturating_add(1);
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
        self.fetching_direction = None;
        self.signal = None;

        true
    }

    /// Complete a page fetch with failure.
    ///
    /// Convenience method that accepts and completes in one call. Does NOT
    /// clear previously loaded pages: `Failure` applies to the most recent
    /// page fetch attempt only. Use
    /// [`is_page_data_valid()`](Self::is_page_data_valid) to check whether
    /// the page data can be relied upon.
    pub fn complete_page_failure(&mut self, request_id: RequestId, error: E) -> bool {
        if self.active_request_id != Some(request_id) {
            self.ignored_results = self.ignored_results.saturating_add(1);
            return false;
        }

        self.status = QueryStatus::Failure;
        self.error = Some(error);
        self.active_request_id = None;
        self.fetching_direction = None;
        self.signal = None;

        true
    }

    /// Whether the given request id is the current active request.
    pub fn is_current_request(&self, request_id: RequestId) -> bool {
        self.active_request_id == Some(request_id)
    }

    /// Reset to idle, clearing everything.
    ///
    /// `max_pages` and `direction` are preserved across resets.
    /// `has_next_page` / `has_previous_page` are reset to the defaults of the
    /// current [`FetchDirection`] (`ForwardOnly` → `true`/`false`,
    /// `Bidirectional` → both `false`). If the resource was previously
    /// exhausted, set the flags again after reset if the direction-based
    /// defaults are wrong.
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
        let (has_next, has_prev) = match self.direction {
            FetchDirection::ForwardOnly => (true, false),
            FetchDirection::Bidirectional => (false, false),
        };
        self.has_next_page = has_next;
        self.has_previous_page = has_prev;
        self.fetching_direction = None;
        self.signal = None;
    }

    /// Invalidate the cache (clear last-updated timestamp).
    pub fn invalidate(&mut self) {
        self.last_updated_at = None;
    }
}
