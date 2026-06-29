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
use crate::core::{QueryError, QueryKey, QueryResource, QueryStatus};
use crate::hook::fetch_query;
use crate::hook::use_query_manual;
use crate::tests::test_support::*;

/// An in-memory persister that stores the last saved snapshot, for asserting
/// on the value-carrying payload in tests.
///
/// `save_count` tracks the number of `save` invocations so coalescing tests
/// can assert exactly how many saves actually fired (additive: existing tests
/// that ignore it are unaffected).
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

// ── H1: a REAL FETCH COMPLETION drives persist_with ──────────────────────
//
// The marquee Core-Change-2 behavior: when a query transitions to
// `QueryStatus::Success` through the real hook fetch path (not via
// `set_query_data`), the `CacheMutation` dirty signal is bumped inside
// `run_query_retry_loop`, which the `persist_with` observer collects and saves.
// This was previously untested — every other test in this file primes the
// cache via `set_query_data` or `apply_success` directly.

#[gpui::test]
fn test_persist_with_driven_by_real_fetch_completion(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();

    // The bucket stores only `WeakEntity`, so a Success entry must be held by a
    // live owner or it is dropped before the (async) observer collects the
    // snapshot. The harness holds the `use_query_manual` entity for the life of
    // the test, mirroring a real component holding the `Entity` from
    // `use_query`.
    struct H {
        entity: Entity<QueryResource<String, QueryError>>,
        _handle: PersistHandle,
    }
    let harness = cx.new(|cx| {
        // Global-layer setup: register the serializer and install the
        // `persist_with` driver. `cx` here is `&mut Context<H>`, which derefs
        // to `&mut App` for `update_global`.
        let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_serializer::<String, QueryError>(|s| {
                serde_json::to_value(s).expect("serialize")
            });
            client.persist_with(
                persister.clone(),
                // Zero debounce: the save task runs immediately once the fetch
                // resolves and bumps `CacheMutation`. (See
                // `test_persist_with_debounce_coalesces` for the non-zero
                // debounce / mock-clock path.)
                PersistOptions {
                    debounce: Duration::ZERO,
                    ..PersistOptions::default()
                },
                cx,
            )
        });
        // Create the resource via the REAL hook path so the bucket owns a
        // `WeakEntity` keyed under "fetched". `use_query_manual` requires an
        // entity `Context`, so it must run in the `cx.new` body (not inside
        // `update_global`, where only `&mut App` is available).
        let (entity, _sub) = use_query_manual::<String, QueryError, _>(
            QueryKey::from("fetched"),
            crate::core::CachePolicy::NoCache,
            crate::core::RequestPolicy::LatestWins,
            cx,
        );
        H { entity, _handle }
    });

    // Drive a REAL fetch to completion through the hook layer. The fetcher
    // resolves with a concrete value; on success the retry loop calls
    // `complete_success` and bumps `CacheMutation`, waking the `persist_with`
    // observer.
    harness.update(cx, |this, cx| {
        fetch_query(
            &this.entity,
            || async { Ok::<_, QueryError>("fetched-value".to_string()) },
            cx,
        );
    });

    // Pump the executor: the fetch future resolves, the success path bumps the
    // dirty signal, the observer collects a fresh snapshot, and (with ZERO
    // debounce) the spawned save task runs immediately.
    cx.run_until_parked();

    // Confirm the fetch really did complete (the trigger is NOT set_query_data).
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
    // The fetched entry's key and value must round-trip into the snapshot via
    // the registered serializer.
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

// ── L12: debounce COALESCING with a NON-zero debounce ────────────────────
//
// `TestAppContext::run_until_parked` drives the executor with `tick(false)`,
// which only fires delayed tasks whose deadline has already passed — it does
// NOT advance the mock wall-clock. However the background executor exposes
// `advance_clock(duration)` (gpui `test-support`), which advances the test
// dispatcher's clock AND matures any due timers. So a non-zero debounce CAN be
// exercised deterministically: fire several rapid mutations, let the bumps
// propagate and stash the latest snapshot, advance the clock past the debounce
// window, then assert EXACTLY ONE save fired containing the latest value.

#[gpui::test]
fn test_persist_with_debounce_coalesces(cx: &mut TestAppContext) {
    setup_query_client(cx);
    let persister = MemPersister::default();
    let captured = persister.last_saved.clone();
    let save_count = persister.save_count.clone();

    let debounce = Duration::from_millis(50);

    // Hold a live Success entry on the "coalesced" key so the snapshot actually
    // contains something to save (the bucket stores only WeakEntity).
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
        H { entity, _handle }
    });

    // Fire N>=3 rapid mutations on the same driver key. Each is issued in its
    // OWN `cx.update` so each bump delivers a separate `observe_global`
    // notification (GPUI coalesces notifications within a single update),
    // spawning a fresh debounced save task. All of these tasks share the single
    // `pending` slot, so only the latest snapshot can ever be saved, and only
    // one task drains the slot per debounce window.
    for i in 0..5_u32 {
        cx.update(|cx| {
            cx.update_global::<QueryClient, _>(|client, cx| {
                client.set_query_data::<String, QueryError>("trigger", format!("v{i}"), cx);
            });
        });
    }
    // Let the bumps propagate and stash the latest snapshot; the debounced save
    // tasks are now parked on their (un-matured) timers.
    cx.run_until_parked();
    // Nothing saved yet — the debounce window has not elapsed.
    assert_eq!(
        *save_count.lock().unwrap(),
        0,
        "no save should fire before the debounce window elapses"
    );

    // Advance the mock clock past the debounce window. `advance_clock` matures
    // the pending timer tasks; a subsequent `run_until_parked` lets exactly one
    // of them drain the shared slot and run `save`. The remaining tasks wake to
    // find the slot already empty and no-op.
    cx.background_executor.advance_clock(debounce + Duration::from_millis(1));
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
    // The "trigger" entities are not retained (created and dropped inside each
    // `set_query_data`), so they do not appear in the snapshot — only the
    // harness-retained "coalesced" Success entry survives. The point of this
    // assertion is that the single coalesced save captured the live cache.
    assert!(
        saved.entries.contains_key("coalesced"),
        "the coalesced save should include the retained Success entry: {:?}",
        saved.entries.keys().collect::<Vec<_>>()
    );
    let _ = harness;
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
