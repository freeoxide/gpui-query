use crate::client::QueryClient;
use crate::core::*;
use crate::tests::test_support::*;
use gpui::{BorrowAppContext as _, TestAppContext};

fn create_success_at_time(
    client: &mut QueryClient,
    cx: &mut gpui::App,
    key: &str,
    data: &str,
    success_time_ms: u64,
) {
    let entity = client.resource_with_policies::<String, QueryError>(
        QueryKey::from(key),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
        cx,
    );
    entity.update(cx, |r, _| {
        r.apply_success(data.to_string(), success_time_ms)
    });
}

#[gpui::test]
fn test_gc_evicts_exactly_expired_resources(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            create_success_at_time(client, cx, "young", "young_data", 2_000);
            create_success_at_time(client, cx, "middle", "middle_data", 1_000);
            create_success_at_time(client, cx, "old", "old_data", 100);

            assert_eq!(client.all_queries::<String, QueryError>().len(), 3);

            client.gc_with_time(2_500, cx);

            assert_eq!(
                client.all_queries::<String, QueryError>().len(),
                2,
                "exactly 1 of 3 resources should be evicted"
            );
            assert!(
                client
                    .query::<String, QueryError>(&QueryKey::from("young"))
                    .is_some(),
                "young (age 500ms) should survive"
            );
            assert!(
                client
                    .query::<String, QueryError>(&QueryKey::from("middle"))
                    .is_some(),
                "middle (age 1500ms) should survive"
            );
            assert!(
                client
                    .query::<String, QueryError>(&QueryKey::from("old"))
                    .is_none(),
                "old (age 2400ms > success_threshold 2000ms) should be evicted"
            );
        });
    });
}

#[gpui::test]
fn test_gc_eviction_counts_match(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            for i in 0..5 {
                let _ = client.resource::<String, QueryError>(format!("idle_{}", i), cx);
            }
            assert_eq!(client.all_queries::<String, QueryError>().len(), 5);

            client.gc_with_time(5_000, cx);

            assert_eq!(
                client.all_queries::<String, QueryError>().len(),
                0,
                "all 5 idle resources with no snapshot should be evicted"
            );
        });
    });
}

#[gpui::test]
fn test_gc_preserves_loading_resource_with_snapshot(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("loading_preserved");
            let prepared = client
                .prepare_fetch_query::<String, QueryError>(key.clone(), cx)
                .expect("should start");

            client.gc_with_time(1_000_000, cx);

            let entity = client
                .query::<String, QueryError>(&key)
                .expect("loading resource must survive GC");

            prepared.complete_success("data".to_string(), cx);
            assert_eq!(
                entity.read(cx).data(),
                Some(&"data".to_string()),
                "entity should be usable after surviving GC"
            );
        });
    });
}

#[gpui::test]
fn test_gc_mixed_states_precise_eviction(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let prepared = client
                .prepare_fetch_query::<String, QueryError>("loading", cx)
                .expect("should start");

            create_success_at_time(client, cx, "success_fresh", "data", 1_000);

            create_success_at_time(client, cx, "success_old", "data", 0);

            assert_eq!(client.all_queries::<String, QueryError>().len(), 3);

            client.gc_with_time(2_500, cx);

            let remaining = client.all_queries::<String, QueryError>();
            assert_eq!(
                remaining.len(),
                2,
                "exactly 1 of 3 resources should be evicted"
            );

            let remaining_keys: Vec<String> = remaining
                .iter()
                .map(|e| e.read(cx).key().to_path())
                .collect();
            assert!(
                remaining_keys.contains(&"loading".to_string()),
                "loading should survive: {:?}",
                remaining_keys
            );
            assert!(
                remaining_keys.contains(&"success_fresh".to_string()),
                "success_fresh should survive: {:?}",
                remaining_keys
            );

            prepared.complete_success("data".to_string(), cx);
        });
    });
}

#[gpui::test]
fn test_gc_survive_then_evict_after_threshold_crossed(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("aged");
            create_success_at_time(client, cx, "aged", "data", 1_000);

            client.gc_with_time(2_000, cx);
            assert!(
                client.query::<String, QueryError>(&key).is_some(),
                "age=1000ms < success_threshold=2000ms => should survive"
            );

            client.gc_with_time(3_500, cx);
            assert!(
                client.query::<String, QueryError>(&key).is_none(),
                "age=2500ms > success_threshold=2000ms => should be evicted"
            );
        });
    });
}

#[gpui::test]
fn test_gc_boundary_success_threshold_exact(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("boundary");
            create_success_at_time(client, cx, "boundary", "data", 1_000);

            client.gc_with_time(3_000, cx);
            assert!(
                client.query::<String, QueryError>(&key).is_none(),
                "age=2000ms == success_threshold=2000ms => must be evicted (>= boundary)"
            );
        });
    });
}
