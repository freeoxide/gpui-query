//! Layer 1: GPUI `QueryClient` — global registry for query resources.
//!
//! `QueryClient` is a GPUI [`Global`] that manages type-partitioned buckets
//! for queries, mutations, and observers. It provides bulk operations like
//! `invalidate_queries`, `cancel_queries`, and garbage collection.
//!
//! # Audit 3 fixes
//!
//! - `gc()` accepts optional `now_ms` parameter via `gc_with_time()` to avoid
//!   redundant syscalls (finding 2)
//! - `expect()` on TypeId downcast replaced with graceful recovery + type name
//!   in error message (findings 3, 4)
//! - `cancel_queries()` added for bulk in-flight request cancellation (finding 5)
//! - `get_query_data()` / `set_query_data()` for ergonomic cache access (finding 6)
//! - `diagnostics()` now populates per-resource diagnostic details (finding 7)
//! - `dehydrate()` / `hydrate()` for state serialization across restarts (finding 8)
//! - `QueryPersister` trait and `persist()` / `restore()` for pluggable persistence (finding 9)
//! - `fetch_query()` for imperative one-shot fetches (finding 10)
//! - `prefetch_query()` for background cache warming (finding 11)

mod bucket;
mod devtools;
mod erased;
mod infinite_bucket;
mod infinite_mutation_ops;
mod lifecycle;
mod mutation_bucket;
#[cfg(feature = "persist")]
mod mutation_signal;
mod observer;
#[cfg(feature = "persist")]
mod persist;
mod prepared_fetch;
mod time;

pub use bucket::QueryBucket;
pub use devtools::{ClientDiagnostic, MutationDiagnostic, QueryDiagnostic};
#[cfg(feature = "persist")]
pub use devtools::{DehydratedEntry, DehydratedState};
#[cfg(feature = "persist")]
pub use erased::QueryPersister;
pub use infinite_bucket::InfiniteQueryBucket;
pub use mutation_bucket::MutationBucket;
#[cfg(feature = "persist")]
pub use mutation_signal::CacheMutation;
pub use observer::{
    InfiniteQueryObserver, MutationObserver, ObservableResource, Observer, ObserverConfig,
    QueryObserver,
};
#[cfg(feature = "persist")]
pub use persist::{
    NoopPersister, PERSIST_VERSION, PersistError, PersistFilter, PersistHandle, PersistOptions,
    PersistSnapshot, PersistedEntry, Persister, SerializerRegistry, hydrate,
};
pub use prepared_fetch::PreparedFetch;
pub use time::current_time_ms;

use std::any::TypeId;

use ahash::AHashMap;
use gpui::{App, Entity, Global};

use crate::client::bucket::shared::GC_INTERVAL;
use crate::client::bucket::types::MIN_GC_TIME_MS;
use crate::client::erased::{ErasedBucket, ErasedInfiniteBucket, ErasedMutationBucket};
use crate::core::{CachePolicy, QueryKey, QueryResource, RequestPolicy};

/// Global registry for query and mutation resources.
///
/// Implements [`Global`] so it can be set once with `cx.set_global(QueryClient::default())`
/// and accessed from any component via `cx.global::<QueryClient>()`.
///
/// # v2 Improvements
///
/// - `Default` impl (no required params)
/// - `AHashMap` for ~2x faster lookups on trusted keys
/// - Actual mutation GC (not a no-op)
/// - Collect-then-update pattern to avoid nested entity borrows
pub struct QueryClient {
    pub(crate) buckets: AHashMap<TypeId, Box<dyn ErasedBucket>>,
    pub(crate) infinite_buckets: AHashMap<TypeId, Box<dyn ErasedInfiniteBucket>>,
    pub(crate) mutation_buckets: AHashMap<TypeId, Box<dyn ErasedMutationBucket>>,
    pub(crate) default_cache_policy: CachePolicy,
    pub(crate) default_request_policy: RequestPolicy,
    pub(crate) gc_time_ms: u64,
    /// Typed-serializer registry for the value-carrying persistence path
    /// (`persist` feature). Populated by `register_serializer::<T, E>`.
    #[cfg(feature = "persist")]
    pub(crate) serializers: Option<crate::client::persist::SerializerRegistry>,
    /// Typed-deserializer registry for [`hydrate`] (`persist` feature).
    /// Populated by `register_deserializer::<T, E>`.
    #[cfg(feature = "persist")]
    pub(crate) deserializers: Option<crate::client::persist::DeserializerRegistry>,
    /// Per-key opaque metadata captured from `Fetched::meta` at fetch
    /// completion (`persist` feature), surfaced into
    /// [`PersistedEntry::meta`](crate::client::persist::PersistedEntry) at
    /// collect time so HTTP `CacheMeta` and similar can round-trip through a
    /// cold start. Entries for evicted keys are simply ignored at collect time.
    #[cfg(feature = "persist")]
    pub(crate) persisted_meta:
        Option<std::collections::HashMap<crate::core::QueryKey, serde_json::Value>>,
    /// Operation counter for opportunistic GC (audit CL1/#105). The GC
    /// subsystem fires every `GC_INTERVAL` resource/mutation operations so it
    /// actually runs in production without requiring hooks to call `gc()`.
    op_count: u64,
    /// Wall-clock ms of the last opportunistic GC sweep. Combined with the op
    /// counter, this debounces GC so a burst of insertions (or a fast test that
    /// creates many resources within `MIN_GC_TIME_MS`) does not trigger GC.
    ///
    /// **L11**: initialized to `0` (rather than `current_time_ms()`) so
    /// `QueryClient` construction does not perform a syscall. The
    /// `MIN_GC_TIME_MS` debounce in `maybe_opportunistic_gc` still suppresses
    /// GC for the first ~1s of life because the very first sweep sets this to
    /// the real clock on its way through.
    last_gc_ms: u64,
}

impl Global for QueryClient {}

impl Default for QueryClient {
    /// **Audit fix #21**: Explicit `Default` impl that sets `gc_time_ms` to
    /// `300_000` (5 minutes), matching `with_policies`. The previous derive
    /// produced `gc_time_ms: 0`, which silently disabled GC — every
    /// non-loading Idle/Failure resource would be evicted on every pass.
    /// All other field defaults are identical to what the derive produced.
    fn default() -> Self {
        Self {
            buckets: AHashMap::new(),
            infinite_buckets: AHashMap::new(),
            mutation_buckets: AHashMap::new(),
            default_cache_policy: CachePolicy::default(),
            default_request_policy: RequestPolicy::default(),
            gc_time_ms: 300_000,
            #[cfg(feature = "persist")]
            serializers: None,
            #[cfg(feature = "persist")]
            deserializers: None,
            #[cfg(feature = "persist")]
            persisted_meta: None,
            op_count: 0,
            last_gc_ms: 0,
        }
    }
}

impl QueryClient {
    /// Create a new client with default policies.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom default policies.
    pub fn with_policies(
        default_cache_policy: CachePolicy,
        default_request_policy: RequestPolicy,
    ) -> Self {
        Self {
            default_cache_policy,
            default_request_policy,
            gc_time_ms: 300_000, // 5 minutes
            ..Default::default()
        }
    }

    /// Set the garbage collection time (in milliseconds).
    ///
    /// Values below 1000ms are clamped to 1000ms during GC to prevent
    /// aggressive eviction of all Idle/Failure resources on every GC pass.
    pub fn with_gc_time(mut self, gc_time_ms: u64) -> Self {
        self.gc_time_ms = gc_time_ms;
        self
    }

    /// Record opaque metadata (e.g. a serialized HTTP `CacheMeta`) for `key`,
    /// captured from a fetcher's [`Fetched::meta`](crate::core::Fetched) at
    /// completion. Surfaced into
    /// [`PersistedEntry::meta`](crate::client::persist::PersistedEntry) at
    /// collect time so the metadata round-trips through persistence. `persist`
    /// feature only.
    #[cfg(feature = "persist")]
    pub(crate) fn record_meta(&mut self, key: crate::core::QueryKey, meta: serde_json::Value) {
        self.persisted_meta
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key, meta);
    }

    /// Opportunistic GC trigger (audit CL1/#105). Runs GC every `GC_INTERVAL`
    /// operations so the GC subsystem actually fires in production without
    /// requiring hooks to call `gc()` explicitly. Without this trigger the
    /// (now correct, live-state-reading) GC never runs in production, which
    /// would render the memory-bound fixes (#1, #2, #8, #91, #108) academic.
    ///
    /// Debounced by BOTH operation count (every `GC_INTERVAL` ops) and wall
    /// clock time (no sweep within `MIN_GC_TIME_MS` of the last). `gc_time_ms`
    /// of 0 disables GC entirely.
    ///
    /// **L11**: `last_gc_ms` starts at `0` (no `current_time_ms` syscall at
    /// construction). To preserve the "no GC in the first ~1s of life"
    /// debounce that the prior `current_time_ms()` initialization provided,
    /// the sentinel `0` is treated as "uninitialized": the first time
    /// `maybe_opportunistic_gc` reaches the time check, it seeds `last_gc_ms`
    /// to `now_ms` and skips that sweep, so a fast test that creates many
    /// resources in well under a second never triggers GC.
    fn maybe_opportunistic_gc(&mut self, cx: &App) {
        if self.gc_time_ms == 0 {
            return;
        }
        self.op_count = self.op_count.wrapping_add(1);
        if !self.op_count.is_multiple_of(GC_INTERVAL as u64) {
            return;
        }
        let now_ms = current_time_ms();
        // L11: seed the debounce window on first reach instead of syscalling
        // at construction.
        if self.last_gc_ms == 0 {
            self.last_gc_ms = now_ms;
            return;
        }
        if now_ms.saturating_sub(self.last_gc_ms) < MIN_GC_TIME_MS {
            return;
        }
        self.last_gc_ms = now_ms;
        self.gc_with_time(now_ms, cx);
    }

    // ── Query operations ────────────────────────────────────────────────

    /// Get or create a query resource for the given key and type pair.
    pub fn resource<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: impl Into<QueryKey>,
        cx: &mut App,
    ) -> Entity<QueryResource<T, E>> {
        self.resource_with_policies::<T, E>(
            key,
            self.default_cache_policy,
            self.default_request_policy,
            cx,
        )
    }

    /// Get or create a query resource with explicit policies.
    ///
    /// Audit 3 fix (findings 3, 4): Uses graceful downcast recovery instead
    /// of `expect()`. On type mismatch, logs the type name and creates a
    /// fresh bucket, preventing application crashes from hypothetical
    /// TypeId collisions.
    pub fn resource_with_policies<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Entity<QueryResource<T, E>> {
        let type_id = TypeId::of::<(T, E)>();
        let bucket = self
            .buckets
            .entry(type_id)
            .or_insert_with(|| Box::new(QueryBucket::<T, E>::new()));

        // M4: `bucket_or_recreate` downcasts once; `downcast_mut` already
        // performs the TypeId check internally, so the prior redundant
        // `bucket.type_id() != expected` pre-check is dropped (it was the
        // double-check that audit fix #11 left in). On the (impossible)
        // mismatch we log + swap in a fresh bucket + return it, all in one
        // place — killing the 5x duplicated recovery block across the client.
        let typed = Self::bucket_or_recreate::<T, E>(bucket);
        let entity = typed.get_or_create(key.into(), cache_policy, request_policy, cx);
        // Audit fix CL1/#105: opportunistically run GC on this op.
        self.maybe_opportunistic_gc(cx);
        entity
    }

    /// Get all query entities of a given type pair.
    pub fn all_queries<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
    ) -> Vec<Entity<QueryResource<T, E>>> {
        let type_id = TypeId::of::<(T, E)>();
        self.buckets
            .get(&type_id)
            .and_then(|b| b.as_any().downcast_ref::<QueryBucket<T, E>>())
            .map(|b| b.all_entities())
            .unwrap_or_default()
    }

    /// Get a specific query entity by key.
    pub fn query<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
        key: &QueryKey,
    ) -> Option<Entity<QueryResource<T, E>>> {
        let type_id = TypeId::of::<(T, E)>();
        self.buckets
            .get(&type_id)
            .and_then(|b| b.as_any().downcast_ref::<QueryBucket<T, E>>())
            .and_then(|b| b.get(key))
    }

    /// Use the bucket's co-located sequencer to generate a `RequestId` for a key.
    ///
    /// Returns `None` if no bucket entry exists for the key. The sequencer is
    /// advanced in-place (mutated) so subsequent calls produce monotonically
    /// increasing IDs. This is the fix for audit findings #1/#5/#15/#18:
    /// using the bucket's persistent sequencer instead of a transient one
    /// prevents every request from getting the same `RequestId(1, 1)`.
    ///
    /// Audit 3 fix (findings 3, 4): Graceful downcast recovery.
    pub fn next_request_id_for_key<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: &QueryKey,
    ) -> Option<crate::core::RequestId> {
        let type_id = TypeId::of::<(T, E)>();
        let bucket = self.buckets.get_mut(&type_id)?;
        // M4: single downcast via the shared helper (redundant TypeId
        // pre-check dropped).
        let typed = Self::bucket_or_recreate::<T, E>(bucket);
        typed.sequencer_mut(key).map(|seq| seq.next_request())
    }

    // ── Erased-bucket recovery helper (M4) ──────────────────────────────

    /// Downcast an erased query bucket to `&mut QueryBucket<T, E>`, recreating
    /// it in place on the (impossible) type mismatch.
    ///
    /// **M4**: this replaces the 5x duplicated `TypeId` pre-check, `eprintln`,
    /// fresh-bucket, and `downcast_mut` match block. `Any::downcast_mut` checks
    /// `TypeId` internally, so the explicit pre-check was redundant; we now
    /// downcast once and, only on the (impossible-after-construction) `None`,
    /// log, swap in a fresh typed bucket, and downcast *that* (which always
    /// succeeds). No production panic.
    fn bucket_or_recreate<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        bucket: &mut Box<dyn ErasedBucket>,
    ) -> &mut QueryBucket<T, E> {
        if bucket
            .as_any_mut()
            .downcast_mut::<QueryBucket<T, E>>()
            .is_none()
        {
            eprintln!(
                "QueryClient: type mismatch in bucket downcast for {}. \
                 Replacing with a fresh bucket.",
                std::any::type_name::<(T, E)>()
            );
            *bucket = Box::new(QueryBucket::<T, E>::new());
            debug_assert!(
                bucket
                    .as_any_mut()
                    .downcast_mut::<QueryBucket<T, E>>()
                    .is_some(),
                "QueryBucket downcast failed after fresh reconstruction"
            );
        }
        // Unwrap is infallible here: either the original downcast succeeded,
        // or we just replaced `*bucket` with a freshly-constructed typed one.
        bucket
            .as_any_mut()
            .downcast_mut::<QueryBucket<T, E>>()
            .expect("QueryBucket downcast succeeds after bucket_or_recreate")
    }

    // ── Data accessors (Audit 3, Finding 6) ─────────────────────────────

    /// Read the cached data for a query key directly, without going through a hook.
    ///
    /// Returns `None` if no resource exists for the key, the entity was collected,
    /// or the resource has no data (has not completed a fetch).
    ///
    /// This is the ergonomic equivalent of TanStack Query's `queryClient.getQueryData(key)`.
    pub fn get_query_data<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
        key: &QueryKey,
        cx: &App,
    ) -> Option<T> {
        let entity = self.query::<T, E>(key)?;
        entity.read_with(cx, |resource, _| resource.data().cloned())
    }

    /// Read the cached data for a query key via a borrow callback, with NO
    /// clone of `T` (audit fix #L12).
    ///
    /// This is the zero-clone counterpart to [`get_query_data`](Self::get_query_data):
    /// instead of returning `Option<T>` (which clones the value out of the
    /// resource), it hands `f` a `&T` for the duration of the call. Use this
    /// when the caller only needs to *inspect* the cached data (e.g. compute a
    /// derived value, render a summary) and would otherwise pay for a full
    /// `T::clone()` it discards immediately.
    ///
    /// Returns `None` if no resource exists for the key, the entity was
    /// collected, or the resource has no data. Returns `Some(R)` (the value
    /// produced by `f`) otherwise. `T` and `E` are unchanged from
    /// `get_query_data`; `R` is the closure's return type and is independent of
    /// `T`, so it does not shadow the crate's `T`/`E` conventions.
    pub fn with_query_data<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
        R,
    >(
        &self,
        key: &QueryKey,
        cx: &App,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        let entity = self.query::<T, E>(key)?;
        entity.read_with(cx, |resource, _| resource.data().map(f))
    }

    /// Write data directly into the cache for a query key, creating the resource
    /// if it does not already exist.
    ///
    /// This is the ergonomic equivalent of TanStack Query's `queryClient.setQueryData(key, data)`.
    /// The resource's previous data is saved for rollback via `rollback_to_previous()`.
    /// The data is set via `set_data()` which saves previous data but does not
    /// change the resource's status or timestamp. Use this for optimistic updates
    /// and manual cache manipulation where you control the lifecycle.
    pub fn set_query_data<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: impl Into<QueryKey>,
        data: T,
        cx: &mut App,
    ) {
        let key = key.into();
        let entity = self.resource::<T, E>(key, cx);
        entity.update(cx, |resource, cx| {
            resource.set_data(data);
            // B2: bump the precise dirty signal so `persist_with` schedules a
            // save. `default_global` creates the marker if absent AND pushes
            // GPUI's `NotifyGlobalObservers` effect (see gpui `App::default_global`),
            // which wakes the `observe_global::<CacheMutation>` observer in
            // `persist_with`. It is infallible, so the no-`persist_with` build's
            // `set_query_data` path never panics on an absent marker.
            #[cfg(feature = "persist")]
            cx.default_global::<crate::client::CacheMutation>();
            // In the default (no-persist) build the closure's `cx` is otherwise
            // unused; reference it so the build stays warning-free.
            #[cfg(not(feature = "persist"))]
            let _ = cx;
        });
    }
}
