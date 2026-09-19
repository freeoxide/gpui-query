use crate::core::*;
use crate::tests::test_support::*;

#[test]
fn invariant_initial_state_is_consistent() {
    let r = fresh_resource();
    assert_eq!(r.status(), QueryStatus::Idle);
    assert!(r.data().is_none(), "Idle => data must be None");
    assert!(r.error().is_none(), "Idle => error must be None");
    assert!(r.active_request_id().is_none());
}

#[test]
fn invariant_after_begin_loading_empty() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let _ = r.begin_request(&mut s, 100, QueryFetchMode::Normal);
    assert_eq!(r.status(), QueryStatus::LoadingEmpty);
    assert!(r.data().is_none(), "LoadingEmpty => data must be None");
    assert!(r.error().is_none(), "begin_request clears error");
    assert!(r.active_request_id().is_some());
    assert!(r.signal().is_some());
}

#[test]
fn invariant_after_begin_loading_with_data() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    r.complete_current_success(rid1, "data1", 200);

    let rid2 = begin_request_id(&mut r, &mut s, 300, QueryFetchMode::Normal);
    assert_eq!(r.status(), QueryStatus::LoadingWithData);
    assert!(
        r.data().is_some(),
        "LoadingWithData => data should still be present"
    );
    assert_eq!(r.data(), Some(&"data1"), "data preserved during refetch");
    assert!(r.error().is_none(), "begin_request clears error");
    assert_eq!(r.active_request_id(), Some(rid2));

    r.complete_current_success(rid2, "data2", 400);
    assert_eq!(r.status(), QueryStatus::Success);
    assert_eq!(r.data(), Some(&"data2"));
    assert_eq!(r.previous_data(), Some(&"data1"));
}

#[test]
fn invariant_after_complete_success() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    r.complete_current_success(rid, "result", 200);

    assert_eq!(r.status(), QueryStatus::Success);
    assert_eq!(r.data(), Some(&"result"), "Success => data must be Some");
    assert!(r.error().is_none(), "Success => error must be None");
    assert!(
        r.active_request_id().is_none(),
        "completed => no active request"
    );
    assert!(r.signal().is_some());
}

#[test]
fn invariant_after_complete_failure() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    r.complete_current_failure(rid, QueryError::response("fail"), 200);

    assert_eq!(r.status(), QueryStatus::Failure);
    assert!(
        r.data().is_none(),
        "Failure from LoadingEmpty => data must be None"
    );
    assert!(r.error().is_some(), "Failure => error must be Some");
    assert!(
        r.active_request_id().is_none(),
        "completed => no active request"
    );
}

#[test]
fn invariant_after_complete_failure_from_loading_with_data() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();

    let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    r.complete_current_success(rid1, "original", 200);

    let rid2 = begin_request_id(&mut r, &mut s, 300, QueryFetchMode::Normal);
    r.complete_current_failure(rid2, QueryError::transport("timeout"), 400);

    assert_eq!(r.status(), QueryStatus::Failure);
    assert!(r.error().is_some(), "Failure => error must be Some");
    assert!(r.active_request_id().is_none());
    assert_eq!(
        r.data(),
        Some(&"original"),
        "apply_failure retains data in-place"
    );
    assert!(
        r.previous_data().is_none(),
        "apply_failure does NOT set previous_data"
    );
}

#[test]
fn invariant_after_cancel_from_loading_empty() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let _ = r.begin_request(&mut s, 100, QueryFetchMode::Normal);

    let cancelled = r.cancel(QueryError::cancelled("abort"));
    assert!(
        cancelled,
        "cancel should return true when request is active"
    );
    assert_eq!(r.status(), QueryStatus::Cancelled);
    assert!(
        r.data().is_none(),
        "Cancelled from LoadingEmpty => data must be None"
    );
    assert!(r.error().is_some(), "Cancelled => error must be Some");
    assert!(
        r.active_request_id().is_none(),
        "cancelled => no active request"
    );
}

#[test]
fn invariant_after_cancel_from_loading_with_data() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();

    let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    r.complete_current_success(rid1, "data", 200);

    let _ = r.begin_request(&mut s, 300, QueryFetchMode::Normal);
    assert_eq!(r.status(), QueryStatus::LoadingWithData);
    assert_eq!(r.data(), Some(&"data"));

    let cancelled = r.cancel(QueryError::cancelled("abort"));
    assert!(cancelled);
    assert_eq!(r.status(), QueryStatus::Cancelled);
    assert!(
        r.data().is_none(),
        "cancel clears data (saved to previous_data)"
    );
    assert!(r.error().is_some());
    assert_eq!(
        r.previous_data(),
        Some(&"data"),
        "cancel saves data to previous_data for rollback"
    );
}

#[test]
fn invariant_cancel_returns_false_when_no_active_request() {
    let mut r = fresh_resource();
    assert!(!r.cancel(QueryError::cancelled("noop")));
    assert_eq!(r.status(), QueryStatus::Idle);
}

#[test]
fn invariant_after_reset() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    r.complete_current_success(rid, "data", 200);
    r.increment_retry();

    r.reset();

    assert_eq!(r.status(), QueryStatus::Idle);
    assert!(r.data().is_none(), "reset clears data");
    assert!(r.error().is_none(), "reset clears error");
    assert!(r.active_request_id().is_none());
    assert!(r.signal().is_none());
    assert!(r.previous_data().is_none());
    assert_eq!(r.cache_hits(), 0);
    assert_eq!(r.cancelled_count(), 0);
    assert_eq!(r.ignored_results(), 0);
    assert_eq!(r.retry_count(), 0);
    assert_eq!(r.cache_policy(), CachePolicy::NoCache);
    assert_eq!(r.request_policy(), RequestPolicy::LatestWins);
}

#[test]
fn invariant_complete_success_optional_none_yields_idle() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    let guard = r.accept_current_request(rid).unwrap();
    r.complete_success_optional(guard, None, 200);

    assert_eq!(
        r.status(),
        QueryStatus::Idle,
        "None data => Idle (not Success)"
    );
    assert!(r.data().is_none(), "Idle => data must be None");
    assert!(r.error().is_none());
}

#[test]
fn invariant_complete_success_optional_some_yields_success() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    let guard = r.accept_current_request(rid).unwrap();
    r.complete_success_optional(guard, Some("data"), 200);

    assert_eq!(r.status(), QueryStatus::Success);
    assert_eq!(r.data(), Some(&"data"));
}

#[test]
fn invariant_complete_failure_with_data() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    let guard = r.accept_current_request(rid).unwrap();
    r.complete_failure_with_data(guard, "fallback", QueryError::response("partial"), 200);

    assert_eq!(r.status(), QueryStatus::Failure);
    assert_eq!(
        r.data(),
        Some(&"fallback"),
        "Failure with data => data must be Some"
    );
    assert!(r.error().is_some(), "Failure => error must be Some");
}

#[test]
fn invariant_stale_accept_rejected() {
    let mut r = fresh_resource();
    let mut s = test_sequencer();
    let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    let rid2 = begin_request_id(&mut r, &mut s, 200, QueryFetchMode::Normal);
    assert!(
        r.accept_current_request(rid1).is_none(),
        "stale request should be rejected"
    );
    assert_eq!(r.ignored_results(), 1);

    assert!(
        r.accept_current_request(rid2).is_some(),
        "current request should be accepted"
    );
}

#[test]
fn table_driven_all_transitions_from_idle() {
    let mut r = fresh_resource();
    assert_eq!(r.status(), QueryStatus::Idle);

    let mut s = test_sequencer();
    let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    assert_eq!(r.status(), QueryStatus::LoadingEmpty);
    assert!(r.data().is_none());

    r.complete_current_success(rid, "data", 200);
    assert_eq!(r.status(), QueryStatus::Success);
    assert_eq!(r.data(), Some(&"data"));
    assert!(r.error().is_none());

    let rid2 = begin_request_id(&mut r, &mut s, 300, QueryFetchMode::Normal);
    assert_eq!(r.status(), QueryStatus::LoadingWithData);
    assert_eq!(r.data(), Some(&"data"), "LoadingWithData preserves data");

    r.complete_current_failure(rid2, QueryError::response("fail"), 400);
    assert_eq!(r.status(), QueryStatus::Failure);
    assert_eq!(
        r.data(),
        Some(&"data"),
        "apply_failure retains data in-place"
    );
    assert!(
        r.previous_data().is_none(),
        "apply_failure does NOT set previous_data"
    );

    let _rid3 = begin_request_id(&mut r, &mut s, 500, QueryFetchMode::Normal);
    assert_eq!(
        r.status(),
        QueryStatus::LoadingWithData,
        "data present => LoadingWithData even after Failure"
    );
    assert!(r.error().is_none(), "begin_request clears error");

    r.cancel(QueryError::cancelled("abort"));
    assert_eq!(r.status(), QueryStatus::Cancelled);
    assert!(r.data().is_none());
    assert!(r.error().is_some());

    r.reset();
    assert_eq!(r.status(), QueryStatus::Idle);
    assert!(r.data().is_none());
    assert!(r.error().is_none());
}

#[test]
fn table_driven_cancel_from_every_loading_state() {
    {
        let mut r = fresh_resource();
        let mut s = test_sequencer();
        let _ = r.begin_request(&mut s, 100, QueryFetchMode::Normal);
        assert_eq!(r.status(), QueryStatus::LoadingEmpty);
        r.cancel(QueryError::cancelled("abort"));
        assert_eq!(r.status(), QueryStatus::Cancelled);
        assert!(r.data().is_none());
        assert!(r.error().is_some());
    }

    {
        let mut r = fresh_resource();
        let mut s = test_sequencer();
        let rid = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
        r.complete_current_success(rid, "data", 200);
        let _ = r.begin_request(&mut s, 300, QueryFetchMode::Normal);
        assert_eq!(r.status(), QueryStatus::LoadingWithData);
        r.cancel(QueryError::cancelled("abort"));
        assert_eq!(r.status(), QueryStatus::Cancelled);
        assert!(
            r.data().is_none(),
            "cancel clears data (saves to previous_data)"
        );
        assert_eq!(r.previous_data(), Some(&"data"));
    }
}

#[test]
fn table_driven_rollback_from_every_state() {

    {
        let mut r = fresh_resource();
        let mut s = test_sequencer();
        let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
        r.complete_current_success(rid1, "v1", 200);
        let rid2 = begin_request_id(&mut r, &mut s, 300, QueryFetchMode::Normal);
        r.complete_current_success(rid2, "v2", 400);
        assert_eq!(r.previous_data(), Some(&"v1"));

        let rolled_back = r.rollback_to_previous();
        assert!(rolled_back);
        assert_eq!(r.status(), QueryStatus::Success, "rollback sets Success");
        assert_eq!(r.data(), Some(&"v1"), "rollback restores previous data");
    }

    {
        let mut r = fresh_resource();
        let mut s = test_sequencer();
        let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
        r.complete_current_success(rid1, "v1", 200);
        let _ = r.begin_request(&mut s, 300, QueryFetchMode::Normal);
        r.cancel(QueryError::cancelled("abort"));
        assert_eq!(r.previous_data(), Some(&"v1"));

        let rolled_back = r.rollback_to_previous();
        assert!(rolled_back);
        assert_eq!(r.status(), QueryStatus::Success);
        assert_eq!(r.data(), Some(&"v1"));
    }

    {
        let mut r = fresh_resource();
        let mut s = test_sequencer();
        let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
        r.complete_current_success(rid1, "v1", 200);
        r.set_data("v2_optimistic");
        assert_eq!(r.data(), Some(&"v2_optimistic"));
        assert_eq!(r.previous_data(), Some(&"v1"));

        let rolled_back = r.rollback_to_previous();
        assert!(rolled_back);
        assert_eq!(r.status(), QueryStatus::Success);
        assert_eq!(r.data(), Some(&"v1"));
    }

    {
        let mut r = fresh_resource();
        assert!(
            !r.rollback_to_previous(),
            "no previous_data => rollback fails"
        );
        assert_eq!(r.status(), QueryStatus::Idle);
    }
}
