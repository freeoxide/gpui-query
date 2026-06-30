//! Async, value-carrying persistence layer for [`QueryClient`](super::QueryClient).
//!
//! This is the Phase B enrichment of the shipped (synchronous, metadata-only)
//! skeleton. It adds:
//!
//! - an async [`Persister`] trait (non-object-safe; generic over the future),
//! - a value-carrying [`PersistedEntry`] (opaque `serde_json::Value` payload),
//! - a debounced [`QueryClient::persist_with`] driver keyed off the precise
//!   [`CacheMutation`](super::CacheMutation) dirty signal,
//! - a typed-serializer registry so core can serialize concrete `T` without a
//!   `T: Serialize` bound leaking onto every resource, and a matching
//!   deserializer registry so [`hydrate`] can re-prime concrete values.
//!
//! See `docs/features.md` and the plan (`snug-wobbling-puzzle.md`, Phase B) for
//! the design rationale.

use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, BorrowAppContext as _, Subscription};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::core::{CachePolicy, QueryKey};

use super::QueryClient;
// The erased-bucket traits' `collect_persistable_into` methods are dispatched
// via the trait object's vtable (`Box<dyn ErasedBucket>`), so the traits
// themselves need not be imported here.

/// Current on-disk snapshot format version. Bumped when the serialized shape of
/// [`PersistSnapshot`] changes in a backwards-incompatible way; loaders reject
/// mismatched versions with [`PersistError::VersionMismatch`].
pub const PERSIST_VERSION: u32 = 1;

// ── Errors ───────────────────────────────────────────────────────────────

/// Errors produced by the persistence layer.
///
/// Every IO failure from a [`Persister`] implementation is mapped to a variant
/// here rather than panicked on; loaders tolerate corrupt/missing files (see
/// [`FilePersister`](../../gpui_query_persist/struct.FilePersister.html)) by
/// degrading to an empty snapshot.
#[derive(Debug, Error)]
pub enum PersistError {
    /// An underlying IO error (read or write) failed.
    #[error("persistence io error: {0}")]
    Io(#[from] std::io::Error),
    /// Serializing the snapshot (or an entry) to the persister's format failed.
    #[error("persistence serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The on-disk snapshot could not be parsed / deserialized.
    ///
    /// Reserved for persister implementations that surface (rather than
    /// tolerate) deserialization failures; the shipped `FilePersister` degrades
    /// corrupt stores to an empty snapshot instead, so core never constructs
    /// this variant. It is retained on the public API for backends that prefer
    /// to propagate parse errors.
    #[error("persistence deserialize error: {0}")]
    Deserialize(String),
    /// The on-disk snapshot's `version` does not match [`PERSIST_VERSION`].
    ///
    /// Treated as a typed error (rather than silent empty-snapshot) so callers
    /// can distinguish "file was corrupt" from "file was written by a
    /// newer/older format we cannot read".
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

/// A single persisted cache entry carrying the typed data as an opaque JSON
/// value plus the metadata needed to re-prime and re-validate it.
///
/// `value` is opaque to core (a `serde_json::Value`); the typed round-trip is
/// driven by the serializer/deserializer registries on [`QueryClient`]. This is
/// Open Question 3 from the design doc.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PersistedEntry {
    /// The serialized data value. Opaque to core.
    pub value: JsonValue,
    /// Wall-clock ms (since UNIX epoch) the entry was cached.
    pub cached_at: u64,
    /// The cache policy in force when the entry was cached.
    pub cache_policy: CachePolicy,
    /// Optional opaque metadata (e.g. ETag/Last-Modified for HTTP). Reserved
    /// for the `gpui-query-http` companion crate.
    pub meta: Option<JsonValue>,
}

/// A full snapshot of the persistable cache, ready to hand to a [`Persister`].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PersistSnapshot {
    /// The persistable entries, keyed by [`QueryKey`] path string.
    ///
    /// Keys are stored as their `to_path()` `String` form so the snapshot is
    /// self-contained (no `Arc` aliasing across processes) and serializable.
    pub entries: HashMap<String, PersistedEntry>,
    /// Format version; see [`PERSIST_VERSION`] and
    /// [`PersistError::VersionMismatch`].
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

/// Owned counterpart to [`QueryKeyFilter`](crate::core::QueryKeyFilter) for the
/// persistence layer.
///
/// The core filter borrows (`Exact(&QueryKey)` / `Prefix(&QueryKey)`) and is
/// therefore neither `Serialize` nor storable inside [`PersistOptions`]; this
/// owned enum lets a caller pin the filter into a long-lived `persist_with`
/// driver.
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
/// `Default` is: every entry, max age 24 hours, 500 ms debounce — a sensible
/// "save the cache to disk shortly after it changes" baseline.
#[derive(Clone, Debug)]
pub struct PersistOptions {
    /// Which entries to include.
    pub filter: PersistFilter,
    /// Skip entries older than this at save time.
    pub max_age: Duration,
    /// Coalesce bursts of [`CacheMutation`](super::CacheMutation) into one save
    /// per window.
    ///
    /// Passing [`Duration::ZERO`] disables the timer-based coalescing window —
    /// each bump still races to drain the pending slot, but there is no
    /// batching delay (saves still serialize through the drain slot).
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
/// `None` means the downcast to the registered `T` failed — unreachable by
/// construction (the bucket looks the closure up by `TypeId::of::<T>()` and
/// passes that same `T`), but callers skip the entry rather than persisting a
/// placeholder, so a future invariant break can never panic the foreground
/// thread from inside the persistence path.
type SerializeFn = Box<dyn Fn(&dyn std::any::Any) -> Option<JsonValue> + Send + Sync>;

/// Registry of `T -> serde_json::Value` serializers, keyed by `TypeId` of the
/// resource's data type `T`.
///
/// The keying is intentionally on `T` alone (not the full `(T, E)` resource
/// pair): the bucket impls (`erased_ops.rs`, `infinite_bucket.rs`) likewise
/// look up by `TypeId::of::<T>()`, so insert and lookup are consistent on `T`.
/// Serialization only depends on the data type, not the error type. Consequence:
/// registering serializers for the same `T` under two different `E` types
/// silently overwrites (last write wins); whichever serializer survives is
/// applied to both `(T, E)` buckets, which is correct because the value *is*
/// that `T`.
///
/// Stored closures accept `&dyn Any` and downcast internally, so the registry
/// stays free of `T: Serialize` bounds on the resource itself.
#[derive(Default)]
pub struct SerializerRegistry {
    serializers: HashMap<TypeId, SerializeFn>,
}

impl SerializerRegistry {
    /// Register a serializer for `T`.
    ///
    /// `f` receives a `&T` already downcast by the bucket impl; we erase it to
    /// `&dyn Any` here so the registry is heterogeneous.
    pub fn register<T: 'static>(&mut self, f: fn(&T) -> JsonValue) {
        let wrap = move |any: &dyn std::any::Any| -> Option<JsonValue> {
            // Downcast to the concrete `T` this closure was registered for. The
            // bucket impl only invokes this after looking the closure up by
            // `TypeId::of::<T>()`, so the `Any` is that `T` and the downcast
            // always succeeds — but degrade to `None` (the bucket then skips the
            // entry) instead of `.expect()`, honoring the no-panic rule for the
            // persistence path even if the TypeId invariant ever breaks.
            any.downcast_ref::<T>().map(f)
        };
        self.serializers.insert(TypeId::of::<T>(), Box::new(wrap));
    }

    /// Look up the serializer closure registered for the given data `TypeId`.
    pub(crate) fn get(&self, type_id: TypeId) -> Option<&SerializeFn> {
        self.serializers.get(&type_id)
    }

    /// Returns `true` if a serializer is registered for `type_id`.
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.serializers.contains_key(&type_id)
    }
}

/// Type-erased hydrate step: decode `JsonValue -> ()`, priming the live cache
/// via `client.set_query_data::<T, E>(key, value, cx)`. The closure captures
/// the concrete `T`/`E` in its monomorphized `register` call site, so it can
/// downcast/re-prime without the registry layer knowing the types.
type HydrateStep =
    Arc<dyn Fn(&mut QueryClient, &QueryKey, &JsonValue, &mut App) -> bool + Send + Sync>;

/// Registry of `serde_json::Value -> primed cache entry` steps, keyed by
/// `TypeId` of the resource's data type `T`. Used by [`hydrate`] to re-prime
/// erased on-disk values to concrete `T` so they can be handed to
/// `set_query_data`.
///
/// Each step returns `true` if it successfully decoded and primed the value,
/// `false` if the value was unparseable (the entry is then skipped).
#[derive(Default)]
pub struct DeserializerRegistry {
    steps: Vec<(TypeId, HydrateStep)>,
}

impl DeserializerRegistry {
    /// Register a deserializer for resources of type `(T, E)`.
    ///
    /// `deserialize` returns `Option<T>`; `None` means the value was
    /// unparseable and the entry is skipped during hydration. On `Some(t)`,
    /// `t` is primed into the cache via `set_query_data::<T, E>(key, t, cx)`.
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
        self.steps.push((TypeId::of::<(T, E)>(), Arc::new(step)));
    }

    /// Iterate every registered `(TypeId, hydrate-step)` pair. Used by
    /// [`hydrate`] to find which registry entry owns a given on-disk key.
    fn iter(&self) -> impl Iterator<Item = (TypeId, HydrateStep)> {
        self.steps.iter().map(|(k, v)| (*k, Arc::clone(v)))
    }
}

// ── Persister trait ──────────────────────────────────────────────────────

/// Async persistence backend for [`QueryClient::persist_with`].
///
/// Non-object-safe (methods return `impl Future`): the trait is consumed
/// generically by `persist_with<P: Persister>`, which monomorphizes the driver
/// around the concrete `P`. This avoids `Pin<Box<dyn Future>>` overhead and
/// keeps the `Send + 'static` bounds visible at the call site (the save future
/// runs on GPUI's `background_executor`, so it must be `Send + 'static`).
///
/// Implementations store cached data in any backend (filesystem, database,
/// KV store, …). See [`gpui_query_persist::FilePersister`] for a reference
/// disk adapter.
pub trait Persister: Send + Sync + 'static {
    /// Load the snapshot from storage.
    ///
    /// Implementations should be tolerant: a missing store yields an empty
    /// snapshot, a corrupt store yields an empty snapshot + a logged warning
    /// (or a typed [`PersistError`] for version mismatches the caller may wish
    /// to handle).
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
/// Holding the handle keeps the underlying [`CacheMutation`](super::CacheMutation)
/// observation (and thus the debounced save loop) alive; dropping it drops the
/// [`Subscription`], so no *new* saves are scheduled. A save task already
/// waiting on its debounce timer is detached and may still complete one final
/// save. The wrapped [`Persister`] is held in an `Arc` so the spawned save
/// future can use it after `persist_with` returns.
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
    /// Register a serializer for resources of type `(T, E)`.
    ///
    /// Only resources whose `T` has a registered serializer are emitted by the
    /// value-carrying `collect_persistable_into` path; unregistered types fall
    /// back to metadata-only (skipped), matching the legacy `dehydrate`.
    ///
    /// `f` is a `fn(&T) -> serde_json::Value` (a plain function pointer, not a
    /// closure) so it is `Send + Sync + 'static` withoutboxing overhead.
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
    /// **Strict-deserializer contract.** [`hydrate`] offers every on-disk entry
    /// to *every* registered deserializer (there is no type discriminator on
    /// [`PersistedEntry`], so routing is by trial). A deserializer MUST return
    /// `None` for any JSON shape it does not recognize as its own `T`; only
    /// return `Some` for values that genuinely decode to `T`. A lax
    /// deserializer that accepts a foreign shape would mis-prime the wrong
    /// bucket. (Each `(T, E)` writes to its own bucket, so typed data is not
    /// clobbered, but a permissive decoder wastes work and can prime a stale
    /// value.) Keep deserializers strict and cheap.
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

    /// Collect a value-carrying snapshot from the live cache, honoring `filter`
    /// and `max_age`. Only resources with a registered serializer (and in
    /// `Success` status) are included.
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

        // Enrich each collected entry with any opaque metadata recorded for its
        // key at fetch-completion time (see QueryClient::record_meta), so HTTP
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
    /// On every `CacheMutation` bump, the callback:
    /// 1. collects a fresh [`PersistSnapshot`] (cheap; main thread, has `&App`),
    /// 2. stashes it in a shared slot, replacing any pending snapshot,
    /// 3. spawns a debounced task that, after `opts.debounce`, takes the latest
    ///    snapshot from the slot and runs `persister.save(&snapshot)` on the
    ///    background executor.
    ///
    /// Rapid bursts coalesce: only the most recently collected snapshot is
    /// saved when the debounce timer elapses. Returning the [`PersistHandle`]
    /// keeps the observation alive; dropping it stops further saves.
    pub fn persist_with<P: Persister>(
        &self,
        persister: P,
        opts: PersistOptions,
        cx: &mut App,
    ) -> PersistHandle {
        let persister: Arc<P> = Arc::new(persister);
        let debounce = opts.debounce;
        // Shared slot for the latest pending snapshot. Replaced on every bump;
        // drained by the debounced save task.
        let pending: Arc<std::sync::Mutex<Option<PersistSnapshot>>> =
            Arc::new(std::sync::Mutex::new(None));

        // Bound on in-flight debounce tasks: at most one pending per window.
        // A bump that arrives while a task is already armed skips spawning a
        // new one (its snapshot still lands in `pending`, where the armed task
        // will drain it), so a burst produces one task rather than N. Cleared
        // by the task after it drains (or finds empty) the slot — on every
        // path, so persistence can never get stuck never-spawning-again.
        let armed: Arc<std::sync::Mutex<bool>> = Arc::new(std::sync::Mutex::new(false));

        // Ensure the marker exists before observing. The bump sites (see
        // `mutation_signal.rs`) call `cx.default_global::<CacheMutation>()`,
        // which — like `set_global`/`global_mut` — pushes a
        // `NotifyGlobalObservers` effect; that notification is what wakes this
        // observer. We seed the marker here too so it is guaranteed present
        // before observation is registered (the idempotent seeding itself also
        // notifies, harmlessly).
        let _ = cx.default_global::<super::CacheMutation>();

        let subscription = {
            let persister = persister.clone();
            let pending = pending.clone();
            let armed = armed.clone();
            let filter = opts.filter;
            let max_age = opts.max_age;
            let bg = cx.background_executor().clone();
            cx.observe_global::<super::CacheMutation>(move |cx| {
                // Collect fresh snapshot on the main thread (has &App).
                let snapshot = cx.update_global::<QueryClient, _>(|client, cx| {
                    client.collect_persist_snapshot(&filter, max_age, cx)
                });
                // Stash as the latest pending snapshot.
                if let Ok(mut slot) = pending.lock() {
                    *slot = Some(snapshot);
                }
                // Spawn a debounced save only if no task is already armed for
                // this window; otherwise let the in-flight task drain the slot
                // we just stashed (latest snapshot wins).
                {
                    let Ok(mut guard) = armed.lock() else {
                        return;
                    };
                    if *guard {
                        return;
                    }
                    *guard = true;
                }
                let persister = persister.clone();
                let pending = pending.clone();
                let armed = armed.clone();
                let bg_for_future = bg.clone();
                bg.spawn(async move {
                    if !debounce.is_zero() {
                        bg_for_future.timer(debounce).await;
                    }
                    // Take the latest snapshot (or no-op if a later task
                    // already drained the slot). Clear `armed` on every path so
                    // the next bump can spawn again — do it after draining so a
                    // bump that lands during the window still coalesces into
                    // this task's drain.
                    let snapshot = pending.lock().ok().and_then(|mut slot| slot.take());
                    if let Ok(mut guard) = armed.lock() {
                        *guard = false;
                    }
                    let Some(snapshot) = snapshot else { return };
                    if let Err(err) = persister.save(&snapshot).await {
                        #[cfg(debug_assertions)]
                        eprintln!("persist_with: save failed: {err}");
                    }
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
/// Useful as a default, for tests that only exercise the dirty-signal/debounce
/// path, or as a base to compose with a real persister behind a feature flag.
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

/// Load a snapshot from `persister` and re-prime the live cache with it.
///
/// This is the value-carrying counterpart to the (metadata-only)
/// [`QueryClient::hydrate`](super::QueryClient::hydrate). For each on-disk
/// entry whose `(T, E)` has a registered deserializer (see
/// [`QueryClient::register_deserializer`]), the JSON `value` is decoded and
/// primed via `set_query_data::<T, E>`. Entries without a registered
/// deserializer are skipped (the caller can still inspect them via the
/// returned [`PersistSnapshot`] for ad-hoc typed priming, matching the legacy
/// `hydrate` escape hatch).
///
/// Entries older than `max_age` or excluded by `filter` are skipped.
///
/// **Routing.** There is no type discriminator on [`PersistedEntry`], so every
/// surviving entry is offered to every registered deserializer (O(deserializers
/// × entries)); each is primed by the first deserializer that decodes it. This
/// relies on the strict-deserializer contract of
/// [`QueryClient::register_deserializer`] — keep deserializers strict.
///
/// Returns the loaded snapshot (post-filter) so callers can perform additional
/// metadata-only priming or diagnostics. Errors from the persister's `load`
/// propagate.
pub async fn hydrate<P: Persister>(
    client: &mut QueryClient,
    persister: &P,
    filter: &PersistFilter,
    max_age: Duration,
    cx: &mut App,
) -> Result<PersistSnapshot, PersistError> {
    let snapshot = persister.load().await?;
    // If the persister already enforces version, we still double-check here so
    // an in-memory persister can't silently feed a mismatched snapshot.
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

    // Clone the step list out (cheap `Arc` bumps) so we drop the immutable
    // borrow on `client` before calling `step(client, …)` which needs
    // `&mut QueryClient` (it calls `set_query_data`).
    let steps: Vec<HydrateStep> = deserializers.iter().map(|(_, s)| s).collect();

    // For each registered (T, E) hydrate-step, walk the snapshot entries and
    // let the step decode + prime any matching key. Because each step is
    // monomorphized over concrete (T, E), it downcasts safely inside its own
    // closure — no cross-type confusion.
    for step in steps {
        for (key_path, entry) in &snapshot.entries {
            let key = QueryKey::from(key_path.as_str());
            if !filter.matches(&key) {
                continue;
            }
            if max_age_ms > 0 && now_ms.saturating_sub(entry.cached_at) > max_age_ms {
                continue;
            }
            step(client, &key, &entry.value, cx);
        }
    }

    Ok(snapshot)
}
