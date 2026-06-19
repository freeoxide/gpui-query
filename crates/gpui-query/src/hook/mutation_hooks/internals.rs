//! Internal retry loops for mutations.
//!
//! Audit fix #15: The previous `run_mutation_loop` and
//! `run_mutation_loop_with_callbacks` were ~90% duplicated. Both are now thin
//! wrappers over a single [`run_mutation_loop_inner`] that takes
//! `Option<MutationCallbacks<T, E>>` (`None` for the no-callback variant).
//!
//! Audit fix #3: A new [`run_mutation_loop_by_ref`] (and
//! [`run_mutation_loop_by_ref_with_callbacks`]) accepts a `Fn(&V) -> Fut`
//! mutator so the retry loop borrows the variables via the stored `Arc<V>`
//! instead of cloning `V` on every attempt. The legacy `Fn(V) -> Fut` loops
//! are kept for backward compatibility and adapt via a thin wrapper closure
//! that performs the single `V::clone` per attempt the old API requires.
//!
//! `run_mutation_loop` and `run_mutation_loop_with_callbacks` handle the
//! async retry logic with backoff, cancelled-mutation detection, and
//! lifecycle callback invocation.

use std::sync::Arc;


use crate::core::{MutationResource, RetryPolicy};

use super::super::options::MutationCallbacks;

use crate::hook::read_entity;

/// Unified retry loop for mutations.
///
/// Audit fix #15: Single implementation shared by the no-callback and
/// with-callback variants. `callbacks` is `None` for the no-callback path.
///
/// Audit fix #19: When retries are available, uses `increment_retry()` +
/// `prepare_retry()` instead of `complete_failure()` followed by `retry()`.
/// This avoids a transient `Failure` status that would cause observers to see
/// a brief Failure flash between retry attempts. Only `complete_failure()` is
/// called when retries are exhausted, which represents a terminal failure.
///
/// Audit fix #1: Does NOT call `cx.notify()` after `increment_retry()` or
/// `prepare_retry()` because those operations do not change the mutation status
/// (stays Loading). The `MutationObserver` only triggers `cx.notify()` on actual
/// status changes, so these intermediate updates are invisible to the component.
///
/// Audit fix #3: Variables are passed as `Arc<V>` and the mutator takes `&V`,
/// so each retry attempt only borrows the variables (no `V::clone` per attempt).
///
/// Audit fix #9: After each retry delay, checks whether the mutation is still
/// in Loading state. If it was cancelled or reset (no longer Loading), stops
/// retrying immediately (and fires callbacks when present).
///
/// Audit fix #27/#121: `entity.update` results are discarded via `let _ =` to
/// silence `unused_must_use` under `AsyncApp` (where `update` returns
/// `Result<R>`).
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
        // Audit fix #3: borrow the variables via the Arc; no V::clone per attempt.
        let result = mutator(&*variables).await;

        match result {
            Ok(data) => {
                // Clone data before update only when callbacks need it.
                let data_for_callback =
                    if callbacks.is_some() { Some(data.clone()) } else { None };

                let Some(entity) = weak.upgrade() else {
                    // Audit fix #9: Entity dropped during mutation. Fire
                    // on_settled with None for both to indicate discard.
                    if let Some(ref cb) = callbacks
                        && let Some(ref f) = cb.on_settled {
                            f(None, None);
                        }
                    return;
                };
                let _ = entity.update(cx, |resource, cx| {
                    resource.complete_success(data);
                    cx.notify();
                });

                // Fire success/settled callbacks outside entity borrow so
                // they can safely call entity.update().
                if let Some(ref cb) = callbacks {
                    if let Some(ref d) = data_for_callback
                        && let Some(ref f) = cb.on_success {
                            f(d);
                        }
                    if let Some(ref f) = cb.on_settled {
                        f(data_for_callback.as_ref(), None);
                    }
                }

                return;
            }
            Err(error) => {
                // Clone error before update only when callbacks need it.
                let error_for_callback =
                    if callbacks.is_some() { Some(error.clone()) } else { None };

                if retry_policy.should_retry(attempt) {
                    // Audit fix #19: Do NOT call complete_failure() here.
                    // Instead, just increment the retry counter and wait for
                    // the delay. This avoids a transient Failure -> Loading
                    // flash for observers.
                    let delay_ms = retry_policy.delay_for_attempt(attempt);

                    let Some(entity) = weak.upgrade() else {
                        // Audit fix #9: Entity dropped between mutator failure and retry.
                        if let Some(ref cb) = callbacks {
                            if let Some(ref ec) = error_for_callback
                                && let Some(ref f) = cb.on_error {
                                    f(ec);
                                }
                            if let Some(ref f) = cb.on_settled {
                                f(None, error_for_callback.as_ref());
                            }
                        }
                        return;
                    };
                    let _ = entity.update(cx, |resource, _cx| {
                        resource.increment_retry();
                        // Audit fix #1: No cx.notify() -- increment_retry does
                        // not change status (stays Loading).
                    });

                    if delay_ms > 0 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(delay_ms))
                            .await;
                    }

                    // Audit fix #9: After the retry delay, check whether the
                    // mutation is still in Loading state. If it was cancelled
                    // or reset, stop retrying immediately.
                    let Some(entity) = weak.upgrade() else {
                        // Audit fix #9/#10: Entity dropped during retry delay.
                        if let Some(ref cb) = callbacks {
                            if let Some(ref ec) = error_for_callback
                                && let Some(ref f) = cb.on_error {
                                    f(ec);
                                }
                            if let Some(ref f) = cb.on_settled {
                                f(None, error_for_callback.as_ref());
                            }
                        }
                        return;
                    };
                    if !read_entity(&entity, cx, |r, _| r.is_loading()).unwrap_or(false) {
                        // Mutation was cancelled or reset during the delay.
                        // Fire error callbacks so callers get a terminal notification.
                        if let Some(ref cb) = callbacks {
                            if let Some(ref ec) = error_for_callback
                                && let Some(ref f) = cb.on_error {
                                    f(ec);
                                }
                            if let Some(ref f) = cb.on_settled {
                                f(None, error_for_callback.as_ref());
                            }
                        }
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "DEBUG: run_mutation_loop_inner: mutation no longer Loading after retry delay, aborting"
                        );
                        return;
                    }

                    // After delay, prepare for retry (refresh signal, stay in Loading).
                    let _ = entity.update(cx, |resource, _cx| {
                        resource.prepare_retry();
                        // Audit fix #1: No cx.notify() -- prepare_retry does
                        // not change status (stays Loading).
                    });

                    attempt += 1;
                } else {
                    // No more retries -- terminal failure.
                    // Audit fix #10: Capture entity availability before
                    // complete_failure so callbacks still fire even if entity
                    // is dropped between the update and callback invocation.
                    let entity_available = weak.upgrade();
                    if let Some(entity) = entity_available {
                        let _ = entity.update(cx, |resource, cx| {
                            resource.complete_failure(error);
                            // Audit fix #4: Reset retry_count on terminal failure.
                            resource.reset_retry_count();
                            cx.notify();
                        });
                    }

                    // Fire error and settled callbacks outside entity borrow.
                    // These fire regardless of whether entity is still alive
                    // (Audit fix #9/#10).
                    if let Some(ref cb) = callbacks {
                        if let Some(ref ec) = error_for_callback
                            && let Some(ref f) = cb.on_error {
                                f(ec);
                            }

                        if let Some(ref f) = cb.on_settled {
                            f(None, error_for_callback.as_ref());
                        }
                    }

                    return;
                }
            }
        }
    }
}

/// Core retry loop for mutations (legacy `Fn(V) -> Fut` mutator).
///
/// Wraps the mutator so each attempt performs a single `V::clone` (preserving
/// the original semantics) and delegates to [`run_mutation_loop_inner`].
///
/// Audit fix #119: The `+ Clone` bound on `F` has been dropped — the mutator
/// is only ever borrowed (`&mutator`), never cloned.
pub(super) async fn run_mutation_loop<V, T, E, F, Fut>(
    weak: &gpui::WeakEntity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    retry_policy: &RetryPolicy,
    cx: &mut gpui::AsyncApp,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let wrapper = move |v: &V| mutator(v.clone());
    run_mutation_loop_inner(weak, variables, wrapper, retry_policy, None, cx).await;
}

/// Like [`run_mutation_loop`] but fires lifecycle callbacks on final outcome
/// (legacy `Fn(V) -> Fut` mutator).
///
/// Audit fix #119: The `+ Clone` bound on `F` has been dropped.
pub(super) async fn run_mutation_loop_with_callbacks<V, T, E, F, Fut>(
    weak: &gpui::WeakEntity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    retry_policy: &RetryPolicy,
    callbacks: MutationCallbacks<T, E>,
    cx: &mut gpui::AsyncApp,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    F: Fn(V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let wrapper = move |v: &V| mutator(v.clone());
    run_mutation_loop_inner(weak, variables, wrapper, retry_policy, Some(callbacks), cx).await;
}

/// Audit fix #3: Retry loop for the new `Fn(&V) -> Fut` mutator signature.
///
/// Borrows the variables via the stored `Arc<V>` on every attempt — no
/// `V::clone` per retry. Use this with [`super::super::mutate_by_ref`] /
/// [`super::super::mutate_arc`].
///
/// Audit fix #119: No `+ Clone` bound on `F` (mutator is borrowed, not cloned).
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

/// Audit fix #3: Like [`run_mutation_loop_by_ref`] but fires lifecycle
/// callbacks on final outcome.
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
