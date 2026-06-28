//! Integration tests for the value-carrying persistence layer (`persist`
//! feature): `persist_with` debounce/coalescing, the typed serializer/deserializer
//! registries, `hydrate` round-trip, the [`CacheMutation`] dirty signal firing
//! on `set_query_data`, and `PersistFilter` / `max_age` behavior.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use gpui::{AppContext as _, BorrowAppContext as _, Entity, TestAppContext};

use crate::client::{
    CacheMutation, NoopPersister, PersistError, PersistFilter, PersistHandle, PersistOptions,
    PersistSnapshot, PersistedEntry, Persister, QueryClient, hydrate,
};
use crate::core::{QueryError, QueryKey, QueryResource};
use crate::tests::test_support::*;

/// An in-memory persister that stores the last saved snapshot, for asserting
/// on the value-carrying payload in tests.
#[derive(Default, Clone)]
struct MemPersister {
    last_saved: Arc<StdMutex<Option<PersistSnapshot>>>,
    load_value: Arc<StdMutex<Option<PersistSnapshot>>>,
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
        *self.last_saved.lock().unwrap() = Some(snapshot.clone());
        Ok(())
    }
}

#[gpui::test]
fn test_set_query_data_bumps_cache_mutation(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        // Install persist_with so the CacheMutation observation is live; the
        // bump must not panic.
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            client.persist_with(NoopPersister, PersistOptions::default(), cx)
        });
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.set_query_data::<String, QueryError>("k1", "v1".to_string(), cx);
        });
        // The marker global now exists.
        assert!(cx.has_global::<CacheMutation>());
    });
}

#[gpui::test]
fn test_collect_persist_snapshot_uses_registered_serializer(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(|s| {
                serde_json::to_value(s).expect("serialize")
            });

            let e = client.resource::<String, QueryError>(QueryKey::from("snap_k"), cx);
            e.update(cx, |r, _| {
                r.apply_success("payload".to_string(), crate::client::current_time_ms())
            });

            let snap = client.collect_persist_snapshot(
                &PersistFilter::All,
                Duration::from_secs(60 * 60 * 24),
                cx,
            );
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
            // NO serializer registered → entry skipped (metadata-only fallback).
            let e = client.resource::<String, QueryError>(QueryKey::from("unreg"), cx);
            e.update(cx, |r, _| {
                r.apply_success("data".to_string(), crate::client::current_time_ms())
            });

            let snap =
                client.collect_persist_snapshot(&PersistFilter::All, Duration::from_secs(3600), cx);
            assert!(
                snap.entries.is_empty(),
                "unregistered type → no value-carrying entry"
            );
        });
    });
}

#[gpui::test]
fn test_collect_persist_snapshot_filter_and_max_age(cx: &mut TestAppContext) {
    setup_query_client(cx);
    cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(|s| {
                serde_json::to_value(s).expect("serialize")
            });
            let now = crate::client::current_time_ms();
            // Two recent entries under the "users" prefix.
            for parts in [["users", "1"], ["users", "2"]] {
                let e = client.resource::<String, QueryError>(QueryKey::from(parts), cx);
                e.update(cx, |r, _| r.apply_success("v".to_string(), now));
            }
            // One OLD entry (≈2.7 h in the past) under "posts".
            let e = client.resource::<String, QueryError>(QueryKey::from(["posts", "9"]), cx);
            e.update(cx, |r, _| {
                r.apply_success("old".to_string(), now.saturating_sub(10_000_000))
            });

            // Prefix "users" → exactly the two users entries.
            let snap = client.collect_persist_snapshot(
                &PersistFilter::Prefix(QueryKey::from(["users"])),
                Duration::from_secs(3600),
                cx,
            );
            assert_eq!(snap.entries.len(), 2);

            // All + 1 s max_age → the old "posts::9" entry is filtered out; the
            // two recent users entries (age ~0) survive. (max_age = 0 means
            // *disabled*, not "all too old", so a positive small max_age is used.)
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

    // The bucket stores only `WeakEntity`, so a Success entry must be held by a
    // live owner or it is dropped before the (async) observer callback collects
    // the snapshot. The harness holds it for the life of the test, mirroring a
    // real component holding the `Entity` from `use_query`.
    struct H {
        entity: Entity<QueryResource<String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        let (_handle, entity) = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(|s| {
                serde_json::to_value(s).expect("serialize")
            });
            let handle = client.persist_with(
                persister.clone(),
                // Zero debounce: the TestAppContext mock clock does not advance
                // wall-clock timers, so a non-zero debounce would never let the
                // save fire here. The debounce *logic* (the `is_zero`
                // short-circuit) is still exercised; a real app uses a non-zero
                // debounce.
                PersistOptions {
                    debounce: Duration::ZERO,
                    ..PersistOptions::default()
                },
                cx,
            );
            let entity = client.resource::<String, QueryError>(QueryKey::from("persisted"), cx);
            entity.update(cx, |r, _| {
                r.apply_success("data".to_string(), crate::client::current_time_ms())
            });
            (handle, entity)
        });
        H { entity, _handle }
    });

    // A mutation (on another key) bumps the dirty signal → persist_with collects
    // the live cache (including the retained Success entry) and saves.
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

    // Pre-load a snapshot with one entry whose value is a JSON string.
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

    // The bucket stores only `WeakEntity`, so hold the "hydrate_k" entity from a
    // harness; hydrate's `set_query_data` then reuses the retained entity rather
    // than creating one that is immediately dropped.
    struct H {
        entity: Entity<QueryResource<String, QueryError>>,
    }
    let harness = cx.new(|cx| {
        cx.update_global::<QueryClient, _>(|client, _cx| {
            client
                .register_deserializer::<String, QueryError>(|v| v.as_str().map(|s| s.to_string()));
        });
        let entity = cx.update_global::<QueryClient, _>(|client, cx| {
            client.resource::<String, QueryError>(QueryKey::from("hydrate_k"), cx)
        });
        H { entity }
    });

    // Run the async hydrate on the background executor, then assert priming.
    let filter = PersistFilter::All;
    let max_age = Duration::from_secs(60 * 60 * 24);
    let outcome = cx.update(|cx| {
        cx.update_global::<QueryClient, _>(|client, cx| {
            // Drive hydrate synchronously inside the global lease: it is an
            // async fn whose future is immediately Ready (the MemPersister's
            // load is a clone, no real await).
            // We cannot `.await` here, so block on it via a tiny executor.
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

#[gpui::test]
fn test_hydrate_rejects_version_mismatch(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    // A snapshot with a bogus version.
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
    let max_age = Duration::from_secs(60 * 60 * 24);
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

// Poll a future that is always immediately Ready (the MemPersister's load is a
// plain clone, no real async work) without pulling in an executor crate.
fn block_on_ready<R>(fut: impl std::future::Future<Output = R>) -> R {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct NoWake;
    impl Wake for NoWake {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(NoWake));
    let mut cx = Context::from_waker(&waker);
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
