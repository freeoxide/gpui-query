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
