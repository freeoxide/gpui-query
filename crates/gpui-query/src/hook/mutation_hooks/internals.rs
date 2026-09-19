//! All entrypoints funnel into [`run_mutation_loop_inner`], whose `Fn(&V)`
//! mutator borrows the variables from the stored `Arc<V>` (no `V::clone` per
//! retry); the `Fn(V)` public entrypoints adapt at the call site.

use std::sync::Arc;

use crate::core::{MutationResource, RetryPolicy};

use super::super::options::MutationCallbacks;

use crate::hook::read_entity;

/// Retries via `increment_retry()` + `prepare_retry()` so observers never see
/// a transient Failure between attempts; only exhausted retries produce a
/// terminal `complete_failure()`. Stops once the mutation leaves Loading
/// (cancelled or reset); intermediate calls don't notify (observer dedupes).
async fn run_mutation_loop_inner<V, T, E, F, Fut>(
    weak: &gpui::WeakEntity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    retry_policy: &RetryPolicy,
    callbacks: Option<MutationCallbacks<T, E>>,
    cx: &mut gpui::AsyncApp,
) where
    V: Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(&V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let mut attempt: u32 = 0;

    loop {
        let result = mutator(&*variables).await;

        match result {
            Ok(data) => {
                let data_for_callback = callbacks.is_some().then(|| data.clone());

                let Some(entity) = weak.upgrade() else {
                    if let Some(ref cb) = callbacks
                        && let Some(ref f) = cb.on_settled
                    {
                        f(None, None);
                    }
                    return;
                };
                let _ = entity.update(cx, |resource, cx| {
                    resource.complete_success(data);
                    cx.notify();
                    #[cfg(feature = "persist")]
                    cx.default_global::<crate::client::CacheMutation>();
                });

                // Fire outside the entity borrow so callbacks can call entity.update().
                if let Some(ref cb) = callbacks {
                    if let Some(ref d) = data_for_callback
                        && let Some(ref f) = cb.on_success
                    {
                        f(d);
                    }
                    if let Some(ref f) = cb.on_settled {
                        f(data_for_callback.as_ref(), None);
                    }
                }

                return;
            }
            Err(error) => {
                let error_for_callback = callbacks.is_some().then(|| error.clone());

                if retry_policy.should_retry(attempt) {
                    let delay_ms = retry_policy.delay_for_attempt(attempt);

                    let Some(entity) = weak.upgrade() else {
                        fire_error_callbacks(&callbacks, &error_for_callback);
                        return;
                    };
                    let _ = entity.update(cx, |resource, _cx| {
                        resource.increment_retry();
                    });

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    let Some(entity) = weak.upgrade() else {
                        fire_error_callbacks(&callbacks, &error_for_callback);
                        return;
                    };
                    if !read_entity(&entity, cx, |r, _| r.is_loading()).unwrap_or(false) {
                        fire_error_callbacks(&callbacks, &error_for_callback);
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "DEBUG: run_mutation_loop_inner: mutation no longer Loading after retry delay, aborting"
                        );
                        return;
                    }

                    let _ = entity.update(cx, |resource, _cx| {
                        resource.prepare_retry();
                    });

                    attempt += 1;
                } else {
                    if let Some(entity) = weak.upgrade() {
                        let _ = entity.update(cx, |resource, cx| {
                            resource.complete_failure(error);
                            resource.reset_retry_count();
                            cx.notify();
                            #[cfg(feature = "persist")]
                            cx.default_global::<crate::client::CacheMutation>();
                        });
                    }

                    fire_error_callbacks(&callbacks, &error_for_callback);
                    return;
                }
            }
        }
    }
}

/// Fires `on_error` / `on_settled` whether or not the entity survived.
fn fire_error_callbacks<T, E>(
    callbacks: &Option<MutationCallbacks<T, E>>,
    error_for_callback: &Option<E>,
) {
    if let Some(cb) = callbacks {
        if let Some(ec) = error_for_callback
            && let Some(ref f) = cb.on_error
        {
            f(ec);
        }
        if let Some(ref f) = cb.on_settled {
            f(None, error_for_callback.as_ref());
        }
    }
}

pub(super) async fn run_mutation_loop_by_ref<V, T, E, F, Fut>(
    weak: &gpui::WeakEntity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    retry_policy: &RetryPolicy,
    cx: &mut gpui::AsyncApp,
) where
    V: Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(&V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    run_mutation_loop_inner(weak, variables, mutator, retry_policy, None, cx).await;
}

pub(super) async fn run_mutation_loop_by_ref_with_callbacks<V, T, E, F, Fut>(
    weak: &gpui::WeakEntity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    retry_policy: &RetryPolicy,
    callbacks: MutationCallbacks<T, E>,
    cx: &mut gpui::AsyncApp,
) where
    V: Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(&V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    run_mutation_loop_inner(weak, variables, mutator, retry_policy, Some(callbacks), cx).await;
}
