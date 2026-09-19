//! Retry-aware page-fetch runners with two-phase completion.

use std::sync::Arc;

use crate::core::{InfiniteQueryResource, RequestId};

use crate::hook::{current_time_ms, read_entity};

/// The runners differ only in cursor page and the `is_next` flag passed to
/// [`InfiniteQueryResource::complete_success_with_guard`]; this enum carries that.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PageDirection {
    Next,
    Previous,
}

impl PageDirection {
    fn is_next(self) -> bool {
        matches!(self, PageDirection::Next)
    }

    /// Last page for `Next`, first for `Previous`; the `Arc<T>` accessor is a refcount bump, no page clone.
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

/// The `request_id` comes from `begin_fetch_*` (never re-read after the fetcher),
/// and completion is two-phase so a superseded request can never write; a
/// cancelled or superseded fetch stops retrying after the delay.
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
