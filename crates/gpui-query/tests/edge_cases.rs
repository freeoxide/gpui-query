use gpui_query::core::{
    CachePolicy, InfiniteQueryResource, QueryBeginResult, QueryError, QueryKey, QueryResource,
    RequestSequencer,
};

fn make_resource(cache: CachePolicy) -> QueryResource<String, QueryError> {
    QueryResource::new(
        QueryKey::from("test"),
        cache,
        gpui_query::core::RequestPolicy::LatestWins,
    )
}

fn begin_request_with_seq(
    resource: &mut QueryResource<String, QueryError>,
    seq: &mut RequestSequencer,
    now_ms: u64,
) -> QueryBeginResult {
    resource.begin_request(seq, now_ms, gpui_query::core::QueryFetchMode::Normal)
}

fn complete_success(
    resource: &mut QueryResource<String, QueryError>,
    request_id: gpui_query::core::RequestId,
    data: &str,
    now_ms: u64,
) -> bool {
    resource.complete_current_success(request_id, data.to_string(), now_ms)
}

#[test]
fn query_key_empty_vec_panics() {
    let result = std::panic::catch_unwind(|| {
        let _ = QueryKey::new(Vec::<String>::new());
    });
    assert!(result.is_err());
}

#[test]
fn ttl_zero_every_begin_request_is_a_fetch() {
    let mut resource = make_resource(CachePolicy::Ttl { ttl_ms: 0 });
    let mut seq = RequestSequencer::new();

    let result = begin_request_with_seq(&mut resource, &mut seq, 1_000);
    assert!(matches!(result, QueryBeginResult::Started { .. }));

    if let QueryBeginResult::Started { request_id, .. } = result {
        assert!(complete_success(&mut resource, request_id, "data", 1_000));
    }

    let result_same = begin_request_with_seq(&mut resource, &mut seq, 1_000);
    assert!(matches!(result_same, QueryBeginResult::CacheHit));

    let result2 = begin_request_with_seq(&mut resource, &mut seq, 1_001);
    assert!(matches!(result2, QueryBeginResult::Started { .. }));
}

#[test]
fn ttl_max_data_is_always_fresh() {
    let mut resource = make_resource(CachePolicy::Ttl { ttl_ms: u64::MAX });
    let mut seq = RequestSequencer::new();

    let result = begin_request_with_seq(&mut resource, &mut seq, 1_000);
    if let QueryBeginResult::Started { request_id, .. } = result {
        assert!(complete_success(&mut resource, request_id, "data", 2_000));
    }

    let result2 = begin_request_with_seq(&mut resource, &mut seq, u64::MAX);
    assert!(matches!(result2, QueryBeginResult::CacheHit));
}

#[test]
fn swr_total_valid_ms_saturates_on_overflow() {
    let policy = CachePolicy::StaleWhileRevalidate {
        ttl_ms: u64::MAX,
        stale_ms: u64::MAX,
    };
    assert_eq!(policy.total_valid_ms(), Some(u64::MAX));
}

#[test]
fn three_concurrent_requests_latest_wins_only_last_completes() {
    let mut resource = make_resource(CachePolicy::Ttl { ttl_ms: 60_000 });
    let mut seq = RequestSequencer::new();

    let r1 = begin_request_with_seq(&mut resource, &mut seq, 1_000);
    let r2 = begin_request_with_seq(&mut resource, &mut seq, 2_000);
    let r3 = begin_request_with_seq(&mut resource, &mut seq, 3_000);

    if let QueryBeginResult::Started {
        request_id: id1, ..
    } = r1
        && let QueryBeginResult::Started {
            request_id: id2, ..
        } = r2
        && let QueryBeginResult::Started {
            request_id: id3, ..
        } = r3
    {
        assert!(!complete_success(&mut resource, id1, "old1", 4_000));
        assert!(!complete_success(&mut resource, id2, "old2", 4_000));
        assert!(complete_success(&mut resource, id3, "newest", 4_000));
        assert_eq!(resource.data(), Some(&"newest".to_string()));
        assert_eq!(resource.cancelled_count(), 2);
        assert_eq!(resource.ignored_results(), 2);
    }
}

#[test]
fn resource_serde_roundtrip_preserves_state() {
    let mut resource = make_resource(CachePolicy::Ttl { ttl_ms: 5_000 });
    let mut seq = RequestSequencer::new();

    let r = begin_request_with_seq(&mut resource, &mut seq, 1_000);
    if let QueryBeginResult::Started { request_id, .. } = r {
        assert!(complete_success(&mut resource, request_id, "hello", 2_000));
    }

    let json = serde_json::to_string(&resource).unwrap();
    let back: QueryResource<String, QueryError> = serde_json::from_str(&json).unwrap();

    assert_eq!(back.status(), gpui_query::core::QueryStatus::Success);
    assert_eq!(back.data(), Some(&"hello".to_string()));
    assert_eq!(back.cache_policy(), CachePolicy::Ttl { ttl_ms: 5_000 });
    assert_eq!(back.key(), &QueryKey::from("test"));
    assert!(back.signal().is_none());
}

#[test]
fn infinite_query_serde_roundtrip_preserves_pages_and_config() {
    let mut resource: InfiniteQueryResource<Vec<String>, QueryError> = InfiniteQueryResource::new(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        gpui_query::core::RequestPolicy::LatestWins,
    );
    let mut seq = RequestSequencer::new();
    resource.set_max_pages(Some(10));

    let id1 = resource.begin_fetch_next(&mut seq, 100).unwrap();
    resource.complete_page_success(id1, vec!["a".to_string()], true, true, 200);
    let id2 = resource.begin_fetch_next(&mut seq, 300).unwrap();
    resource.complete_page_success(id2, vec!["b".to_string()], false, true, 400);

    let json = serde_json::to_string(&resource).unwrap();
    let back: InfiniteQueryResource<Vec<String>, QueryError> = serde_json::from_str(&json).unwrap();

    assert_eq!(back.page_count(), 2);
    assert_eq!(back.max_pages(), Some(10));
    assert!(!back.has_next_page());
    assert!(back.signal().is_none());
}
