use crate::core::*;
use crate::tests::test_support::*;
use std::num::NonZero;

#[test]
fn begin_request_with_id_swr_ignore_while_loading_with_active_request() {
    let mut r: QueryResource<&str> = QueryResource::new(
        "swr-ignore",
        CachePolicy::StaleWhileRevalidate {
            ttl_ms: 500,
            stale_ms: 1_000,
        },
        RequestPolicy::IgnoreWhileLoading,
    );
    let mut seq = test_sequencer();

    r.apply_success("cached", 100);

    let _ = r.begin_request(&mut seq, 1_500, QueryFetchMode::Force);
    assert!(r.is_loading());

    let result = r.begin_request_with_id(
        Some(RequestId::scoped(NonZero::new(99).unwrap(), 1)),
        1_500,
        QueryFetchMode::Normal,
    );

    match result {
        QueryBeginResult::StaleCacheHit {
            request_id,
            replaced_request_id,
            ..
        } => {
            assert!(
                replaced_request_id.is_none(),
                "no replacement under IgnoreWhileLoading"
            );
            assert_ne!(
                request_id,
                RequestId::scoped(NonZero::new(99).unwrap(), 1),
                "should use existing active request id"
            );
        }
        other => panic!("expected StaleCacheHit, got {:?}", other),
    }
}

#[test]
fn complete_current_optional_success_rejects_stale_id() {
    let mut r = test_resource();
    let mut s = test_sequencer();

    let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    let rid2 = begin_request_id(&mut r, &mut s, 200, QueryFetchMode::Normal);

    assert!(
        !r.complete_current_optional_success(rid1, Some("stale"), 300),
        "stale ID should be rejected"
    );
    assert_eq!(r.ignored_results(), 1);

    assert!(
        r.complete_current_optional_success(rid2, Some("fresh"), 300),
        "current ID should be accepted"
    );
    assert_eq!(r.data(), Some(&"fresh"));
}

#[test]
fn complete_current_failure_with_data_rejects_stale_id() {
    let mut r = test_resource();
    let mut s = test_sequencer();

    let rid1 = begin_request_id(&mut r, &mut s, 100, QueryFetchMode::Normal);
    let _rid2 = begin_request_id(&mut r, &mut s, 200, QueryFetchMode::Normal);

    assert!(
        !r.complete_current_failure_with_data(rid1, "fallback", QueryError::response("stale"), 300),
        "stale ID should be rejected"
    );
    assert_eq!(r.ignored_results(), 1);

    assert!(r.active_request_id().is_some());
}

#[test]
fn ignore_while_loading_rejects_forced_fetch_when_loading() {
    let mut r: QueryResource<&str> = QueryResource::new(
        "test",
        CachePolicy::NoCache,
        RequestPolicy::IgnoreWhileLoading,
    );
    let mut s = test_sequencer();

    let _ = r.begin_request(&mut s, 100, QueryFetchMode::Normal);

    let result = r.begin_request(&mut s, 200, QueryFetchMode::Force);
    assert!(
        matches!(result, QueryBeginResult::IgnoredWhileLoading { .. }),
        "Force mode should still respect IgnoreWhileLoading"
    );
}

#[test]
fn query_error_sanitized_mongodb_connection() {
    let err = QueryError::transport("connect mongodb://admin:secret@host/db failed");
    let clean = err.sanitized();
    assert!(
        clean.message().contains("[REDACTED_CONNECTION]"),
        "mongodb connection string should be redacted"
    );
    assert!(
        !clean.message().contains("admin:secret"),
        "credentials should be removed"
    );
}

#[test]
fn query_error_sanitized_empty_message() {
    let err = QueryError::response("");
    let clean = err.sanitized();
    assert_eq!(clean.message(), "");
}

#[test]
fn query_error_new_with_explicit_kind() {
    let err = QueryError::new(QueryErrorKind::Transport, "timeout");
    assert_eq!(err.kind(), QueryErrorKind::Transport);
    assert_eq!(err.message(), "timeout");
}

#[test]
fn record_cache_hit_does_not_clear_cancelled_status() {
    let mut r: QueryResource<&str> = QueryResource::new(
        "cache-cancel",
        CachePolicy::Ttl { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    );
    r.apply_success("data", 1_000);

    let mut seq = test_sequencer();
    let _ = r.begin_request(&mut seq, 1_100, QueryFetchMode::Force);
    r.cancel(QueryError::cancelled("abort"));
    assert_eq!(r.status(), QueryStatus::Cancelled);

    r.record_cache_hit();
    assert_eq!(
        r.status(),
        QueryStatus::Cancelled,
        "cache hit should not clear Cancelled status"
    );
    assert_eq!(r.cache_hits(), 1);
}

#[test]
fn join_appends_segment() {
    let key = QueryKey::from(["users"]);
    let extended = key.join("42");
    assert_eq!(extended.parts().len(), 2);
    assert_eq!(extended.to_path(), "users::42");
    assert_eq!(key.parts().len(), 1);
}

#[test]
fn join_chain_creates_multi_part_key() {
    let key = QueryKey::from("users").join("42").join("posts");
    assert_eq!(key.parts().len(), 3);
    assert_eq!(key.to_path(), "users::42::posts");
}

#[test]
fn from_vec_string() {
    let key = QueryKey::from(vec!["users".to_string(), "42".to_string()]);
    assert_eq!(key.parts().len(), 2);
    assert_eq!(key.to_path(), "users::42");
}

#[test]
fn deref_allows_indexing() {
    let key = QueryKey::from(["a", "b", "c"]);
    assert_eq!(&*key[0], "a");
    assert_eq!(&*key[2], "c");
    assert_eq!(key.len(), 3);
}

#[test]
fn serde_deserialize_single_string() {
    let json = "\"users\"";
    let key: QueryKey = serde_json::from_str(json).unwrap();
    assert_eq!(key.parts().len(), 1);
    assert_eq!(key.first_segment(), "users");
}

#[test]
fn hash_consistency() {
    use std::collections::HashSet;
    let k1 = QueryKey::from(["users", "42"]);
    let k2 = QueryKey::from(["users", "42"]);
    let k3 = QueryKey::from(["users", "43"]);
    let mut set = HashSet::new();
    set.insert(k1.clone());
    assert!(set.contains(&k2), "equal keys must have equal hashes");
    assert!(!set.contains(&k3), "different keys should not match");
}

#[test]
fn ignore_while_loading_prevents_previous_page_replacement() {
    let mut r = InfiniteQueryResource::<Vec<String>>::new(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::IgnoreWhileLoading,
    );
    let mut seq = RequestSequencer::new();
    r.set_has_previous_page(true);

    let _id1 = r.begin_fetch_previous(&mut seq, 1_000).unwrap();
    assert!(r.is_fetching_previous_page());

    let id2 = r.begin_fetch_previous(&mut seq, 2_000);
    assert!(
        id2.is_none(),
        "second begin_fetch_previous should be ignored"
    );
    assert_eq!(r.cancelled_count(), 0, "no cancellation on ignore");
}

#[test]
fn ignore_while_loading_cross_direction_next_then_prev() {
    let mut r = InfiniteQueryResource::<Vec<String>>::new_bidirectional(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::IgnoreWhileLoading,
    );
    let mut seq = RequestSequencer::new();
    r.set_has_next_page(true);
    r.set_has_previous_page(true);

    let _id_next = r.begin_fetch_next(&mut seq, 1_000).unwrap();
    assert!(r.is_fetching_next_page());

    let id_prev = r.begin_fetch_previous(&mut seq, 2_000);
    assert!(
        id_prev.is_some(),
        "cross-direction should succeed under IgnoreWhileLoading"
    );
    assert!(r.is_fetching_previous_page());
    assert!(!r.is_fetching_next_page());
}

#[test]
fn infinite_query_reset_preserves_retry_policy() {
    let mut r = InfiniteQueryResource::<Vec<String>>::new(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    );
    let policy = RetryPolicy::new(10)
        .with_delay(500)
        .with_exponential_backoff();
    r.set_retry_policy(policy.clone());
    r.reset();
    assert_eq!(
        r.retry_policy(),
        &policy,
        "retry_policy should survive reset"
    );
}

#[test]
fn bidirectional_resource_initial_accessors() {
    let r = InfiniteQueryResource::<Vec<String>>::new_bidirectional(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    );
    assert_eq!(r.cache_policy(), CachePolicy::Ttl { ttl_ms: 60_000 });
    assert_eq!(r.request_policy(), RequestPolicy::LatestWins);
    assert_eq!(r.direction(), FetchDirection::Bidirectional);
    assert!(!r.has_next_page());
    assert!(!r.has_previous_page());
}

#[test]
fn prepend_with_has_more_true_preserves_has_previous() {
    let mut r = InfiniteQueryResource::<Vec<String>>::new(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    );
    let mut seq = RequestSequencer::new();

    let id1 = r.begin_fetch_next(&mut seq, 1_000).unwrap();
    r.complete_page_success(id1, vec!["page1".to_string()], true, true, 2_000);

    r.set_has_previous_page(true);

    let id2 = r.begin_fetch_previous(&mut seq, 3_000).unwrap();
    r.complete_page_success(id2, vec!["page0".to_string()], true, false, 4_000);

    assert!(
        r.has_previous_page(),
        "has_more=true should keep has_previous_page=true"
    );
    assert_eq!(r.page_count(), 2);
    assert_eq!(r.first_page(), Some(&vec!["page0".to_string()]));
}
