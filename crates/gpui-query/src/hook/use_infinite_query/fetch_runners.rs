//! Internal async fetch runners for infinite query page fetches.
//!
//! These are the retry-aware async functions that execute the actual fetch
//! operations with captured `RequestId`s and two-phase completion protocol.

use std::sync::Arc;

use crate::core::{InfiniteQueryResource, RequestId};

use crate::hook::{current_time_ms, read_entity};

// ── Internal fetch runners ───────────────────────────────────────────────

/// Execute a fetch-next-page operation with a captured `RequestId`.
///
/// #fix #5/#6: The `request_id` is the one returned from `begin_fetch_next`,
/// not re-read after the fetcher completes. This prevents stale-ID acceptance
/// when concurrent fetches are in flight.
///
/// #fix #12: Uses two-phase completion (`accept_current_request` then
/// `complete_success_with_guard`/`complete_failure_with_guard`) to close
/// the race window between reading active_request_id and completing.
///
/// #fix #13: Applies retry policy on fetch failure.
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
    let mut attempt: u32 = 0;

    loop {
        // Audit fix #5/#73: Read the last page inside the loop via the cheap
        // refcount-bumped `Arc<T>` accessor (no full page clone), and re-read
        // it fresh each retry so the fetcher sees up-to-date data. We capture
        // the `Arc<T>` and hand the fetcher an `Option<&T>` via `as_ref` — the
        // fetcher signature is unchanged.
        let last_page_arc: Option<Arc<T>> = {
            let e = match entity.upgrade() {
                Some(e) => e,
                None => return,
            };
            read_entity(&e, cx, |r, _| r.last_page_arc()).flatten()
        };

        let result = fetcher(last_page_arc.as_ref().map(|a| a.as_ref())).await;

        let now_ms = current_time_ms();

        let e = match entity.upgrade() {
            Some(e) => e,
            None => return,
        };

        match result {
            Ok((page, has_more)) => {
                // #fix #12: Two-phase completion — accept then complete.
                let _ = e.update(cx, |resource, cx| {
                    if let Some(guard) = resource.accept_current_request(request_id) {
                        resource.complete_success_with_guard(
                            &guard, page, has_more, true, now_ms,
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
                    let e = match entity.upgrade() {
                        Some(e) => e,
                        None => return,
                    };
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
                            resource.complete_failure_with_guard(&guard, error);
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

/// Execute a fetch-previous-page operation with a captured `RequestId`.
///
/// Same fixes as `run_fetch_next_page_with_id`:
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
    let mut attempt: u32 = 0;

    loop {
        // Audit fix #5/#73: Read the first page inside the loop via the cheap
        // refcount-bumped `Arc<T>` accessor (no full page clone), and re-read
        // it fresh each retry so the fetcher sees up-to-date data. We capture
        // the `Arc<T>` and hand the fetcher an `Option<&T>` via `as_ref` — the
        // fetcher signature is unchanged.
        let first_page_arc: Option<Arc<T>> = {
            let e = match entity.upgrade() {
                Some(e) => e,
                None => return,
            };
            read_entity(&e, cx, |r, _| r.first_page_arc()).flatten()
        };

        let result = fetcher(first_page_arc.as_ref().map(|a| a.as_ref())).await;

        let now_ms = current_time_ms();

        let e = match entity.upgrade() {
            Some(e) => e,
            None => return,
        };

        match result {
            Ok((page, has_more)) => {
                let _ = e.update(cx, |resource, cx| {
                    if let Some(guard) = resource.accept_current_request(request_id) {
                        resource.complete_success_with_guard(
                            &guard, page, has_more, false, now_ms,
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
                if retry_policy.should_retry(attempt) {
                    let delay_ms = retry_policy.delay_for_attempt(attempt);
                    attempt += 1;

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    // #fix #7: Check signal cancellation after retry delay.
                    let e = match entity.upgrade() {
                        Some(e) => e,
                        None => return,
                    };
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

                    // #fix #1: No cx.notify() during retry wait.
                } else {
                    // Audit fix #72: notify is moved INSIDE the accept arm so a
                    // discarded (stale) result does not trigger a spurious re-render.
                    let _ = e.update(cx, |resource, cx| {
                        if let Some(guard) = resource.accept_current_request(request_id) {
                            resource.complete_failure_with_guard(&guard, error);
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
