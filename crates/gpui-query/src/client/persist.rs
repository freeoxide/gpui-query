//! Async, value-carrying persistence for [`QueryClient`](super::QueryClient):
//! the [`Persister`] trait, the debounced [`QueryClient::persist_with`] driver,
//! [`hydrate`], and the typed serializer/deserializer registries.
//!
//! This layer trusts a persister's `load` output beyond a version check, but
//! never panics on it: unrecognized versions error out and values no
//! deserializer accepts are skipped. A persister reading untrusted storage
//! should validate and size-limit payloads itself.

use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::{App, Subscription};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::core::{CachePolicy, QueryKey};

use super::QueryClient;

/// Current on-disk snapshot format version. Bumped when the serialized shape
/// of [`PersistSnapshot`] changes incompatibly; loaders reject mismatches with
/// [`PersistError::VersionMismatch`].
pub const PERSIST_VERSION: u32 = 1;

// ── Errors ───────────────────────────────────────────────────────────────

/// Errors produced by the persistence layer.
///
/// Every IO failure from a [`Persister`] implementation maps to a variant
/// here rather than panicking; tolerant persisters degrade a corrupt store to
/// an empty snapshot.
#[derive(Debug, Error)]
pub enum PersistError {
    /// An underlying IO error (read or write) failed.
    #[error("persistence io error: {0}")]
    Io(#[from] std::io::Error),
    /// Serializing the snapshot (or an entry) to the persister's format failed.
    #[error("persistence serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The on-disk snapshot could not be parsed. Reserved for persisters that
    /// surface (rather than tolerate) parse failures; core never constructs
    /// this variant.
    #[error("persistence deserialize error: {0}")]
    Deserialize(String),
    /// The on-disk snapshot's `version` does not match [`PERSIST_VERSION`],
    /// so the file was written by a format we cannot read.
    #[error("persistence version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// The version this loader understands ([`PERSIST_VERSION`]).
        expected: u32,
        /// The version actually found on disk.
        found: u32,
    },
    /// The requested path was unusable (e.g. the OS returned no cache dir).
    #[error("persistence bad path: {0}")]
    BadPath(String),
    /// The persister could not acquire a required resource (e.g. file lock).
    #[error("persistence permission denied: {0}")]
    Permission(String),
}

// ── Snapshot types ───────────────────────────────────────────────────────

/// One persisted cache entry: the typed data as an opaque JSON value plus the
/// metadata needed to re-prime and re-validate it.
///
/// `value` is opaque to core; the typed round-trip is driven by the
/// serializer/deserializer registries on [`QueryClient`].
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PersistedEntry {
    /// The serialized data value. Opaque to core.
    pub value: JsonValue,
    /// Wall-clock ms (since UNIX epoch) the entry was cached.
    pub cached_at: u64,
    /// The cache policy in force when the entry was cached.
    pub cache_policy: CachePolicy,
    /// Optional opaque metadata (e.g. ETag/Last-Modified for HTTP), captured
    /// from `Fetched::meta` at fetch completion. Reserved for the
    /// `gpui-query-http` companion crate.
    pub meta: Option<JsonValue>,
}

/// A full snapshot of the persistable cache, ready to hand to a [`Persister`].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PersistSnapshot {
    /// The persistable entries, keyed by [`QueryKey`] path string so the
    /// snapshot is self-contained and serializable.
    pub entries: HashMap<String, PersistedEntry>,
    /// Format version; see [`PERSIST_VERSION`].
    pub version: u32,
}

impl PersistSnapshot {
    /// Construct an empty snapshot at the current version.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            version: PERSIST_VERSION,
        }
    }
}

// ── Owned filter (vs core's borrowing QueryKeyFilter<'a>) ────────────────

/// Owned counterpart to [`QueryKeyFilter`](crate::core::QueryKeyFilter), so a
/// filter can be pinned inside long-lived structures like
/// [`PersistOptions`].
#[derive(Clone, Debug)]
pub enum PersistFilter {
    /// Persist only the entry matching exactly this key.
    Exact(QueryKey),
    /// Persist every entry whose key starts with this prefix.
    Prefix(QueryKey),
    /// Persist every persistable entry.
    All,
}

impl PersistFilter {
    /// Returns `true` if `key` should be included under this filter.
    pub fn matches(&self, key: &QueryKey) -> bool {
        match self {
            PersistFilter::Exact(target) => key == target,
            PersistFilter::Prefix(prefix) => key.starts_with(prefix),
            PersistFilter::All => true,
        }
    }
}

/// Tuning knobs for [`QueryClient::persist_with`].
///
/// `Default` is: every entry, max age 24 hours, 500 ms debounce.
#[derive(Clone, Debug)]
pub struct PersistOptions {
    /// Which entries to include.
    pub filter: PersistFilter,
    /// Skip entries older than this at save time.
    pub max_age: Duration,
    /// Coalesce bursts of [`CacheMutation`](super::CacheMutation) into one
    /// save per window. [`Duration::ZERO`] skips the delay: a bump arriving
    /// while no save is pending saves immediately.
    pub debounce: Duration,
}

impl Default for PersistOptions {
    fn default() -> Self {
        Self {
            filter: PersistFilter::All,
            max_age: Duration::from_secs(24 * 60 * 60),
            debounce: Duration::from_millis(500),
        }
    }
}

// ── Serializer / deserializer registries ─────────────────────────────────

/// Type-erased serializer closure: `&dyn Any -> Option<serde_json::Value>`.
///
/// `None` means the downcast failed; the caller then skips the entry rather
/// than persisting junk.
type SerializeFn = Box<dyn Fn(&dyn std::any::Any) -> Option<JsonValue> + Send + Sync>;

/// Registry of `T -> serde_json::Value` serializers, keyed by `TypeId` of
/// the resource's data type `T`.
///
/// Keyed on `T` alone, matching the bucket lookup: serialization depends only
/// on the data type, so registering for the same `T` under two error types
/// overwrites (last write wins), and the surviving closure applies to every
/// `(T, E)` bucket. That is correct because the value is that `T`.
#[derive(Default)]
pub struct SerializerRegistry {
    serializers: HashMap<TypeId, SerializeFn>,
}

impl SerializerRegistry {
    /// Register a serializer for `T`. `f` is a plain `fn` pointer (no
    /// captures) so it is `Send + Sync + 'static` without boxing.
    pub fn register<T: 'static>(&mut self, f: fn(&T) -> JsonValue) {
        let wrap = move |any: &dyn std::any::Any| -> Option<JsonValue> {
            // Downcast failure is unreachable (buckets look the closure up by
            // `TypeId::of::<T>()`), but degrade to None so this path can
            // never panic.
            any.downcast_ref::<T>().map(f)
        };
        self.serializers.insert(TypeId::of::<T>(), Box::new(wrap));
    }

    /// Look up the serializer registered for `type_id`.
    pub(crate) fn get(&self, type_id: TypeId) -> Option<&SerializeFn> {
        self.serializers.get(&type_id)
    }

    /// Returns `true` if a serializer is registered for `type_id`.
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.serializers.contains_key(&type_id)
    }
}

/// Type-erased hydrate step: decode a `JsonValue` and prime the live cache
/// via `set_query_data::<T, E>`. The concrete types are captured at the
/// `register` call site, so no cross-type confusion is possible.
type HydrateStep =
    Arc<dyn Fn(&mut QueryClient, &QueryKey, &JsonValue, &mut App) -> bool + Send + Sync>;

/// Registry of `serde_json::Value -> primed cache entry` steps, used by
/// [`hydrate`] to re-prime on-disk values. Each step returns `true` if it
/// decoded and primed the value, `false` to skip the entry.
#[derive(Default)]
pub struct DeserializerRegistry {
    steps: Vec<HydrateStep>,
}

impl DeserializerRegistry {
    /// Register a deserializer for resources of type `(T, E)`. `deserialize`
    /// returns `None` for values it cannot decode; the entry is then skipped.
    /// On `Some(t)` the value is primed via `set_query_data::<T, E>`.
    pub fn register<T, E>(&mut self, deserialize: fn(&JsonValue) -> Option<T>)
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        let step = move |client: &mut QueryClient,
                         key: &QueryKey,
                         value: &JsonValue,
                         cx: &mut App|
              -> bool {
            let Some(t) = deserialize(value) else {
                return false;
            };
            client.set_query_data::<T, E>(key.clone(), t, cx);
            true
        };
        self.steps.push(Arc::new(step));
    }

    fn iter(&self) -> impl Iterator<Item = &HydrateStep> {
        self.steps.iter()
    }
}

// ── Persister trait ──────────────────────────────────────────────────────

/// Async persistence backend for [`QueryClient::persist_with`].
///
/// Non-object-safe (methods return `impl Future`): `persist_with<P>`
/// monomorphizes the driver around the concrete `P`, avoiding
/// `Pin<Box<dyn Future>>` overhead and keeping the `Send + 'static` bounds
/// visible at the call site. The save future runs on GPUI's background
/// executor.
///
/// See the `FilePersister` adapter in the `gpui-query-persist` satellite
/// crate for a reference disk implementation.
pub trait Persister: Send + Sync + 'static {
    /// Load the snapshot from storage. Implementations should tolerate a
    /// missing or corrupt store by yielding an empty snapshot (or a typed
    /// [`PersistError`] for version mismatches).
    fn load(&self) -> impl Future<Output = Result<PersistSnapshot, PersistError>> + Send;

    /// Save `snapshot`, replacing any previously stored data.
    fn save(
        &self,
        snapshot: &PersistSnapshot,
    ) -> impl Future<Output = Result<(), PersistError>> + Send;
}

// ── PersistHandle ────────────────────────────────────────────────────────

/// Drop-guard returned by [`QueryClient::persist_with`].
///
/// Holding the handle keeps the [`CacheMutation`](super::CacheMutation)
/// observation alive; dropping it stops new saves from being scheduled. A
/// task that is already armed still collects and completes its final save,
/// so nothing pending at drop time is lost.
pub struct PersistHandle {
    // Subscription is dropped when the handle is, ending observation.
    _subscription: Option<Subscription>,
}

impl PersistHandle {
    /// Construct a handle that does nothing on drop (for tests / no-op).
    pub fn empty() -> Self {
        Self {
            _subscription: None,
        }
    }
}

// ── QueryClient methods ─────────────────────────────────────────────────

impl QueryClient {
    /// Register a serializer for resources of data type `T`.
    ///
    /// Only `Success` resources whose `T` has a registered serializer are
    /// emitted by [`collect_persist_snapshot`](Self::collect_persist_snapshot);
    /// unregistered types are skipped.
    pub fn register_serializer<T, E>(&mut self, f: fn(&T) -> JsonValue)
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        let registry = self
            .serializers
            .get_or_insert_with(SerializerRegistry::default);
        registry.register::<T>(f);
    }

    /// Register a deserializer for resources of type `(T, E)`, enabling
    /// [`hydrate`] to re-prime on-disk values of this type.
    ///
    /// [`hydrate`] offers every on-disk entry to every registered
    /// deserializer (there is no type discriminator on [`PersistedEntry`]).
    /// A deserializer MUST return `None` for any JSON shape that is not its
    /// own `T`; a lax one can prime a stale or foreign value into a bucket
    /// it does not belong to.
    pub fn register_deserializer<T, E>(&mut self, deserialize: fn(&JsonValue) -> Option<T>)
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        let registry = self
            .deserializers
            .get_or_insert_with(DeserializerRegistry::default);
        registry.register::<T, E>(deserialize);
    }

    /// Collect a value-carrying snapshot from the live cache, honoring
    /// `filter` and `max_age`. Only `Success` resources with a registered
    /// serializer are included.
    pub fn collect_persist_snapshot(
        &self,
        filter: &PersistFilter,
        max_age: Duration,
        cx: &App,
    ) -> PersistSnapshot {
        let Some(ref registry) = self.serializers else {
            return PersistSnapshot::new();
        };
        let now_ms = crate::client::time::current_time_ms();
        let max_age_ms = max_age.as_millis() as u64;

        let mut out: Vec<(QueryKey, PersistedEntry)> = Vec::new();
        for bucket in self.buckets.values() {
            bucket.collect_persistable_into(cx, registry, now_ms, &mut out);
        }
        for bucket in self.infinite_buckets.values() {
            bucket.collect_persistable_into(cx, registry, now_ms, &mut out);
        }

        // Attach metadata recorded at fetch completion (record_meta) so HTTP
        // CacheMeta and similar round-trip through PersistedEntry.meta.
        if let Some(meta_map) = &self.persisted_meta {
            for (key, entry) in &mut out {
                if let Some(m) = meta_map.get(key) {
                    entry.meta = Some(m.clone());
                }
            }
        }

        let mut snapshot = PersistSnapshot::new();
        for (key, entry) in out {
            if !filter.matches(&key) {
                continue;
            }
            if max_age_ms > 0 && now_ms.saturating_sub(entry.cached_at) > max_age_ms {
                continue;
            }
            snapshot.entries.insert(key.to_path(), entry);
        }
        snapshot
    }

    /// Drive a [`Persister`] from the live cache, debounced on the
    /// [`CacheMutation`](super::CacheMutation) dirty signal.
    ///
    /// Each bump arms at most one main-thread task; after `opts.debounce`
    /// the task collects a fresh [`PersistSnapshot`] and runs
    /// `persister.save(&snapshot)` on the background executor. Because
    /// collection happens at drain time, a burst of bumps coalesces into one
    /// save of the latest state. Dropping the returned [`PersistHandle`]
    /// stops scheduling new saves; an armed task still finishes.
    pub fn persist_with<P: Persister>(
        &self,
        persister: P,
        opts: PersistOptions,
        cx: &mut App,
    ) -> PersistHandle {
        let persister: Arc<P> = Arc::new(persister);
        let debounce = opts.debounce;
        let bg = cx.background_executor().clone();
        // At most one armed task per window; cleared by the task itself just
        // before it collects, on every path.
        let armed = Arc::new(AtomicBool::new(false));

        // Seed the marker so observation is registered against a global that
        // already exists; bump sites use the same idempotent seeding.
        let _ = cx.default_global::<super::CacheMutation>();

        let subscription = {
            let persister = persister.clone();
            let armed = armed.clone();
            let filter = opts.filter;
            let max_age = opts.max_age;
            cx.observe_global::<super::CacheMutation>(move |cx| {
                // If a task is already armed it will collect after this bump
                // when its window elapses; nothing else to do.
                if armed.swap(true, Ordering::AcqRel) {
                    return;
                }
                let persister = persister.clone();
                let filter = filter.clone();
                let armed = armed.clone();
                let bg = bg.clone();
                cx.spawn(async move |cx| {
                    if !debounce.is_zero() {
                        bg.timer(debounce).await;
                    }
                    // Disarm before collecting: a bump landing now arms a
                    // fresh task instead of trusting one about to finish.
                    armed.store(false, Ordering::Release);
                    let Ok(snapshot) = cx.update_global::<QueryClient, _>(|client, cx| {
                        client.collect_persist_snapshot(&filter, max_age, cx)
                    }) else {
                        return;
                    };
                    // Collect on the main thread (entity reads), save on the
                    // background executor (IO), per the Persister contract.
                    bg.spawn(async move {
                        if let Err(err) = persister.save(&snapshot).await {
                            #[cfg(debug_assertions)]
                            eprintln!("persist_with: save failed: {err}");
                        }
                    })
                    .detach();
                })
                .detach();
            })
        };

        PersistHandle {
            _subscription: Some(subscription),
        }
    }
}

// ── NoopPersister ────────────────────────────────────────────────────────

/// A [`Persister`] that persists nothing and loads an empty snapshot.
///
/// Useful as a default, in tests that only exercise the debounce path, or as
/// a base to compose with a real persister behind a feature flag.
pub struct NoopPersister;

impl Persister for NoopPersister {
    async fn load(&self) -> Result<PersistSnapshot, PersistError> {
        Ok(PersistSnapshot::new())
    }

    async fn save(&self, _snapshot: &PersistSnapshot) -> Result<(), PersistError> {
        Ok(())
    }
}

// ── hydrate ──────────────────────────────────────────────────────────────

/// Load a snapshot from `persister` and re-prime the live cache with it: the
/// value-carrying counterpart to the metadata-only
/// [`QueryClient::hydrate`](super::QueryClient::hydrate).
///
/// Every entry surviving `filter` and `max_age` is offered to every
/// registered deserializer (see [`QueryClient::register_deserializer`]);
/// each one that decodes primes the value via `set_query_data`. Entries no
/// deserializer accepts are skipped. Stored keys are `to_path()` strings;
/// they are split back into segments so `Exact`/`Prefix` filters match the
/// live multi-segment key shapes.
///
/// Returns the loaded snapshot (post-filter) so callers can inspect entries
/// or prime types with no registered deserializer themselves. Errors from
/// `load` propagate.
pub async fn hydrate<P: Persister>(
    client: &mut QueryClient,
    persister: &P,
    filter: &PersistFilter,
    max_age: Duration,
    cx: &mut App,
) -> Result<PersistSnapshot, PersistError> {
    let snapshot = persister.load().await?;
    // Check even if the persister already enforces the version, so an
    // in-memory persister cannot feed a mismatched snapshot through.
    if snapshot.version != PERSIST_VERSION {
        return Err(PersistError::VersionMismatch {
            expected: PERSIST_VERSION,
            found: snapshot.version,
        });
    }
    let now_ms = crate::client::time::current_time_ms();
    let max_age_ms = max_age.as_millis() as u64;

    let Some(deserializers) = client.deserializers.as_ref() else {
        return Ok(snapshot);
    };

    // Clone the steps (cheap Arc bumps) so the immutable borrow on `client`
    // ends before each step takes `&mut QueryClient` for set_query_data.
    let steps: Vec<HydrateStep> = deserializers.iter().cloned().collect();

    // One key reconstruction and filter pass per entry; every step then gets
    // a shot at the value.
    for (key_path, entry) in &snapshot.entries {
        let key = QueryKey::new(key_path.split("::"));
        if !filter.matches(&key) {
            continue;
        }
        if max_age_ms > 0 && now_ms.saturating_sub(entry.cached_at) > max_age_ms {
            continue;
        }
        for step in &steps {
            step(client, &key, &entry.value, cx);
        }
    }

    Ok(snapshot)
}
