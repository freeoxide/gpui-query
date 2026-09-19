use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext};

use crate::core::{CachePolicy, QueryError, QueryKey, QueryResource, QueryStatus, RequestPolicy};
use crate::hook::*;
use crate::tests::test_support::*;

#[gpui::test]
fn test_use_query_manual_no_auto_fetch_then_manual_fetch(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<&'static str, QueryError, _>(
            QueryKey::from("manual-no-auto"),
            CachePolicy::Ttl { ttl_ms: 0 },
            RequestPolicy::LatestWins,
            cx,
        );
        assert_eq!(entity.read(cx).status(), QueryStatus::Idle);
        assert!(entity.read(cx).data().is_none());
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(
            harness.read(cx).entity.read(cx).status(),
            QueryStatus::Idle,
            "use_query_manual should never auto-fetch"
        );
    });

    harness.update(cx, |this, cx| {
        fetch_query(
            &this.entity,
            || async { Ok::<_, QueryError>("manual-result") },
            cx,
        );
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(resource.data(), Some(&"manual-result"));
    });
}

#[gpui::test]
fn test_use_query_manual_multiple_fetches(cx: &mut TestAppContext) {
    setup_query_client(cx);

    let call_count = Arc::new(Mutex::new(0u32));
    let cc1 = call_count.clone();
    let cc2 = call_count.clone();

    struct H {
        entity: Entity<QueryResource<u32, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<u32, QueryError, _>(
            QueryKey::from("multi-manual"),
            CachePolicy::NoCache,
            RequestPolicy::LatestWins,
            cx,
        );
        H { entity }
    });

    let cc_first = cc1.clone();
    harness.update(cx, |this, cx| {
        fetch_query(
            &this.entity,
            move || {
                let cc_first = cc_first.clone();
                async move {
                    *cc_first.lock().unwrap() += 1;
                    Ok::<_, QueryError>(1_u32)
                }
            },
            cx,
        );
    });

    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(harness.read(cx).entity.read(cx).data(), Some(&1));
    });

    harness.update(cx, |this, cx| {
        fetch_query(
            &this.entity,
            move || {
                let cc2 = cc2.clone();
                async move {
                    *cc2.lock().unwrap() += 1;
                    Ok::<_, QueryError>(2_u32)
                }
            },
            cx,
        );
    });

    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(harness.read(cx).entity.read(cx).data(), Some(&2));
    });
    assert_eq!(
        *call_count.lock().unwrap(),
        2,
        "both fetches should have executed"
    );
}

#[gpui::test]
fn test_fetch_query_on_idle_entity(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<String, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<String, QueryError, _>(
            QueryKey::from("fresh-key"),
            CachePolicy::NoCache,
            RequestPolicy::LatestWins,
            cx,
        );
        assert_eq!(entity.read(cx).status(), QueryStatus::Idle);

        fetch_query(
            &entity,
            || async { Ok::<_, QueryError>("fresh-data".to_string()) },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(resource.data(), Some(&"fresh-data".to_string()));
    });
}

#[gpui::test]
fn test_fetch_query_after_resource_reset(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query(
            QueryOptions::new("reset-test").cache_policy(CachePolicy::NoCache),
            |_signal| async move { Ok::<_, QueryError>("initial") },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        assert_eq!(harness.read(cx).entity.read(cx).data(), Some(&"initial"));
    });
    let entity = cx.update(|cx| harness.read(cx).entity.clone());
    cx.update(|cx| {
        entity.update(cx, |r, _| {
            r.reset();
        });
    });

    cx.update(|cx| {
        assert_eq!(harness.read(cx).entity.read(cx).status(), QueryStatus::Idle);
    });

    harness.update(cx, |this, cx| {
        fetch_query(
            &this.entity,
            || async { Ok::<_, QueryError>("after-reset") },
            cx,
        );
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(resource.data(), Some(&"after-reset"));
    });
}

#[gpui::test]
fn test_fetch_query_concurrent_calls_latest_wins(cx: &mut TestAppContext) {
    setup_test(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let gate = Gate::new();
    let gate_clone = gate.clone();
    let executor = cx.background_executor.clone();

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<&'static str, QueryError, _>(
            QueryKey::from("concurrent-fetch"),
            CachePolicy::NoCache,
            RequestPolicy::LatestWins,
            cx,
        );
        let executor = executor.clone();
        fetch_query(
            &entity,
            move || {
                let gate_clone = gate_clone.clone();
                let executor = executor.clone();
                async move {
                    gate_clone.wait(&executor).await;
                    Ok::<_, QueryError>("first")
                }
            },
            cx,
        );
        fetch_query(&entity, || async { Ok::<_, QueryError>("second") }, cx);
        H { entity }
    });

    gate.release();

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(
            resource.data(),
            Some(&"second"),
            "LatestWins: second fetch should be the winner"
        );
    });
}

#[gpui::test]
fn test_fetch_query_with_signal_completes(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<&'static str, QueryError, _>(
            QueryKey::from("signal-fetch"),
            CachePolicy::NoCache,
            RequestPolicy::LatestWins,
            cx,
        );
        fetch_query_with_signal(
            &entity,
            |_signal| async { Ok::<_, QueryError>("signal-result") },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(resource.data(), Some(&"signal-result"));
    });
}

#[gpui::test]
fn test_fetch_query_with_signal_failure(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_manual::<&'static str, QueryError, _>(
            QueryKey::from("signal-fail"),
            CachePolicy::NoCache,
            RequestPolicy::LatestWins,
            cx,
        );
        fetch_query_with_signal(
            &entity,
            |_signal| async { Err::<&'static str, _>(QueryError::response("signal-error")) },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Failure);
        let err = resource.error().expect("should have error");
        assert!(err.to_string().contains("signal-error"));
    });
}
