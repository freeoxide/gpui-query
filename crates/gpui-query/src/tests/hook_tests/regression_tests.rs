use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext};

use crate::core::{
    CachePolicy, InfiniteQueryResource, MutationResource, MutationStatus, QueryError, QueryKey,
    QueryResource, RequestPolicy, RetryPolicy,
};
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

#[gpui::test]
fn hook_query_discarded_success_keeps_active_retry_count(cx: &mut TestAppContext) {
    setup_test(cx);

    let gate_a = Gate::new();
    let gate_a_for_fetch = gate_a.clone();
    let gate_b = Gate::new();
    let gate_b_for_fetch = gate_b.clone();
    let b_calls = Arc::new(Mutex::new(0u32));
    let b_calls_for_fetch = b_calls.clone();
    let executor = cx.background_executor.clone();

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<&'static str, QueryError, _>(
            QueryKey::from("discarded-success-retry-count"),
            CachePolicy::NoCache,
            RequestPolicy::LatestWins,
            cx,
        );
        entity.update(cx, |r, _| {
            r.set_retry_policy(RetryPolicy::new(3).with_delay(0))
        });

        let executor_for_a = executor.clone();
        fetch_query(
            &entity,
            move || {
                let gate_a_for_fetch = gate_a_for_fetch.clone();
                let executor_for_a = executor_for_a.clone();
                async move {
                    gate_a_for_fetch.wait(&executor_for_a).await;
                    Ok::<_, QueryError>("superseded-data")
                }
            },
            cx,
        );

        let executor_for_b = executor.clone();
        fetch_query(
            &entity,
            move || {
                let b_calls_for_fetch = b_calls_for_fetch.clone();
                let gate_b_for_fetch = gate_b_for_fetch.clone();
                let executor_for_b = executor_for_b.clone();
                async move {
                    let n = {
                        let mut g = b_calls_for_fetch.lock().unwrap();
                        *g += 1;
                        *g
                    };
                    if n >= 2 {
                        gate_b_for_fetch.wait(&executor_for_b).await;
                    }
                    Err::<_, QueryError>(QueryError::response("b-fail"))
                }
            },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    gate_a.release();
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(5));
    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(
            harness.read(cx).entity.read(cx).retry_count(),
            1,
            "a discarded superseded success must not clobber the active \
             request's mid-flight retry count"
        );
    });

    gate_b.release();
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(20));
    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(crate::core::QueryStatus::Failure, resource.status());
        assert!(
            resource.data().is_none(),
            "the superseded success must never land"
        );
        assert_eq!(resource.retry_count(), 0);
    });
}

#[gpui::test]
fn hook_infinite_retry_count_tracks_attempts(cx: &mut TestAppContext) {
    setup_query_client(cx);

    let calls = Arc::new(Mutex::new(0u32));
    let calls_for_fetch = calls.clone();
    let gate = Gate::new();
    let gate_for_fetch = gate.clone();
    let executor = cx.background_executor.clone();

    struct H {
        entity: Entity<InfiniteQueryResource<Vec<i32>, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_infinite_query(
            InfiniteQueryOptions::new("infinite-retry-count")
                .cache_policy(CachePolicy::Ttl { ttl_ms: 0 })
                .retry_policy(RetryPolicy::new(5).with_delay(0)),
            move |_last_page| {
                let calls_for_fetch = calls_for_fetch.clone();
                let gate_for_fetch = gate_for_fetch.clone();
                let executor = executor.clone();
                async move {
                    let n = {
                        let mut g = calls_for_fetch.lock().unwrap();
                        *g += 1;
                        *g
                    };
                    if n >= 2 {
                        gate_for_fetch.wait(&executor).await;
                    }
                    Err::<_, QueryError>(QueryError::response("inf-transient"))
                }
            },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(
            harness.read(cx).entity.read(cx).retry_count(),
            1,
            "infinite retry attempts must be reflected in retry_count while \
             the retry sequence is in flight"
        );
    });

    gate.release();
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(20));
    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(
            crate::core::QueryStatus::Failure,
            harness.read(cx).entity.read(cx).status()
        );
    });
}

#[gpui::test]
fn hook_mutation_retry_count_reset_on_success(cx: &mut TestAppContext) {
    setup_test(cx);

    let calls = Arc::new(Mutex::new(0u32));
    let calls_for_mutator = calls.clone();

    struct H {
        mutation: Entity<MutationResource<String, String, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_mutation::<String, String, QueryError, _>(
            MutationOptions {
                retry_policy: RetryPolicy::new(2).with_delay(0),
                gc_time_ms: 300_000,
            },
            cx,
        );
        mutate(
            &entity,
            "vars".to_string(),
            move |_v| {
                let calls_for_mutator = calls_for_mutator.clone();
                async move {
                    let n = {
                        let mut g = calls_for_mutator.lock().unwrap();
                        *g += 1;
                        *g
                    };
                    if n < 3 {
                        Err::<String, _>(QueryError::response("transient"))
                    } else {
                        Ok::<_, QueryError>("recovered".to_string())
                    }
                }
            },
            cx,
        );
        H { mutation: entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).mutation.read(cx);
        assert_eq!(resource.status(), MutationStatus::Success);
        assert_eq!(
            resource.retry_count(),
            0,
            "an accepted success must reset the retry counter, matching the \
             query family"
        );
    });
}
