//! Integration tests for the value-carrying persistence layer:
//! `persist_with` debounce/coalescing, the serializer/deserializer registries,
//! `hydrate` round-trip, the `CacheMutation` dirty signal, and
//! `PersistFilter`/`max_age` behavior.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use gpui::{AppContext as _, BorrowAppContext as _, Entity, TestAppContext};

use crate::client::{
    CacheMutation, NoopPersister, PersistError, PersistFilter, PersistHandle, PersistOptions,
    PersistSnapshot, PersistedEntry, Persister, QueryClient, hydrate,
};
use crate::core::{
    InfiniteQueryResource, MutationResource, QueryError, QueryKey, QueryResource, QueryStatus,
};
use crate::hook::{
    InfiniteQueryOptions, fetch_query, mutate, use_infinite_query, use_mutation, use_query_manual,
};
use crate::tests::test_support::*;

/// In-memory persister for asserting on saved payloads. `save_count` counts
/// `save` calls so coalescing tests can assert exactly how many fired.
#[derive(Default, Clone)]
struct MemPersister {
    last_saved: Arc<StdMutex<Option<PersistSnapshot>>>,
    load_value: Arc<StdMutex<Option<PersistSnapshot>>>,
    save_count: Arc<StdMutex<u32>>,
}

impl Persister for MemPersister {
    async fn load(&self) -> Result<PersistSnapshot, PersistError> {
        Ok(self
            .load_value
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| PersistSnapshot {
                entries: Default::default(),
                version: crate::client::PERSIST_VERSION,
            }))
    }

    async fn save(&self, snapshot: &PersistSnapshot) -> Result<(), PersistError> {
        *self.save_count.lock().unwrap() += 1;
        *self.last_saved.lock().unwrap() = Some(snapshot.clone());
        Ok(())
    }
}

/// Serializer for the `String`-typed fixtures.
fn ser_string(s: &String) -> serde_json::Value {
    serde_json::to_value(s).expect("serialize")
}

/// Debounce disabled: the TestAppContext mock clock never advances wall-clock
/// timers on its own, so a non-zero debounce would leave the save un-fired
/// unless the test calls `advance_clock`.
fn zero_debounce() -> PersistOptions {
    PersistOptions {
        debounce: Duration::ZERO,
        ..PersistOptions::default()
    }
}

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[gpui::test]
fn test_set_query_data_bumps_cache_mutation(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        // The observation must be live when the bump fires, and must not panic.
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            client.persist_with(NoopPersister, PersistOptions::default(), cx)
        });
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.set_query_data::<String, QueryError>("k1", "v1".to_string(), cx);
        });
        assert!(cx.has_global::<CacheMutation>());
    });
}

#[gpui::test]
fn test_collect_persist_snapshot_uses_registered_serializer(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);

            let e = client.resource::<String, QueryError>(QueryKey::from("snap_k"), cx);
            e.update(cx, |r, _| {
                r.apply_success("payload".to_string(), crate::client::current_time_ms())
            });

            let snap = client.collect_persist_snapshot(&PersistFilter::All, DAY, cx);
            assert_eq!(snap.entries.len(), 1);
            let entry = snap.entries.get("snap_k").expect("entry present");
            assert_eq!(entry.value, serde_json::json!("payload"));
        });
    });
}

#[gpui::test]
fn test_collect_persist_snapshot_skips_unregistered_types(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            // No serializer registered: the entry is skipped entirely.
            let e = client.resource::<String, QueryError>(QueryKey::from("unreg"), cx);
            e.update(cx, |r, _| {
                r.apply_success("data".to_string(), crate::client::current_time_ms())
            });

            let snap = client.collect_persist_snapshot(&PersistFilter::All, Duration::from_secs(3600), cx);
            assert!(
                snap.entries.is_empty(),
                "unregistered type -> no value-carrying entry"
            );
        });
    });
}

#[gpui::test]
fn test_collect_persist_snapshot_filter_and_max_age(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);
            let now = crate::client::current_time_ms();
            // Two recent entries under the "users" prefix.
            for parts in [["users", "1"], ["users", "2"]] {
                let e = client.resource::<String, QueryError>(QueryKey::from(parts), cx);
                e.update(cx, |r, _| r.apply_success("v".to_string(), now));
            }
            // One entry ~2.8 h in the past under "posts".
            let e = client.resource::<String, QueryError>(QueryKey::from(["posts", "9"]), cx);
            e.update(cx, |r, _| {
                r.apply_success("old".to_string(), now.saturating_sub(10_000_000))
            });

            let snap = client.collect_persist_snapshot(
                &PersistFilter::Prefix(QueryKey::from(["users"])),
                Duration::from_secs(3600),
                cx,
            );
            assert_eq!(snap.entries.len(), 2);

            // max_age = 0 means "disabled", not "everything is too old", so
            // a small positive value is what filters the old entry out here.
            let snap_all =
                client.collect_persist_snapshot(&PersistFilter::All, Duration::from_secs(1), cx);
            assert_eq!(snap_all.entries.len(), 2);
            assert!(!snap_all.entries.contains_key("posts::9"));
        });
    });
}

#[gpui::test]
fn test_persist_with_saves_on_mutation(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();

    // The bucket stores only WeakEntity, so a live owner must hold the
    // Success entry or it dies before the observer collects the snapshot.
    struct H {
        _entity: Entity<QueryResource<String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let (_handle, entity) = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);
            let handle = client.persist_with(persister.clone(), zero_debounce(), cx);
            let entity = client.resource::<String, QueryError>(QueryKey::from("persisted"), cx);
            entity.update(cx, |r, _| {
                r.apply_success("data".to_string(), crate::client::current_time_ms())
            });
            (handle, entity)
        });
        H {
            _entity: entity,
            _handle,
        }
    });

    // A mutation on another key bumps the dirty signal; persist_with collects
    // the live cache (including the retained Success entry) and saves it.
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.set_query_data::<String, QueryError>("trigger", "x".to_string(), cx);
        });
    });
    cx.run_until_parked();

    let saved = captured
        .lock()
        .unwrap()
        .clone()
        .expect("persist_with should have saved after the mutation");
    assert!(
        saved.entries.contains_key("persisted"),
        "saved snapshot should include the retained Success entry: {:?}",
        saved.entries.keys().collect::<Vec<_>>()
    );
    let _ = harness;
}

#[gpui::test]
fn test_hydrate_primes_via_deserializer_registry(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();

    let mut snap = PersistSnapshot {
        entries: Default::default(),
        version: crate::client::PERSIST_VERSION,
    };
    snap.entries.insert(
        "hydrate_k".to_string(),
        PersistedEntry {
            value: serde_json::json!("hydrated-value"),
            cached_at: crate::client::current_time_ms(),
            cache_policy: crate::core::CachePolicy::default(),
            meta: None,
        },
    );
    *persister.load_value.lock().unwrap() = Some(snap);

    // Hold the "hydrate_k" entity alive so hydrate's set_query_data reuses it
    // instead of creating an entity that is dropped immediately (WeakEntity).
    struct H {
        _entity: Entity<QueryResource<String, QueryError>>,
    }
    let harness = cx.new(|cx| {
        cx.update_global::<QueryClient, _>(|client, _cx| {
            client
                .register_deserializer::<String, QueryError>(|v| v.as_str().map(|s| s.to_string()));
        });
        let entity = cx.update_global::<QueryClient, _>(|client, cx| {
            client.resource::<String, QueryError>(QueryKey::from("hydrate_k"), cx)
        });
        H { _entity: entity }
    });

    let filter = PersistFilter::All;
    let max_age = DAY;
    let outcome = cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            // The MemPersister load resolves immediately, so the hydrate
            // future is Ready on first poll and can be driven synchronously.
            block_on_ready(hydrate(client, &persister, &filter, max_age, cx))
        })
    });

    assert!(
        outcome.is_ok(),
        "hydrate should succeed: {:?}",
        outcome.err()
    );

    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let data =
                client.get_query_data::<String, QueryError>(&QueryKey::from("hydrate_k"), cx);
            assert_eq!(
                data,
                Some("hydrated-value".to_string()),
                "hydrate should have primed the value via the deserializer registry"
            );
        });
    });
    let _ = harness;
}

// A stored multi-segment path must hydrate as a segmented key; priming it
// as one flat segment would hide it from Exact/Prefix filters.

#[gpui::test]
fn test_hydrate_rebuilds_multi_segment_keys(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();

    let mut snap = PersistSnapshot {
        entries: Default::default(),
        version: crate::client::PERSIST_VERSION,
    };
    snap.entries.insert(
        "users::42::posts".to_string(),
        PersistedEntry {
            value: serde_json::json!("post-data"),
            cached_at: crate::client::current_time_ms(),
            cache_policy: crate::core::CachePolicy::default(),
            meta: None,
        },
    );
    *persister.load_value.lock().unwrap() = Some(snap);

    // Retain the multi-segment entity so hydrate's set_query_data reuses it
    // instead of creating one that dies immediately (WeakEntity).
    struct H {
        _entity: Entity<QueryResource<String, QueryError>>,
    }
    let harness = cx.new(|cx| {
        cx.update_global::<QueryClient, _>(|client, _cx| {
            client
                .register_deserializer::<String, QueryError>(|v| v.as_str().map(|s| s.to_string()));
        });
        let entity = cx.update_global::<QueryClient, _>(|client, cx| {
            client.resource::<String, QueryError>(QueryKey::from(["users", "42", "posts"]), cx)
        });
        H { _entity: entity }
    });

    let key = QueryKey::from(["users", "42", "posts"]);
    let prefix = PersistFilter::Prefix(QueryKey::from(["users"]));
    let outcome = cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            block_on_ready(hydrate(client, &persister, &prefix, DAY, cx))
        })
    });
    assert!(outcome.is_ok(), "hydrate should succeed: {:?}", outcome.err());

    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let data = client.get_query_data::<String, QueryError>(&key, cx);
            assert_eq!(
                data,
                Some("post-data".to_string()),
                "Prefix must match the reconstructed segments"
            );
        });
    });

    // Overwrite with a sentinel so the Exact pass has to re-prime to pass.
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.set_query_data::<String, QueryError>(key.clone(), "sentinel".to_string(), cx);
        });
    });
    let exact = PersistFilter::Exact(key.clone());
    let outcome = cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            block_on_ready(hydrate(client, &persister, &exact, DAY, cx))
        })
    });
    assert!(outcome.is_ok(), "hydrate should succeed: {:?}", outcome.err());

    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let data = client.get_query_data::<String, QueryError>(&key, cx);
            assert_eq!(
                data,
                Some("post-data".to_string()),
                "Exact must match the reconstructed segments"
            );
        });
    });
    let _ = harness;
}

#[gpui::test]
fn test_hydrate_rejects_version_mismatch(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    *persister.load_value.lock().unwrap() = Some(PersistSnapshot {
        entries: Default::default(),
        version: 9999,
    });

    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, _cx| {
            client
                .register_deserializer::<String, QueryError>(|v| v.as_str().map(|s| s.to_string()));
        });
    });

    let filter = PersistFilter::All;
    let max_age = DAY;
    let outcome = cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            block_on_ready(hydrate(client, &persister, &filter, max_age, cx))
        })
    });

    match outcome {
        Err(PersistError::VersionMismatch { expected, found }) => {
            assert_eq!(expected, crate::client::PERSIST_VERSION);
            assert_eq!(found, 9999);
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

// A real fetch completion (not set_query_data) drives persist_with: the
// success arm of the hook retry loop bumps CacheMutation, which the
// persist_with observer collects and saves.

#[gpui::test]
fn test_persist_with_driven_by_real_fetch_completion(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();

    struct H {
        entity: Entity<QueryResource<String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);
            client.persist_with(persister.clone(), zero_debounce(), cx)
        });
        // The real hook path keys the bucket under "fetched";
        // use_query_manual needs an entity Context, so it cannot run inside
        // update_global (which only offers &mut App).
        let (entity, _sub) = use_query_manual::<String, QueryError, _>(
            QueryKey::from("fetched"),
            crate::core::CachePolicy::NoCache,
            crate::core::RequestPolicy::LatestWins,
            cx,
        );
        H { entity, _handle }
    });

    // Resolve a real fetch; the success path bumps the dirty signal, waking
    // the persist_with observer.
    harness.update(cx, |this, cx| {
        fetch_query(
            &this.entity,
            || async { Ok::<_, QueryError>("fetched-value".to_string()) },
            cx,
        );
    });
    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(
            resource.status(),
            QueryStatus::Success,
            "the real fetch must resolve to Success before asserting on the save"
        );
        assert_eq!(resource.data(), Some(&"fetched-value".to_string()));
    });

    let saved = captured
        .lock()
        .unwrap()
        .clone()
        .expect("persist_with should have saved after the real fetch completed");
    let entry = saved
        .entries
        .get("fetched")
        .expect("saved snapshot should include the fetched entry");
    assert_eq!(
        entry.value,
        serde_json::json!("fetched-value"),
        "the snapshot value should be the fetched result, serialized"
    );
    let _ = harness;
}

// A real mutation completion bumps the dirty signal the same way. Mutation
// buckets are never collected into the snapshot; what gets saved is the
// retained Success query entry the bump wakes the observer for.

#[gpui::test]
fn test_persist_with_driven_by_real_mutation_completion(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();

    struct H {
        _query: Entity<QueryResource<String, QueryError>>,
        mutation: Entity<MutationResource<String, String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);
            client.persist_with(persister.clone(), zero_debounce(), cx)
        });
        // Prime a retained Success query entry; apply_success does NOT bump
        // CacheMutation, so no save fires from the priming itself.
        let query = cx.update_global::<QueryClient, _>(|client, cx| {
            let e = client.resource::<String, QueryError>(QueryKey::from("retained"), cx);
            e.update(cx, |r, _| {
                r.apply_success("data".to_string(), crate::client::current_time_ms())
            });
            e
        });
        let (mutation, _msub) = use_mutation::<String, String, QueryError, _>((), cx);
        H {
            _query: query,
            mutation,
            _handle,
        }
    });

    harness.update(cx, |this, cx| {
        mutate(
            &this.mutation,
            "vars".to_string(),
            |_vars| async move { Ok::<_, QueryError>("mutation-done".to_string()) },
            cx,
        );
    });
    cx.run_until_parked();

    cx.update(|cx| {
        let m = harness.read(cx).mutation.read(cx);
        assert_eq!(
            m.data().cloned(),
            Some("mutation-done".to_string()),
            "the real mutation must resolve to Success before asserting on the save"
        );
    });

    let saved = captured
        .lock()
        .unwrap()
        .clone()
        .expect("persist_with should have saved after the real mutation completed");
    assert!(
        saved.entries.contains_key("retained"),
        "the mutation-completion bump should have saved the retained Success entry: {:?}",
        saved.entries.keys().collect::<Vec<_>>()
    );
    let _ = harness;
}

// The infinite family: use_infinite_query auto-fetches the first page, and
// its success arm bumps CacheMutation. Infinite buckets are collected (first
// page only), so the page lands in the snapshot.

#[gpui::test]
fn test_persist_with_driven_by_real_infinite_completion(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();

    struct H {
        infinite: Entity<InfiniteQueryResource<Vec<String>, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            // The page type is Vec<String>; the serializer is keyed on it.
            client.register_serializer::<Vec<String>, QueryError>(|v| {
                serde_json::to_value(v).expect("serialize")
            });
            client.persist_with(persister.clone(), zero_debounce(), cx)
        });
        let (infinite, _isub) = use_infinite_query(
            InfiniteQueryOptions::new("infinite-feed")
                .cache_policy(crate::core::CachePolicy::Ttl { ttl_ms: 0 }),
            |_last_page| async move { Ok::<_, QueryError>((vec!["page-0".to_string()], true)) },
            cx,
        );
        H { infinite, _handle }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let r = harness.read(cx).infinite.read(cx);
        assert_eq!(
            r.status(),
            QueryStatus::Success,
            "the real infinite fetch must resolve to Success before asserting on the save"
        );
    });

    let saved = captured
        .lock()
        .unwrap()
        .clone()
        .expect("persist_with should have saved after the real infinite fetch completed");
    let entry = saved
        .entries
        .get("infinite-feed")
        .expect("saved snapshot should include the infinite first-page entry");
    assert_eq!(
        entry.value,
        serde_json::json!(["page-0"]),
        "the snapshot value should be the fetched first page, serialized"
    );
    let _ = harness;
}

// The imperative escape hatch (prepare_fetch_query, the fetchQuery
// equivalent) also bumps CacheMutation on completion, so imperative results
// are not invisible to persist_with.

#[gpui::test]
fn test_persist_with_driven_by_imperative_prepared_fetch(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();

    struct H {
        query: Entity<QueryResource<String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);
            client.persist_with(persister.clone(), zero_debounce(), cx)
        });
        // Retain the query via the real hook path so the bucket's WeakEntity
        // survives until the observer collects (prepare_fetch_query reuses
        // this same entity via resource()).
        let (query, _qsub) = use_query_manual::<String, QueryError, _>(
            QueryKey::from("imperative"),
            crate::core::CachePolicy::NoCache,
            crate::core::RequestPolicy::LatestWins,
            cx,
        );
        H { query, _handle }
    });

    harness.update(cx, |_this, cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            let prepared = client
                .prepare_fetch_query::<String, QueryError>(QueryKey::from("imperative"), cx)
                .expect("imperative fetch should start (resource is not fresh)");
            prepared.complete_success("imperative-value".to_string(), cx);
        });
    });
    cx.run_until_parked();

    cx.update(|cx| {
        let r = harness.read(cx).query.read(cx);
        assert_eq!(
            r.data(),
            Some(&"imperative-value".to_string()),
            "the imperative fetch must have completed before asserting on the save"
        );
    });

    let saved = captured
        .lock()
        .unwrap()
        .clone()
        .expect("persist_with should have saved after the imperative completion");
    let entry = saved
        .entries
        .get("imperative")
        .expect("saved snapshot should include the imperative Success entry");
    assert_eq!(
        entry.value,
        serde_json::json!("imperative-value"),
        "the snapshot value should be the imperative fetch result, serialized"
    );
    let _ = harness;
}

// Non-zero debounce with a deterministic clock: fire several rapid bumps,
// advance the mock clock past the window, and expect exactly one save
// containing the latest state.

#[gpui::test]
fn test_persist_with_debounce_coalesces(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();
    let save_count = persister.save_count.clone();

    let debounce = Duration::from_millis(50);

    // The bucket stores only WeakEntity; the harness keeps the Success entry
    // alive so the snapshot has something to save.
    struct H {
        _entity: Entity<QueryResource<String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let (_handle, entity) = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(ser_string);
            let handle = client.persist_with(
                persister.clone(),
                PersistOptions {
                    debounce,
                    ..PersistOptions::default()
                },
                cx,
            );
            let entity = client.resource::<String, QueryError>(QueryKey::from("coalesced"), cx);
            entity.update(cx, |r, _| {
                r.apply_success("seed".to_string(), crate::client::current_time_ms())
            });
            (handle, entity)
        });
        H {
            _entity: entity,
            _handle,
        }
    });

    // Each bump needs its own cx.update: GPUI coalesces notifications raised
    // within a single update, which would deliver only one observer call.
    for i in 0..5_u32 {
        cx.update(|cx| {
            cx.update_global::<QueryClient, _>(|client, cx| {
                client.set_query_data::<String, QueryError>("trigger", format!("v{i}"), cx);
            });
        });
    }
    cx.run_until_parked();
    assert_eq!(
        *save_count.lock().unwrap(),
        0,
        "no save should fire before the debounce window elapses"
    );

    // advance_clock matures the pending timer; exactly one task collects and
    // saves, any others wake to find the window already drained.
    cx.background_executor
        .advance_clock(debounce + Duration::from_millis(1));
    cx.run_until_parked();

    assert_eq!(
        *save_count.lock().unwrap(),
        1,
        "a rapid burst of mutations should coalesce into exactly one save"
    );
    let saved = captured
        .lock()
        .unwrap()
        .clone()
        .expect("the single coalesced save should have produced a snapshot");
    // The "trigger" entities die inside each update, so only the
    // harness-retained "coalesced" entry can appear.
    assert!(
        saved.entries.contains_key("coalesced"),
        "the coalesced save should include the retained Success entry: {:?}",
        saved.entries.keys().collect::<Vec<_>>()
    );
    let _ = harness;
}

/// Poll a future that is always immediately Ready (MemPersister's load is a
/// plain clone, no real async work) without pulling in an executor crate.
fn block_on_ready<R>(fut: impl std::future::Future<Output = R>) -> R {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    let mut cx = Context::from_waker(Waker::noop());
    let mut fut = Box::pin(fut);
    // SAFETY: pinned on the heap; we hold the only reference.
    let mut pinned: Pin<&mut dyn Future<Output = R>> = Pin::as_mut(&mut fut);
    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::hint::spin_loop(),
        }
    }
}
