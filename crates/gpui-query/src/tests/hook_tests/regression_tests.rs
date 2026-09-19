//! Regression tests for stored-task cancellation and the cross-context
//! mutate race.
//!
//! Stored mutation/infinite tasks are cancelled when superseded: `set_current_task`
//! drops the previous `gpui::Task`, and dropping a GPUI task aborts its future.
//! `test_stored_mutation_task_aborted_when_entity_dropped` checks that an in-flight
//! mutation whose entity is dropped never runs its post-gate side effect.
//!
//! `test_mutate_from_two_spawn_contexts_second_rejected` races two `mutate()`
//! calls from different async spawn contexts (the synchronous double-call case
//! is covered by `test_mutate_double_while_loading_*`); the atomic check+begin
//! guard must still reject the second while the first is Loading.

use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext};

use crate::core::{MutationResource, QueryError};
use crate::hook::*;
use crate::tests::test_support::*;

// Stored task is cancelled when its entity is dropped.

#[gpui::test]
fn test_stored_mutation_task_aborted_when_entity_dropped(cx: &mut TestAppContext) {
    setup_test(cx);

    // Counter incremented after the gate is released: stays at 0 if the task
    // is correctly aborted on entity drop.
    let landed = Arc::new(Mutex::new(0u32));
    let landed_clone = landed.clone();

    let gate = Gate::new();
    let gate_clone = gate.clone();
    let executor = cx.background_executor.clone();

    {
        #[allow(dead_code)]
        struct H {
            _mutation: Entity<MutationResource<String, String, QueryError>>,
        }

        let harness = cx.new(|cx| {
            let (entity, _sub) = use_mutation::<String, String, QueryError, _>((), cx);

            mutate(
                &entity,
                "vars".to_string(),
                move |_v| {
                    let landed_clone = landed_clone.clone();
                    let gate_clone = gate_clone.clone();
                    let executor = executor.clone();
                    async move {
                        // Park here. If the task is aborted (entity dropped),
                        // this future is dropped and the line below never runs.
                        gate_clone.wait(&executor).await;
                        *landed_clone.lock().unwrap() += 1;
                        Ok::<_, QueryError>("done".to_string())
                    }
                },
                cx,
            );
            assert!(
                entity.read(cx).is_loading(),
                "mutation should be Loading while the gated fetcher is parked"
            );
            H { _mutation: entity }
        });

        // Sanity: still loading before we drop the harness.
        cx.update(|cx| {
            assert!(
                harness.read(cx)._mutation.read(cx).is_loading(),
                "mutation still Loading before entity drop"
            );
        });

        // Dropping `harness` drops the entity and its CurrentTask, which
        // aborts the gated future.
        drop(harness);
    }

    // GPUI releases dropped entities at the end of `App::update`, not during
    // `run_until_parked`. Force that flush so the entity is gone before the
    // gate opens, otherwise the parked future would still run its post-gate
    // side effect and `landed` would read 1.
    cx.update(|_| {});

    // If the task had not been aborted, the mutator would proceed past the
    // gate and increment `landed`.
    gate.release();
    cx.run_until_parked();

    assert_eq!(
        *landed.lock().unwrap(),
        0,
        "superseded/dropped mutation task must be aborted — its post-gate side \
         effect should never land"
    );
}

// Two mutate() calls from different spawn contexts.
//
// The second `mutate()` fires from an independent `Context::spawn` task that
// re-enters the harness entity via `AsyncApp`: the real cross-context race
// shape. We assert the rejection contract directly (second fetcher never
// runs, first stays in-flight and uncorrupted). We deliberately do NOT assert
// post-release completion of the gated first mutate: a completed
// `Context::spawn` task interacts with the `TestAppContext` executor such
// that later `run_until_parked` calls stop draining the background-timer
// wake-up chain `Gate::wait` relies on. Gated-mutation completion is covered
// by `retry_reset_tests`.

#[gpui::test]
fn test_mutate_from_two_spawn_contexts_second_rejected(cx: &mut TestAppContext) {
    setup_test(cx);

    let first_call_count = Arc::new(Mutex::new(0u32));
    let second_call_count = Arc::new(Mutex::new(0u32));

    // Keeps the first mutate's fetcher in flight while the second mutate is
    // issued from a different spawn context.
    let gate = Gate::new();
    let gate_for_first = gate.clone();
    let executor = cx.background_executor.clone();

    // The first fetcher parks on the gate so the mutation stays Loading.
    #[allow(dead_code)]
    struct H {
        mutation: Entity<MutationResource<String, String, QueryError>>,
    }
    let fc = first_call_count.clone();
    let harness = cx.new(|cx| {
        let (entity, _sub) = use_mutation::<String, String, QueryError, _>((), cx);
        let executor = executor.clone();
        mutate(
            &entity,
            "first".to_string(),
            move |_v| {
                let fc = fc.clone();
                let gate_for_first = gate_for_first.clone();
                let executor = executor.clone();
                async move {
                    *fc.lock().unwrap() += 1;
                    gate_for_first.wait(&executor).await;
                    Ok::<_, QueryError>("first-result".to_string())
                }
            },
            cx,
        );
        H { mutation: entity }
    });

    cx.update(|cx| {
        assert!(
            harness.read(cx).mutation.read(cx).is_loading(),
            "first mutate should be Loading while parked on the gate"
        );
    });

    // Second mutate, issued from a different async spawn context: spawn on the
    // harness `Context<H>` and re-enter the entity via `AsyncApp` to call
    // mutate. An independent task racing the in-flight one.
    let sc = second_call_count.clone();
    // A Gate signals that the spawned task ran its context; the main task
    // drains once.
    let second_ran = Gate::new();
    let second_ran_clone = second_ran.clone();
    let _second_task = harness.update(cx, |_this, cx| {
        cx.spawn(async move |weak_self, async_cx| {
            if let Some(h) = weak_self.upgrade() {
                let _ = h.update(async_cx, |this, cx| {
                    let sc = sc.clone();
                    mutate(
                        &this.mutation,
                        "second".to_string(),
                        move |_v| {
                            let sc = sc.clone();
                            async move {
                                *sc.lock().unwrap() += 1;
                                Ok::<_, QueryError>("second-result".to_string())
                            }
                        },
                        cx,
                    );
                });
            }
            second_ran_clone.release();
        })
    });

    // Drain so the spawned second mutate runs (and, because the first is
    // still Loading, is rejected).
    cx.run_until_parked();
    assert!(
        second_ran.is_released(),
        "second mutate's spawn context must execute so the TOCTOU guard is \
         actually exercised"
    );

    // The second mutate's fetcher never ran (rejected by the is_loading guard).
    assert_eq!(
        *second_call_count.lock().unwrap(),
        0,
        "second mutate from a different spawn context must be rejected while \
         the first is Loading"
    );
    // The first mutate is still in-flight and uncorrupted.
    assert_eq!(
        *first_call_count.lock().unwrap(),
        1,
        "first mutate's fetcher should run exactly once"
    );
    cx.update(|cx| {
        assert!(
            harness.read(cx).mutation.read(cx).is_loading(),
            "first mutate must still be Loading — the rejected second mutate \
             must not have corrupted the in-flight first"
        );
    });

    // Hygiene: release the gate so the parked first fetcher can progress
    // (not asserted; see the note above the test).
    gate.release();
    cx.run_until_parked();
}
