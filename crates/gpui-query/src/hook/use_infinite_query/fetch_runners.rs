//! Internal async fetch runners for infinite query page fetches.
//!
//! These are the retry-aware async functions that execute the actual fetch
//! operations with captured `RequestId`s and two-phase completion protocol.

use std::sync::Arc;

use crate::core::{InfiniteQueryResource, RequestId};

use crate::hook::{current_time_ms, read_entity};

// ── Internal fetch runners ───────────────────────────────────────────────

/// Direction of an infinite-query page fetch.
///
/// The next/previous fetch runners are ~90% identical, differing only in
/// which page they read as the cursor and which `is_next` flag they pass to
/// [`InfiniteQueryResource::complete_success_with_guard`]. This enum
/// parameterizes that single difference so the shared body lives in one place.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PageDirection {
    Next,
    Previous,
}

impl PageDirection {
    /// The `is_next` flag handed to `complete_success_with_guard`.
    fn is_next(self) -> bool {
        match self {
            PageDirection::Next => true,
            PageDirection::Previous => false,
        }
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

/// Execute a fetch-page operation with a captured `RequestId` in the given
/// [`PageDirection`].
///
/// #fix #5/#6: The `request_id` is the one returned from `begin_fetch_*`,
/// not re-read after the fetcher completes. This prevents stale-ID acceptance
/// when concurrent fetches are in flight.
///
/// #fix #12: Uses two-phase completion (`accept_current_request` then
/// `complete_success_with_guard`/`complete_failure_with_guard`) to close
/// the race window between reading active_request_id and completing.
///
/// #fix #13: Applies retry policy on fetch failure.
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
        // Audit fix #5/#73: Read the cursor page inside the loop via the cheap
        // refcount-bumped `Arc<T>` accessor (no full page clone), and re-read
        // it fresh each retry so the fetcher sees up-to-date data. We capture
        // the `Arc<T>` and hand the fetcher an `Option<&T>` via `as_ref` — the
        // fetcher signature is unchanged. For `Next` this is the last page;
        // for `Previous` it is the first page.
        let cursor_page_arc: Option<Arc<T>> = {
            let Some(e) = entity.upgrade() else { return };
            read_entity(&e, cx, |r, _| direction.cursor_page_arc(r)).flatten()
        };

        let result = fetcher(cursor_page_arc.as_ref().map(|a| a.as_ref())).await;

        let now_ms = current_time_ms();

        let Some(e) = entity.upgrade() else { return };

        match result {
            Ok((page, has_more)) => {
                // #fix #12: Two-phase completion — accept then complete.
                let _ = e.update(cx, |resource, cx| {
                    if let Some(guard) = resource.accept_current_request(request_id) {
                        resource.complete_success_with_guard(
                            guard, page, has_more, direction.is_next(), now_ms,
                        );
                        // Notify on terminal state change (success).
                        cx.notify();
                    } else {
                        // stale request, result discarded
                    }
                });
                return;
            }
            Err(error) => {
                // #fix #13: Apply retry policy.
                if retry_policy.should_retry(attempt) {
                    let delay_ms = retry_policy.delay_for_attempt(attempt);
                    attempt += 1;

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    // #fix #7: After the retry delay, check whether the signal
                    // has been cancelled. A cancelled fetch should not retry.
                    let Some(e) = entity.upgrade() else { return };
                    let cancelled = read_entity(&e, cx, |r, _| {
                        r.signal().map(|s| s.is_cancelled()).unwrap_or(false)
                    }).unwrap_or(true);
                    if cancelled {
                        return;
                    }

                    // Audit fix #73/#118: After the delay, also confirm this
                    // request is still the active one. If a newer fetch has
                    // superseded it, bail out instead of retrying a stale op.
                    let still_current = read_entity(&e, cx, |r, _| {
                        r.is_current_request(request_id)
                    }).unwrap_or(false);
                    if !still_current {
                        return;
                    }

                    // #fix #1: No cx.notify() during retry wait. Status stays
                    // LoadingWithData/LoadingEmpty during retries, so the
                    // InfiniteQueryObserver deduplicates and no re-render is
                    // needed until terminal state (success or final failure).

                    // Loop to retry
                } else {
                    // No more retries — complete with failure using two-phase protocol
                    // Audit fix #72: notify is moved INSIDE the accept arm so a
                    // discarded (stale) result does not trigger a spurious re-render.
                    let _ = e.update(cx, |resource, cx| {
                        if let Some(guard) = resource.accept_current_request(request_id) {
                            resource.complete_failure_with_guard(guard, error);
                            // Notify on terminal state change (failure).
                            cx.notify();
                        } else {
                            // stale request, result discarded
                        }
                    });
                    return;
                }
            }
        }
    }
}

/// Execute a fetch-next-page operation with a captured `RequestId`.
///
/// Thin direction-specific wrapper around [`run_fetch_page_with_id`]. See that
/// function's docs for the shared behavior (captured `RequestId`, two-phase
/// completion, retry policy).
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
    run_fetch_page_with_id(entity, fetcher, request_id, retry_policy, cx, PageDirection::Next).await;
}

/// Execute a fetch-previous-page operation with a captured `RequestId`.
///
/// Thin direction-specific wrapper around [`run_fetch_page_with_id`]. Same
/// fixes as [`run_fetch_next_page_with_id`]:
/// - Captured `RequestId` prevents stale-ID acceptance
/// - Two-phase completion protocol
/// - Retry policy on failure
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
    run_fetch_page_with_id(entity, fetcher, request_id, retry_policy, cx, PageDirection::Previous).await;
}
