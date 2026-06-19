//! Page management methods for [`InfiniteQueryResource`].

use std::sync::Arc;

use super::{FetchDirection, InfiniteQueryResource};

// ── Page management ─────────────────────────────────────────────────────

impl<T, E> InfiniteQueryResource<T, E> {
    /// Set whether more pages are available after the last loaded page.
    pub fn set_has_next_page(&mut self, has_next: bool) {
        self.has_next_page = has_next;
    }

    /// Set whether more pages are available before the first loaded page.
    pub fn set_has_previous_page(&mut self, has_prev: bool) {
        self.has_previous_page = has_prev;
    }

    /// Set the fetch direction mode.
    ///
    /// This does not change the current `has_next_page` / `has_previous_page`
    /// flags — it only affects what `reset()` restores them to.
    pub fn set_direction(&mut self, direction: FetchDirection) {
        self.direction = direction;
    }

    /// Set the maximum number of pages to retain.
    ///
    /// A value of `Some(0)` is treated as unbounded (`None`) to prevent
    /// accidentally draining all pages. Callers that want no page retention
    /// should use `reset()` instead.
    ///
    /// Returns evicted pages (if any) as `Arc<T>` handles so the caller can log
    /// or process them without cloning the page data (audit #5).
    pub fn set_max_pages(&mut self, max: Option<usize>) -> Vec<Arc<T>> {
        // Treat 0 as unbounded to prevent draining all pages.
        self.max_pages = match max {
            Some(0) => None,
            other => other,
        };
        self.enforce_max_pages_remove_front()
    }

    /// Append a page to the end.
    ///
    /// **Audit 3**: Uses `VecDeque::push_back` — O(1) amortized.
    ///
    /// Returns evicted pages (if any) as `Arc<T>` handles (audit #5).
    pub fn append_page(&mut self, page: T) -> Vec<Arc<T>> {
        self.pages.push_back(Arc::new(page));
        self.enforce_max_pages_remove_front()
    }

    /// Prepend a page to the beginning.
    ///
    /// **Audit 3**: Uses `VecDeque::push_front` — O(1) amortized instead of
    /// the previous `Vec::insert(0, page)` which was O(n).
    ///
    /// Returns evicted pages (if any) as `Arc<T>` handles (audit #5).
    pub fn prepend_page(&mut self, page: T) -> Vec<Arc<T>> {
        self.pages.push_front(Arc::new(page));
        self.enforce_max_pages_remove_back()
    }

    /// **v2 fix**: Use `Vec::drain` instead of O(n²) `remove(0)`.
    ///
    /// **Audit 2 fix**: `max_pages` of 0 is treated as unbounded. At least 1
    /// page is always retained. Returns evicted pages for caller inspection.
    ///
    /// **Audit 3**: Uses `VecDeque::drain` — O(k) where k is the number of
    /// evicted pages.
    pub(super) fn enforce_max_pages_remove_front(&mut self) -> Vec<Arc<T>> {
        if let Some(max) = self.max_pages
            && max > 0 && self.pages.len() > max {
                return self.pages.drain(..self.pages.len() - max).collect();
            }
        Vec::new()
    }

    /// Evict pages from the back until within `max_pages`.
    ///
    /// **Audit 2 fix**: `max_pages` of 0 is treated as unbounded. At least 1
    /// page is always retained. Returns evicted pages for caller inspection.
    ///
    /// **Audit 3**: Uses `VecDeque::pop_back` — O(1) per eviction.
    ///
    /// **Audit 36**: Uses `VecDeque::drain` (matching the front variant) instead
    /// of a `while`/`pop_back` loop — O(k) where k is the number of evicted
    /// pages. The drained range is reversed so the returned vector preserves the
    /// original back-to-front eviction order (most-recently-prepended page first).
    pub(super) fn enforce_max_pages_remove_back(&mut self) -> Vec<Arc<T>> {
        if let Some(max) = self.max_pages
            && max > 0 && self.pages.len() > max {
                // Evict the oldest `len - max` pages from the back. The pages
                // that survive are the first `max` (indices `0..max`), so drain
                // from `max..`. The previous `start = len - max` formula drained
                // `max` pages from the middle/back and left too few behind.
                // Drain in reverse so the returned vector preserves the original
                // back-to-front eviction order (most-recently-prepended page
                // first) without a separate reverse() pass (N26).
                return self.pages.drain(max..).rev().collect();
            }
        Vec::new()
    }
}
