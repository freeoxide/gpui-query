//! Accessor (getter / setter) methods for [`InfiniteQueryResource`].

use std::collections::VecDeque;
use std::sync::Arc;

use super::{FetchDirection, InfiniteQueryResource};
use crate::core::{
    CachePolicy, QueryKey, QuerySignal, QueryStatus, QueryTimestamp, RequestId, RequestPolicy,
    RetryPolicy,
};

// ── Accessors ────────────────────────────────────────────────────────────

impl<T, E> InfiniteQueryResource<T, E> {
    /// All loaded pages, in order from first to last.
    ///
    /// Pages are stored internally as `Arc<T>` (audit #5) so that fetchers can
    /// receive a cheap `Arc::clone` via [`first_page_arc`](Self::first_page_arc) /
    /// [`last_page_arc`](Self::last_page_arc) instead of copying the page data.
    /// Most call sites only need a `&T` view — use [`first_page`](Self::first_page)
    /// / [`last_page`](Self::last_page), or iterate with `.iter().map(|a| a.as_ref())`.
    ///
    /// **Note**: When `status()` is `Failure`, previously loaded pages are still
    /// present and valid — the failure applies only to the most recent page fetch.
    /// Use [`is_page_data_valid`](Self::is_page_data_valid) to check whether the
    /// current page data can be relied upon.
    pub fn pages(&self) -> &VecDeque<Arc<T>> {
        &self.pages
    }

    /// Number of loaded pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The first loaded page, if any (borrowed view).
    pub fn first_page(&self) -> Option<&T> {
        self.pages.front().map(|a| a.as_ref())
    }

    /// The last loaded page, if any (borrowed view).
    pub fn last_page(&self) -> Option<&T> {
        self.pages.back().map(|a| a.as_ref())
    }

    /// Cheap `Arc::clone` of the first page, if any (audit #5).
    ///
    /// Hand this to a `fetch_previous_page` fetcher instead of cloning the full
    /// page data — only the refcount is bumped.
    pub fn first_page_arc(&self) -> Option<Arc<T>> {
        self.pages.front().cloned()
    }

    /// Cheap `Arc::clone` of the last page, if any (audit #5).
    ///
    /// Hand this to a `fetch_next_page` fetcher instead of cloning the full
    /// page data — only the refcount is bumped.
    pub fn last_page_arc(&self) -> Option<Arc<T>> {
        self.pages.back().cloned()
    }

    /// Whether there are more pages after the last loaded page.
    pub fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    /// Whether there are more pages before the first loaded page.
    pub fn has_previous_page(&self) -> bool {
        self.has_previous_page
    }

    /// Whether a `fetch_next_page` request is in flight.
    pub fn is_fetching_next_page(&self) -> bool {
        matches!(self.fetching_direction, Some(super::lifecycle::PageDirection::Next))
    }

    /// Whether a `fetch_previous_page` request is in flight.
    pub fn is_fetching_previous_page(&self) -> bool {
        matches!(self.fetching_direction, Some(super::lifecycle::PageDirection::Previous))
    }

    /// Maximum number of pages to retain.
    pub fn max_pages(&self) -> Option<usize> {
        self.max_pages
    }

    /// The fetch direction mode for this query.
    ///
    /// **Audit 3**: Controls the default assumptions for `has_next_page` and
    /// `has_previous_page` after construction and after `reset()`.
    pub fn direction(&self) -> FetchDirection {
        self.direction
    }

    /// Current status.
    pub fn status(&self) -> QueryStatus {
        self.status
    }

    /// Most recent error.
    pub fn error(&self) -> Option<&E> {
        self.error.as_ref()
    }

    /// Whether loading.
    pub fn is_loading(&self) -> bool {
        self.status.is_loading()
    }

    /// Cache key.
    pub fn key(&self) -> &QueryKey {
        &self.key
    }

    /// Active request id.
    pub fn active_request_id(&self) -> Option<RequestId> {
        self.active_request_id
    }

    /// Cache policy.
    pub fn cache_policy(&self) -> CachePolicy {
        self.cache_policy
    }

    /// Request policy.
    pub fn request_policy(&self) -> RequestPolicy {
        self.request_policy
    }

    /// Set the cache policy.
    ///
    /// This allows policy updates on existing resources when the same key is
    /// reused with different policies (e.g., a different TTL).
    pub fn set_cache_policy(&mut self, policy: CachePolicy) {
        self.cache_policy = policy;
    }

    /// Set the request policy.
    ///
    /// This allows policy updates on existing resources when the same key is
    /// reused with different request behavior.
    pub fn set_request_policy(&mut self, policy: RequestPolicy) {
        self.request_policy = policy;
    }

    /// The retry policy for page fetches.
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    /// Set the retry policy.
    ///
    /// Stored by `use_infinite_query` from [`InfiniteQueryOptions::retry_policy`]
    /// so that fetch helpers can read it from the entity.
    pub fn set_retry_policy(&mut self, policy: RetryPolicy) {
        self.retry_policy = policy;
    }

    /// When the current request started (ms).
    pub fn started_at_ms(&self) -> Option<u64> {
        self.started_at.map(QueryTimestamp::as_millis)
    }

    /// When data was last updated (ms).
    pub fn last_updated_at_ms(&self) -> Option<u64> {
        self.last_updated_at.map(QueryTimestamp::as_millis)
    }

    /// Cache age in milliseconds (L6).
    ///
    /// Mirrors [`QueryResource::cache_age_ms`]: returns `None` when there is
    /// no recorded `last_updated_at`, and also `None` on clock skew
    /// (`now_ms` before the recorded timestamp) via `checked_sub`. Used by
    /// `InfiniteQueryBucket::collect_diagnostics` so the infinite diagnostic
    /// matches the regular query's `cache_age_ms` behavior (the previous
    /// inline `saturating_sub` returned `Some(0)` on skew).
    ///
    /// [`QueryResource::cache_age_ms`]: crate::core::QueryResource::cache_age_ms
    pub fn cache_age_ms(&self, now_ms: u64) -> Option<u64> {
        QueryTimestamp::from(now_ms).elapsed_since(self.last_updated_at?)
    }

    /// Total cache hits.
    pub fn cache_hits(&self) -> u64 {
        self.cache_hits
    }

    /// Total cancelled requests.
    pub fn cancelled_count(&self) -> u64 {
        self.cancelled_count
    }

    /// Total ignored results (completed requests whose ID no longer matched).
    ///
    /// Incremented when `complete_page_success` or `complete_page_failure`
    /// receives a stale request ID, i.e. the result was produced by a fetch
    /// that was subsequently replaced by a newer one.
    pub fn ignored_results(&self) -> u64 {
        self.ignored_results
    }

    /// Number of retry attempts for the current page fetch.
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Increment the retry counter.
    pub fn increment_retry(&mut self) {
        self.retry_count = self.retry_count.saturating_add(1);
    }

    /// Increment the ignored-results counter.
    ///
    /// Mirrors `QueryResource::mark_ignored_result` so the client layer's
    /// bulk-cancel path can bump `ignored_results` for infinite queries the
    /// same way it does for regular queries (M5 core half).
    pub fn mark_ignored_result(&mut self) {
        self.ignored_results = self.ignored_results.saturating_add(1);
    }

    /// Reset the retry counter to zero.
    pub fn reset_retry_count(&mut self) {
        self.retry_count = 0;
    }

    /// Whether any pages have been loaded.
    pub fn has_data(&self) -> bool {
        !self.pages.is_empty()
    }

    /// Whether the currently loaded page data is valid.
    ///
    /// Returns `true` when:
    /// - Status is `Success` (pages are up to date), or
    /// - Status is `LoadingWithData` or `LoadingEmpty` (pages from a previous
    ///   successful fetch are still valid while a new page is being fetched).
    ///
    /// Returns `false` when:
    /// - Status is `Idle` (no pages have been fetched yet), or
    /// - Status is `Cancelled` (data was explicitly cleared).
    ///
    /// **Important**: When status is `Failure`, this returns `true` if pages were
    /// previously loaded. A `Failure` status means the *last page fetch* failed,
    /// but all previously loaded pages remain valid. This is distinct from
    /// `QueryResource` where `Failure` invalidates the single data slot.
    pub fn is_page_data_valid(&self) -> bool {
        match self.status {
            QueryStatus::Success | QueryStatus::LoadingWithData => true,
            QueryStatus::Failure => !self.pages.is_empty(),
            QueryStatus::LoadingEmpty | QueryStatus::Idle | QueryStatus::Cancelled => false,
        }
    }

    /// Cancellation signal.
    pub fn signal(&self) -> Option<&QuerySignal> {
        self.signal.as_ref()
    }
}
