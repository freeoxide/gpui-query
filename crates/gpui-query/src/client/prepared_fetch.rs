use gpui::{App, Entity};

use crate::core::QueryResource;

/// Run your fetcher with `self.signal`, then complete via
/// `complete_success` or `complete_failure`.
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
    pub entity: Entity<QueryResource<T, E>>,
    pub request_id: crate::core::RequestId,
    pub signal: crate::core::QuerySignal,
    /// The fetch's logical completion clock, captured at prepare time and
    /// reused by the complete_* methods.
    pub(crate) now_ms: u64,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> PreparedFetch<T, E> {
    /// A no-op if the request ID is no longer active (replaced by a newer
    /// request).
    pub fn complete_success(self, data: T, cx: &mut App) {
        self.entity.update(cx, |resource, _cx| {
            let accepted = resource.complete_current_success(self.request_id, data, self.now_ms);
            // Wake the persistence driver only when accepted; a stale no-op must not schedule a save.
            if accepted {
                #[cfg(feature = "persist")]
                _cx.default_global::<crate::client::CacheMutation>();
            }
        });
    }

    /// A no-op if the request ID is no longer active (replaced by a newer
    /// request).
    pub fn complete_failure(self, error: E, cx: &mut App) {
        self.entity.update(cx, |resource, _cx| {
            let accepted = resource.complete_current_failure(self.request_id, error, self.now_ms);
            if accepted {
                #[cfg(feature = "persist")]
                _cx.default_global::<crate::client::CacheMutation>();
            }
        });
    }
}
