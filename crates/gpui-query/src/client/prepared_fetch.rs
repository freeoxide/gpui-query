//! Prepared fetch type for imperative and prefetch query operations.
//!
//! [`PreparedFetch`] is returned by `QueryClient::prepare_fetch_query` and
//! `QueryClient::prepare_prefetch_query`. It holds the entity, request ID,
//! and cooperative cancellation signal needed to complete an async fetch.

use gpui::{App, Entity};

use crate::core::QueryResource;

/// A prepared fetch returned by [`QueryClient::prepare_fetch_query`] or
/// [`QueryClient::prepare_prefetch_query`].
///
/// Contains the entity, request ID, and cooperative cancellation signal
/// needed to perform the async fetch and complete the resource.
///
/// The caller should:
/// 1. Call their fetcher with `self.signal`
/// 2. Use `complete_success` or `complete_failure` with the result
///
/// # Example
///
/// ```no_run
/// use gpui_query::client::QueryClient;
/// use gpui_query::core::QueryKey;
/// # #[derive(Clone)]
/// # struct Data;
/// # #[derive(Clone, Debug)]
/// # struct Error;
/// # fn _doc(client: &mut QueryClient, cx: &mut gpui::App) {
/// # let key = QueryKey::from("data");
///
/// let prepared = client.prepare_fetch_query::<Data, Error>(key, cx).unwrap();
/// let signal = prepared.signal.clone();
/// // Use cx.spawn() to run your async fetcher with the signal, then call
/// // prepared.complete_success(data, cx) or prepared.complete_failure(e, cx).
/// # }
/// ```
#[must_use = "the prepared fetch holds the request ID and cancellation signal; dropping it without calling complete_success/complete_failure abandons the in-flight request"]
pub struct PreparedFetch<T, E> {
    /// The query resource entity.
    pub entity: Entity<QueryResource<T, E>>,
    /// The request ID for the started request.
    pub request_id: crate::core::RequestId,
    /// The cooperative cancellation signal for the in-flight request.
    pub signal: crate::core::QuerySignal,
    /// **M3**: the wall-clock ms captured at prepare time. Reused by
    /// `complete_success` / `complete_failure` so they don't re-syscall
    /// `current_time_ms()` (the fetch's logical completion time is the prepare
    /// time, matching the request's `started_at`).
    pub(crate) now_ms: u64,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> PreparedFetch<T, E> {
    /// Complete the fetch with success.
    ///
    /// Calls `complete_current_success` on the resource entity. If the request
    /// ID is no longer active (replaced by a newer request), this is a no-op.
    ///
    /// **M3**: reuses the `now_ms` captured at prepare time instead of
    /// re-syscalling `current_time_ms()`.
    pub fn complete_success(self, data: T, cx: &mut App) {
        self.entity.update(cx, |resource, _| {
            resource.complete_current_success(self.request_id, data, self.now_ms);
        });
    }

    /// Complete the fetch with failure.
    ///
    /// Calls `complete_current_failure` on the resource entity. If the request
    /// ID is no longer active (replaced by a newer request), this is a no-op.
    ///
    /// **M3**: reuses the `now_ms` captured at prepare time instead of
    /// re-syscalling `current_time_ms()`.
    pub fn complete_failure(self, error: E, cx: &mut App) {
        self.entity.update(cx, |resource, _| {
            resource.complete_current_failure(self.request_id, error, self.now_ms);
        });
    }
}
