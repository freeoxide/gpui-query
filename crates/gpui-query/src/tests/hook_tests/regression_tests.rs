//! Regression tests for audit findings #131 and #132.
//!
//! #131: A stored mutation/infinite task is cancelled when superseded. The
//! production mechanism is `CurrentTask` + `set_current_task`: storing a new
//! task drops the previous `gpui::Task`, and dropping a GPUI task aborts its
//! future. We verify that an in-flight mutation whose entity is dropped (the
//! unmount/replacement path) has its post-gate side effect suppressed — the
//! gated mutator never reaches the code after `gate.wait(...).await` because
//! the task is aborted.
//!
//! #132: The real TOCTOU race — two `mutate()` calls issued from *different*
//! async spawn contexts. The existing `test_mutate_double_while_loading_*`
//! tests cover only the synchronous double-call within one `cx.update` scope.
//! Here we interleave via independent background spawns + [`Gate`]; the second
//! mutate must still be rejected while the first is `Loading`, proving the
//! atomic check+begin guard (audit #7/#8) holds across genuinely concurrent
//! callers.

use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext};

use crate::core::{MutationResource, QueryError};
use crate::hook::*;
use crate::tests::test_support::*;

// ── #131: stored task is cancelled when its entity is dropped ───────────────

#[gpui::test]
fn test_stored_mutation_task_aborted_when_entity_dropped(cx: &mut TestAppContext) {
    setup_test(cx);

    // Side-effect counter incremented *after* the gate is released. If the
    // task is correctly aborted on entity drop, the post-gate increment never
    // runs and the counter stays at 0.
    let landed = Arc::new(Mutex::new(0u32));
    let landed_clone = landed.clone();

    let gate = Gate::new();
    let gate_clone = gate.clone();
    let executor = cx.background_executor.clone();

    // Create the mutation entity and start a gated mutate, then drop the
    // entity handle while the mutator is still parked on the gate.
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

        // Dropping `harness` drops the entity → drops MutationResource → drops
        // CurrentTask → drops the gpui::Task → aborts the gated future.
        drop(harness);
    }

    // GPUI defers the actual entity release until the next `App::update` flushes
    // effects (`release_dropped_entities` runs at the end of `App::update`, not
    // during `run_until_parked`). Force that flush here so the entity — and its
    // stored `CurrentTask`/`gpui::Task` — is dropped *before* we release the
    // gate. Without this, the parked future would still be alive when the gate
    // opens, run its post-gate side effect, and `landed` would read 1.
    cx.update(|_| {});

    // Release the gate. If the task had *not* been aborted, the mutator would
    // now proceed past the gate and increment `landed`. The aborted task does
    // not, so `landed` stays 0.
    gate.release();
    cx.run_until_parked();

    assert_eq!(
        *landed.lock().unwrap(),
        0,
        "superseded/dropped mutation task must be aborted — its post-gate side \
         effect should never land (#131: CurrentTask Drop-cancellation)"
    );
}

// ── #132: TOCTOU race — two mutate() calls from different spawn contexts ────
//
// The existing `test_mutate_double_while_loading_*` tests issue both calls
// synchronously inside one `cx.update` scope. This test fires the second
// `mutate()` from an *independent* `Context::spawn` task that re-enters the
// harness entity via `AsyncApp` — the genuine cross-context race shape. The
// atomic check+begin guard (audit #7/#8) must still reject it while the first
// is `Loading`.
//
// We assert the rejection contract directly (the #132 concern): the second
// mutate's fetcher never runs, the first mutate stays in-flight and
// uncorrupted, and there is no double execution. We deliberately do NOT assert
// post-release completion of the gated first mutate here: a completed
// `Context::spawn` task interacts with the `TestAppContext` executor such that
// subsequent `run_until_parked` calls stop draining the background-timer
// wake-up chain that `Gate::wait` relies on (verified empirically: even a
// no-op spawn before `gate.release()` prevents the first task from
// completing). Gated-mutation completion is already covered by
// `retry_reset_tests`; this test's job is the cross-context rejection.

#[gpui::test]
fn test_mutate_from_two_spawn_contexts_second_rejected(cx: &mut TestAppContext) {
    setup_test(cx);

    let first_call_count = Arc::new(Mutex::new(0u32));
    let second_call_count = Arc::new(Mutex::new(0u32));

    // A gate that keeps the *first* mutate's fetcher in flight while the second
    // mutate is issued from a different spawn context.
    let gate = Gate::new();
    let gate_for_first = gate.clone();
    let executor = cx.background_executor.clone();

    // The first mutate is started inside `cx.new` (the proven gated-mutation
    // shape); its fetcher parks on the gate so the mutation stays `Loading`.
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

    // Second mutate — issued from a DIFFERENT async spawn context. We spawn on
    // the harness `Context<H>` (so the task runs under `run_until_parked` and
    // receives a `WeakEntity<H>` + `AsyncApp`), then re-enter the harness
    // entity from inside that distinct context to call `mutate`. This is the
    // real TOCTOU shape: an independent task racing the in-flight one.
    let sc = second_call_count.clone();
    let second_ran = Arc::new(Mutex::new(false));
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
            *second_ran_clone.lock().unwrap() = true;
        })
    });

    // Drive the executor until the spawned second mutate has actually run (and,
    // because the first is still Loading, been rejected). Bounded so a
    // regression that leaves it pending fails loudly.
    for _ in 0..200 {
        cx.run_until_parked();
        if *second_ran.lock().unwrap() {
            break;
        }
    }
    assert!(
        *second_ran.lock().unwrap(),
        "second mutate's spawn context must execute so the TOCTOU guard is \
         actually exercised"
    );

    // ── The #132 contract: the cross-context second mutate is rejected ──
    // The second mutate's fetcher never ran (rejected by the is_loading guard).
    assert_eq!(
        *second_call_count.lock().unwrap(),
        0,
        "second mutate from a different spawn context must be rejected while \
         the first is Loading (#132 TOCTOU race)"
    );
    // The first mutate is still in-flight and uncorrupted: its fetcher ran
    // exactly once and the resource is still Loading.
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

    // Hygiene: release the gate so the parked first fetcher can make progress
    // once the test's executor drains it (not asserted — see module note).
    gate.release();
    cx.run_until_parked();
}
