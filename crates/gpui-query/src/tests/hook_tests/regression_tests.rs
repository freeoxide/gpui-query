use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext};

use crate::core::{MutationResource, QueryError};
use crate::hook::*;
use crate::tests::test_support::*;

#[gpui::test]
fn test_stored_mutation_task_aborted_when_entity_dropped(cx: &mut TestAppContext) {
    setup_test(cx);

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

        cx.update(|cx| {
            assert!(
                harness.read(cx)._mutation.read(cx).is_loading(),
                "mutation still Loading before entity drop"
            );
        });

        drop(harness);
    }

    cx.update(|_| {});

    gate.release();
    cx.run_until_parked();

    assert_eq!(
        *landed.lock().unwrap(),
        0,
        "superseded/dropped mutation task must be aborted — its post-gate side \
         effect should never land"
    );
}

#[gpui::test]
fn test_mutate_from_two_spawn_contexts_second_rejected(cx: &mut TestAppContext) {
    setup_test(cx);

    let first_call_count = Arc::new(Mutex::new(0u32));
    let second_call_count = Arc::new(Mutex::new(0u32));

    let gate = Gate::new();
    let gate_for_first = gate.clone();
    let executor = cx.background_executor.clone();

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

    let sc = second_call_count.clone();
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

    cx.run_until_parked();
    assert!(
        second_ran.is_released(),
        "second mutate's spawn context must execute so the TOCTOU guard is \
         actually exercised"
    );

    assert_eq!(
        *second_call_count.lock().unwrap(),
        0,
        "second mutate from a different spawn context must be rejected while \
         the first is Loading"
    );
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

    gate.release();
    cx.run_until_parked();
}
