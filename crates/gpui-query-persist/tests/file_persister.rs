//! Tests for `FilePersister`: round-trip, concurrent saves, corrupt-file
//! tolerance, version rejection, and the `NoopPersister` no-op.
//!
//! These are plain `#[test]`s; the persister's `save`/`load` futures do no real
//! async work, so `pollster::block_on` suffices (no tokio needed).

use std::collections::HashMap;

use gpui_query::client::{
    NoopPersister, PERSIST_VERSION, PersistError, PersistSnapshot, PersistedEntry, Persister,
};
use gpui_query::core::CachePolicy;
use gpui_query_persist::{FilePersister, PersistFormat};

fn sample_snapshot() -> PersistSnapshot {
    let mut entries = HashMap::new();
    entries.insert(
        "users::42".to_string(),
        PersistedEntry {
            value: serde_json::json!({"name": "Alice"}),
            cached_at: 1_700_000_000_000,
            cache_policy: CachePolicy::Ttl { ttl_ms: 60_000 },
            meta: None,
        },
    );
    PersistSnapshot {
        entries,
        version: PERSIST_VERSION,
    }
}

#[test]
fn file_persister_round_trip_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.json");
    let p = FilePersister::json(&path);

    // Load from missing file → empty snapshot.
    let loaded = pollster::block_on(p.load()).expect("load missing");
    assert!(loaded.entries.is_empty());
    assert_eq!(loaded.version, PERSIST_VERSION);

    // Save + reload round-trips.
    let snap = sample_snapshot();
    pollster::block_on(p.save(&snap)).expect("save");
    let reloaded = pollster::block_on(p.load()).expect("reload");
    assert_eq!(reloaded.entries.len(), 1);
    assert_eq!(reloaded.version, PERSIST_VERSION);
    let entry = reloaded.entries.get("users::42").expect("entry present");
    assert_eq!(entry.cached_at, 1_700_000_000_000);
    assert_eq!(entry.cache_policy, CachePolicy::Ttl { ttl_ms: 60_000 });
}

#[test]
fn file_persister_round_trip_bincode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.bin");
    let p = FilePersister::bincode(&path);

    let snap = sample_snapshot();
    pollster::block_on(p.save(&snap)).expect("save");
    let reloaded = pollster::block_on(p.load()).expect("reload");
    assert_eq!(reloaded.entries.len(), 1);
    let entry = reloaded.entries.get("users::42").expect("entry present");
    assert_eq!(entry.value, serde_json::json!({"name": "Alice"}));
}

#[test]
fn file_persister_corrupt_file_yields_empty_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.json");
    std::fs::write(&path, b"{ this is not valid json").expect("write garbage");

    let p = FilePersister::json(&path);
    let loaded = pollster::block_on(p.load()).expect("tolerant load");
    assert!(loaded.entries.is_empty(), "corrupt cache → empty snapshot");
}

#[test]
fn file_persister_version_mismatch_is_typed_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.json");
    // Hand-write a snapshot with a wrong version.
    let wrong = serde_json::json!({
        "entries": {},
        "version": 9999,
    });
    std::fs::write(&path, serde_json::to_vec(&wrong).unwrap()).expect("write wrong-version");

    let p = FilePersister::json(&path);
    let err = pollster::block_on(p.load()).expect_err("version mismatch should error");
    match err {
        PersistError::VersionMismatch { expected, found } => {
            assert_eq!(expected, PERSIST_VERSION);
            assert_eq!(found, 9999);
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

#[test]
fn file_persister_concurrent_saves_do_not_corrupt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.json");
    let p = std::sync::Arc::new(FilePersister::json(&path));

    // Many concurrent saves; the internal Mutex serializes them so the final
    // on-disk file is always a complete, parseable snapshot.
    let mut handles = Vec::new();
    for i in 0..16u64 {
        let p = std::sync::Arc::clone(&p);
        handles.push(std::thread::spawn(move || {
            let mut snap = sample_snapshot();
            snap.entries.insert(
                format!("k{i}"),
                PersistedEntry {
                    value: serde_json::json!(i),
                    cached_at: i,
                    cache_policy: CachePolicy::NoCache,
                    meta: None,
                },
            );
            pollster::block_on(p.save(&snap)).expect("save");
        }));
    }
    for h in handles {
        h.join().expect("thread");
    }

    let final_ = pollster::block_on(p.load()).expect("final load");
    // The file is always parseable; the last writer's full snapshot survives.
    assert!(
        !final_.entries.is_empty(),
        "final snapshot should have at least one entry"
    );
    assert_eq!(final_.version, PERSIST_VERSION);
}

#[test]
fn noop_persister_round_trip() {
    let p = NoopPersister;
    let snap = sample_snapshot();
    pollster::block_on(p.save(&snap)).expect("noop save");
    let loaded = pollster::block_on(p.load()).expect("noop load");
    assert!(loaded.entries.is_empty());
    assert_eq!(loaded.version, PERSIST_VERSION);
}

#[test]
fn file_persister_format_choice_round_trips() {
    for fmt in [PersistFormat::Json, PersistFormat::Bincode] {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache");
        let p = FilePersister::new(&path, fmt);
        let snap = sample_snapshot();
        pollster::block_on(p.save(&snap)).expect("save");
        let reloaded = pollster::block_on(p.load()).expect("reload");
        assert_eq!(reloaded.entries.len(), 1, "format {fmt:?} round-trips");
    }
}

#[test]
fn file_persister_large_snapshot_round_trips() {
    // A large snapshot with distinct keys and realistic-sized JSON values,
    // round-tripped through BOTH formats. Covers the "large snapshot" row of
    // docs/features.md and guards against truncation/size-sensitive regressions
    // in the atomic-write and tolerant-load paths.
    const N: usize = 10_000;

    let mut entries = HashMap::with_capacity(N);
    for i in 0..N {
        let key = format!("users::{i}");
        let value = serde_json::json!({
            "id": i,
            "name": format!("User {i}"),
            "email": format!("user{i}@example.test"),
            "active": i % 2 == 0,
            "tags": [format!("tag-{i}"), "member", "persisted"],
            "profile": {
                "bio": format!("Auto-generated biography for user {i}."),
                "followers": i as u64 * 3,
            },
        });
        entries.insert(
            key,
            PersistedEntry {
                value,
                cached_at: 1_700_000_000_000 + i as u64,
                cache_policy: if i % 2 == 0 {
                    CachePolicy::Ttl { ttl_ms: 60_000 }
                } else {
                    CachePolicy::NoCache
                },
                meta: None,
            },
        );
    }
    let snap = PersistSnapshot {
        entries,
        version: PERSIST_VERSION,
    };

    for fmt in [PersistFormat::Json, PersistFormat::Bincode] {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache");
        let p = FilePersister::new(&path, fmt);

        pollster::block_on(p.save(&snap)).expect("save");
        let reloaded = pollster::block_on(p.load()).expect("reload");

        assert_eq!(
            reloaded.entries.len(),
            N,
            "format {fmt:?}: all {N} entries survived the round-trip"
        );
        assert_eq!(reloaded.version, PERSIST_VERSION);

        // Sample a well-known even-indexed entry (so active==true and policy is
        // Ttl, matching the loop's parity rules) to lock in value-level integrity.
        let sample_key = "users::9000";
        let entry = reloaded
            .entries
            .get(sample_key)
            .unwrap_or_else(|| panic!("format {fmt:?}: sampled entry {sample_key} present"));
        assert_eq!(entry.cached_at, 1_700_000_000_000 + 9000);
        assert_eq!(entry.cache_policy, CachePolicy::Ttl { ttl_ms: 60_000 });
        assert_eq!(entry.value, serde_json::json!({
            "id": 9000,
            "name": "User 9000",
            "email": "user9000@example.test",
            "active": true,
            "tags": ["tag-9000", "member", "persisted"],
            "profile": {
                "bio": "Auto-generated biography for user 9000.",
                "followers": 27000,
            },
        }));
    }
}

#[test]
fn file_persister_corrupt_bincode_yields_empty_snapshot() {
    // Mirror of `file_persister_corrupt_file_yields_empty_snapshot`, but for
    // the bincode deserialize-failure branch (src/lib.rs:208-214), which
    // previously had zero test coverage: garbage bytes on disk must degrade to
    // an empty snapshot without panicking or erroring.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.bin");
    std::fs::write(&path, b"\x00\x01\x02 this is definitely not bincode \xff\xfe garbage").expect("write garbage");

    let p = FilePersister::bincode(&path);
    let loaded = pollster::block_on(p.load()).expect("tolerant load");
    assert!(loaded.entries.is_empty(), "corrupt bincode → empty snapshot");
    assert_eq!(loaded.version, PERSIST_VERSION);
}
