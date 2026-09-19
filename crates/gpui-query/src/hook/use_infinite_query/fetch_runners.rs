//! Internal async fetch runners for infinite query page fetches: retry-aware,
//! running with a captured [`RequestId`] and two-phase completion.

use std::sync::Arc;

use crate::core::{InfiniteQueryResource, RequestId};

use crate::hook::{current_time_ms, read_entity};

/// Direction of an infinite-query page fetch.
///
/// The next/previous runners differ only in which page they read as the
/// cursor and which `is_next` flag they pass to
/// [`InfiniteQueryResource::complete_success_with_guard`]; this enum
/// parameterizes that difference so the body lives in one place.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PageDirection {
    Next,
    Previous,
}

impl PageDirection {
    /// The `is_next` flag handed to `complete_success_with_guard`.
    fn is_next(self) -> bool {
        matches!(self, PageDirection::Next)
    }

    /// The page used as the fetcher cursor: the last page for `Next`, the
    /// first page for `Previous`. Read via the refcount-bumped `Arc<T>`
    /// accessor (no full page clone).
    fn cursor_page_arc<T: Clone + Send + Sync + 'static, E>(
        self,
        resource: &InfiniteQueryResource<T, E>,
    ) -> Option<Arc<T>> {
        match self {
            PageDirection::Next => resource.last_page_arc(),
            PageDirection::Previous => resource.first_page_arc(),
        }
    }
}

/// Execute a page fetch with a captured `RequestId` in the given direction.
///
/// The `request_id` is the one returned from `begin_fetch_*`, not re-read
/// after the fetcher completes, and completion is two-phase
/// (`accept_current_request` then complete) so a superseded request can never
/// write. After each retry delay the signal and the active request are checked
/// in one read pass; a cancelled or superseded fetch stops retrying.
async fn run_fetch_page_with_id<T, E, F, Fut>(
    entity: &gpui::WeakEntity<InfiniteQueryResource<T, E>>,
    fetcher: &F,
    request_id: RequestId,
    retry_policy: &crate::core::RetryPolicy,
    cx: &mut gpui::AsyncApp,
    direction: PageDirection,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    let mut attempt: u32 = 0;

    loop {
        // Re-read the cursor fresh each attempt so the fetcher sees
        // up-to-date data; the Arc access is a cheap refcount bump.
        let cursor_page_arc: Option<Arc<T>> = {
            let Some(e) = entity.upgrade() else { return };
            read_entity(&e, cx, |r, _| direction.cursor_page_arc(r)).flatten()
        };

        let result = fetcher(cursor_page_arc.as_ref().map(|a| a.as_ref())).await;

        let now_ms = current_time_ms();

        let Some(e) = entity.upgrade() else { return };

        match result {
            Ok((page, has_more)) => {
                let _ = e.update(cx, |resource, cx| {
                    if let Some(guard) = resource.accept_current_request(request_id) {
                        resource.complete_success_with_guard(
                            guard,
                            page,
                            has_more,
                            direction.is_next(),
                            now_ms,
                        );
                        cx.notify();
                        #[cfg(feature = "persist")]
                        cx.default_global::<crate::client::CacheMutation>();
                    }
                });
                return;
            }
            Err(error) => {
                if retry_policy.should_retry(attempt) {
                    let delay_ms = retry_policy.delay_for_attempt(attempt);
                    attempt += 1;

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    // No notify during retry wait: status stays Loading and
                    // the InfiniteQueryObserver dedupes.
                    let Some(e) = entity.upgrade() else { return };
                    let (cancelled, still_current) = read_entity(&e, cx, |r, _| {
                        (
                            r.signal().map(|s| s.is_cancelled()).unwrap_or(false),
                            r.is_current_request(request_id),
                        )
                    })
                    .unwrap_or((true, false));
                    if cancelled || !still_current {
                        return;
                    }
                } else {
                    let _ = e.update(cx, |resource, cx| {
                        if let Some(guard) = resource.accept_current_request(request_id) {
                            resource.complete_failure_with_guard(guard, error);
                            cx.notify();
                            #[cfg(feature = "persist")]
                            cx.default_global::<crate::client::CacheMutation>();
                        }
                    });
                    return;
                }
            }
        }
    }
}

/// Execute a fetch-next-page operation with a captured `RequestId`. Thin
/// direction-specific wrapper around [`run_fetch_page_with_id`].
pub(super) async fn run_fetch_next_page_with_id<T, E, F, Fut>(
    entity: &gpui::WeakEntity<InfiniteQueryResource<T, E>>,
    fetcher: &F,
    request_id: RequestId,
    retry_policy: &crate::core::RetryPolicy,
    cx: &mut gpui::AsyncApp,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    run_fetch_page_with_id(
        entity,
        fetcher,
        request_id,
        retry_policy,
        cx,
        PageDirection::Next,
    )
    .await;
}

/// Execute a fetch-previous-page operation with a captured `RequestId`. Thin
/// direction-specific wrapper around [`run_fetch_page_with_id`].
pub(super) async fn run_fetch_previous_page_with_id<T, E, F, Fut>(
    entity: &gpui::WeakEntity<InfiniteQueryResource<T, E>>,
    fetcher: &F,
    request_id: RequestId,
    retry_policy: &crate::core::RetryPolicy,
    cx: &mut gpui::AsyncApp,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(Option<&T>) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(T, bool), E>> + Send + 'static,
{
    run_fetch_page_with_id(
        entity,
        fetcher,
        request_id,
        retry_policy,
        cx,
        PageDirection::Previous,
    )
    .await;
}
