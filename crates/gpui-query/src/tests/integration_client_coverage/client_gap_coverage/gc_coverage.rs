use gpui::{AppContext as _, BorrowAppContext as _, TestAppContext};

use crate::client::QueryClient;
use crate::core::*;
use crate::tests::test_support::*;

#[gpui::test]
fn test_gc_preserves_swr_resources_within_stale_window(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 500);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("swr_gc");
            let swr = CachePolicy::StaleWhileRevalidate {
                ttl_ms: 1_000,
                stale_ms: 5_000,
            };
            let entity = client.resource_with_policies::<String, QueryError>(
                key.clone(),
                swr,
                RequestPolicy::LatestWins,
                cx,
            );
            entity.update(cx, |r, _| r.apply_success("data".to_string(), 1_000));

            client.gc_with_time(3_000, cx);
            assert!(
                client.query::<String, QueryError>(&key).is_some(),
                "SWR resource within stale window must survive GC"
            );

            client.gc_with_time(8_000, cx);
            assert!(
                client.query::<String, QueryError>(&key).is_none(),
                "SWR resource past total valid window should be evicted"
            );
        });
    });
}

#[gpui::test]
fn test_gc_preserves_swr_resources_within_ttl(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 5_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("swr_fresh");
            let swr = CachePolicy::StaleWhileRevalidate {
                ttl_ms: 5_000,
                stale_ms: 3_000,
            };
            let entity = client.resource_with_policies::<String, QueryError>(
                key.clone(),
                swr,
                RequestPolicy::LatestWins,
                cx,
            );
            entity.update(cx, |r, _| r.apply_success("fresh".to_string(), 1_000));

            client.gc_with_time(3_000, cx);
            assert!(
                client.query::<String, QueryError>(&key).is_some(),
                "SWR resource within TTL must survive GC"
            );
        });
    });
}

#[gpui::test]
fn test_gc_preserves_success_mutation(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let entity = cx.new(|_| {
                MutationResource::<String, String, QueryError>::new(RetryPolicy::no_retries())
            });
            client.register_mutation::<String, String, QueryError>(&entity, cx);

            entity.update(cx, |m, _| {
                m.begin("vars".to_string());
                m.complete_success("done".to_string());
            });
            assert!(entity.read(cx).is_success());

            client.gc_with_time(1_000_000, cx);

            let mutations = client.all_mutations::<String, String, QueryError>();
            assert!(
                !mutations.is_empty(),
                "Success mutation should survive GC — only Idle/Failure are evictable"
            );
        });
    });
}

#[gpui::test]
fn test_gc_evicts_idle_infinite_query_with_realistic_timing(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("inf_gc_idle");
            let _entity = client.infinite_resource::<String, QueryError>(key.clone(), cx);

            client.gc_with_time(100_000, cx);

            assert!(
                client.infinite_query::<String, QueryError>(&key).is_none(),
                "idle infinite query should be evicted by GC"
            );
        });
    });
}

#[gpui::test]
fn test_gc_preserves_loading_infinite_query(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("inf_gc_loading");
            let entity = client.infinite_resource::<String, QueryError>(key.clone(), cx);

            let _rid = client
                .next_request_id_for_infinite_key::<String, QueryError>(&key)
                .expect("request id");
            let mut seq = RequestSequencer::new();
            entity.update(cx, |r, _| {
                r.begin_fetch_next(&mut seq, 1_000);
            });
            assert!(entity.read(cx).status().is_loading());

            client.gc_with_time(1_000_000, cx);

            assert!(
                client.infinite_query::<String, QueryError>(&key).is_some(),
                "loading infinite query must survive GC regardless of age"
            );
        });
    });
}

#[gpui::test]
fn test_gc_evicts_aged_successful_infinite_query(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let key = QueryKey::from("inf_gc_success");
            let entity = client.infinite_resource::<String, QueryError>(key.clone(), cx);

            entity.update(cx, |r, _| {
                let mut seq = RequestSequencer::new();
                let id = r.begin_fetch_next(&mut seq, 1_000).expect("begin fetch");
                r.complete_page_success(id, "page0".to_string(), false, true, 1_000);
            });
            assert_eq!(entity.read(cx).status(), QueryStatus::Success);

            client.gc_with_time(3_500, cx);
            assert!(
                client.infinite_query::<String, QueryError>(&key).is_none(),
                "successful infinite query aged past success_threshold must be evicted (#1/#133)"
            );
        });
    });
}

#[gpui::test]
fn test_bucket_default_max_entries_allows_many_resources(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            for i in 0..100 {
                let key = format!("max_{}", i);
                let _entity = client.resource::<String, QueryError>(key, cx);
            }

            let all = client.all_queries::<String, QueryError>();
            assert_eq!(
                all.len(),
                100,
                "all 100 resources should exist within default max_entries(10_000)"
            );
        });
    });
}

#[gpui::test]
fn test_loading_mutation_survives_gc(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1_000);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let entity = cx.new(|_| {
                MutationResource::<String, String, QueryError>::new(RetryPolicy::no_retries())
            });
            client.register_mutation::<String, String, QueryError>(&entity, cx);

            entity.update(cx, |m, _| {
                m.begin("vars".to_string());
            });
            assert!(entity.read(cx).is_loading());

            client.gc_with_time(1_000_000, cx);

            let mutations = client.all_mutations::<String, String, QueryError>();
            assert_eq!(
                mutations.len(),
                1,
                "loading mutation must survive GC regardless of age"
            );
        });
    });
}

#[gpui::test]
fn test_idle_mutation_is_evicted_by_gc_after_age_exceeds_threshold(cx: &mut TestAppContext) {
    setup_query_client_with_gc(cx, 1);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let entity = cx.new(|_| {
                MutationResource::<String, String, QueryError>::new(RetryPolicy::no_retries())
            });
            client.register_mutation::<String, String, QueryError>(&entity, cx);

            assert!(
                entity.read(cx).is_idle(),
                "mutation should be idle before any operation"
            );
            assert_eq!(
                client.all_mutations::<String, String, QueryError>().len(),
                1,
                "mutation should exist before GC"
            );

            let far_future = crate::client::current_time_ms() + 3_600_000;
            client.gc_with_time(far_future, cx);

            assert_eq!(
                client.all_mutations::<String, String, QueryError>().len(),
                0,
                "idle mutation should be evicted when age exceeds gc_threshold"
            );
        });
    });
}

#[gpui::test]
fn test_observer_status_dedup_default_config_is_status_change_only(_cx: &mut TestAppContext) {
    let config = crate::client::ObserverConfig::default();
    assert!(
        config.notify_on_status_change_only,
        "default ObserverConfig should notify on status change only"
    );

    let always_notify = crate::client::ObserverConfig {
        notify_on_status_change_only: false,
    };
    assert!(
        !always_notify.notify_on_status_change_only,
        "explicit always-notify config should be false"
    );
}

#[gpui::test]
fn test_mutation_bucket_evict_oldest_keeps_count_bounded(cx: &mut TestAppContext) {
    const MAX_ENTRIES: usize = 10_000;

    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let mut live: Vec<gpui::Entity<MutationResource<String, String, QueryError>>> =
                Vec::with_capacity(MAX_ENTRIES + 2);

            for _ in 0..(MAX_ENTRIES + 2) {
                let entity = cx.new(|_| {
                    MutationResource::<String, String, QueryError>::new(RetryPolicy::no_retries())
                });
                client.register_mutation::<String, String, QueryError>(&entity, cx);
                live.push(entity);
            }

            let mutations = client.all_mutations::<String, String, QueryError>();
            assert_eq!(
                mutations.len(),
                MAX_ENTRIES,
                "MutationBucket entry count must stay bounded at DEFAULT_MAX_ENTRIES \
                 ({}); evict_oldest should have triggered on every insert past the cap",
                MAX_ENTRIES
            );

            let diag = client.diagnostics(cx);
            assert_eq!(
                diag.mutation_count, MAX_ENTRIES,
                "diagnostics.mutation_count must match the bounded bucket size"
            );

            drop(live);
        });
    });
}
