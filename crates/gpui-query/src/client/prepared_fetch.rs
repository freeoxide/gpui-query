//! [`PreparedFetch`]: the handle returned by the imperative fetch and
//! prefetch operations.

use gpui::{App, Entity};

use crate::core::QueryResource;

/// A prepared fetch returned by
/// [`QueryClient::prepare_fetch_query`](crate::client::QueryClient::prepare_fetch_query)
/// or
/// [`QueryClient::prepare_prefetch_query`](crate::client::QueryClient::prepare_prefetch_query).
///
/// Holds the entity, request ID, and cooperative cancellation signal needed
/// to perform the async fetch: call your fetcher with `self.signal`, then
/// complete via `complete_success` or `complete_failure`.
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
    /// Completion time captured at prepare time; the fetch's logical
    /// completion clock, reused by the complete_* methods.
    pub(crate) now_ms: u64,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> PreparedFetch<T, E> {
    /// Complete the fetch with success. A no-op if the request ID is no
    /// longer active (replaced by a newer request).
    pub fn complete_success(self, data: T, cx: &mut App) {
        self.entity.update(cx, |resource, cx| {
            let accepted = resource.complete_current_success(self.request_id, data, self.now_ms);
            // Wake the persistence driver, but only when the completion was
            // actually accepted, so a stale no-op does not schedule a save.
            if accepted {
                #[cfg(feature = "persist")]
                cx.default_global::<crate::client::CacheMutation>();
            }
        });
    }

    /// Complete the fetch with failure. A no-op if the request ID is no
    /// longer active (replaced by a newer request).
    pub fn complete_failure(self, error: E, cx: &mut App) {
        self.entity.update(cx, |resource, cx| {
            let accepted = resource.complete_current_failure(self.request_id, error, self.now_ms);
            if accepted {
                #[cfg(feature = "persist")]
                cx.default_global::<crate::client::CacheMutation>();
            }
        });
    }
}
