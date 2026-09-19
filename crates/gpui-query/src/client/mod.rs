//! GPUI `QueryClient`: a [`Global`] registry managing type-partitioned
//! buckets for queries, mutations, and observers, with bulk operations
//! (invalidation, cancellation, GC) on top.

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
/// Implements [`Global`] so it can be set once with
/// `cx.set_global(QueryClient::default())` and accessed from any component
/// via `cx.global::<QueryClient>()`.
pub struct QueryClient {
    pub(crate) buckets: AHashMap<TypeId, Box<dyn ErasedBucket>>,
    pub(crate) infinite_buckets: AHashMap<TypeId, Box<dyn ErasedInfiniteBucket>>,
    pub(crate) mutation_buckets: AHashMap<TypeId, Box<dyn ErasedMutationBucket>>,
    pub(crate) default_cache_policy: CachePolicy,
    pub(crate) default_request_policy: RequestPolicy,
    pub(crate) gc_time_ms: u64,
    /// Typed-serializer registry for the value-carrying persistence path
    /// (`persist` feature), populated by `register_serializer::<T, E>`.
    #[cfg(feature = "persist")]
    pub(crate) serializers: Option<crate::client::persist::SerializerRegistry>,
    /// Typed-deserializer registry for [`hydrate`] (`persist` feature),
    /// populated by `register_deserializer::<T, E>`.
    #[cfg(feature = "persist")]
    pub(crate) deserializers: Option<crate::client::persist::DeserializerRegistry>,
    /// Opaque per-key metadata captured from `Fetched::meta` at fetch
    /// completion, surfaced into `PersistedEntry::meta` at collect time so
    /// HTTP `CacheMeta` and similar round-trip through a cold start. Pruned
    /// of evicted keys by GC.
    #[cfg(feature = "persist")]
    pub(crate) persisted_meta:
        Option<std::collections::HashMap<crate::core::QueryKey, serde_json::Value>>,
    /// Operation counter driving opportunistic GC every `GC_INTERVAL` ops.
    op_count: u64,
    /// Wall-clock ms of the last GC sweep; GC runs at most once per
    /// `MIN_GC_TIME_MS`. `0` means "not yet seeded" (avoids a syscall at
    /// construction; the first reach seeds it and skips that sweep).
    last_gc_ms: u64,
}

impl Global for QueryClient {}

impl Default for QueryClient {
    /// `gc_time_ms` defaults to 300_000 (5 minutes), matching
    /// [`with_policies`](Self::with_policies).
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
    /// A value of 0 disables GC entirely.
    pub fn with_gc_time(mut self, gc_time_ms: u64) -> Self {
        self.gc_time_ms = gc_time_ms;
        self
    }

    /// Record opaque metadata (e.g. a serialized HTTP `CacheMeta`) for `key`,
    /// captured from a fetcher's `Fetched::meta` at completion and surfaced
    /// into `PersistedEntry::meta` so it round-trips through persistence.
    /// `persist` feature only.
    #[cfg(feature = "persist")]
    pub(crate) fn record_meta(&mut self, key: crate::core::QueryKey, meta: serde_json::Value) {
        self.persisted_meta
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key, meta);
    }

    /// GC trigger for resource-creating ops: runs GC every `GC_INTERVAL`
    /// operations, at most once per `MIN_GC_TIME_MS`, so the GC subsystem
    /// fires in production without hooks calling `gc()` explicitly.
    /// `gc_time_ms` of 0 disables GC entirely.
    fn maybe_opportunistic_gc(&mut self, cx: &App) {
        if self.gc_time_ms == 0 {
            return;
        }
        self.op_count = self.op_count.wrapping_add(1);
        if !self.op_count.is_multiple_of(GC_INTERVAL as u64) {
            return;
        }
        let now_ms = current_time_ms();
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
    /// A bucket downcast mismatch (impossible while `TypeId` keys are
    /// sound) replaces the bucket instead of panicking.
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

        let typed = Self::bucket_or_recreate::<T, E>(bucket);
        let entity = typed.get_or_create(key.into(), cache_policy, request_policy, cx);
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

    /// Use the bucket's co-located sequencer to generate a `RequestId` for a
    /// key. Returns `None` if no bucket entry exists for the key. The
    /// sequencer is persistent, so IDs stay monotonic for the entry's
    /// lifetime.
    pub fn next_request_id_for_key<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: &QueryKey,
    ) -> Option<crate::core::RequestId> {
        let type_id = TypeId::of::<(T, E)>();
        let bucket = self.buckets.get_mut(&type_id)?;
        let typed = Self::bucket_or_recreate::<T, E>(bucket);
        typed.sequencer_mut(key).map(|seq| seq.next_request())
    }

    // ── Erased-bucket recovery helper ───────────────────────────────────

    /// Downcast an erased bucket to `&mut QueryBucket<T, E>`. On a mismatch
    /// (unreachable while `TypeId` keys are sound) the bucket is replaced
    /// with a fresh typed one rather than panicking.
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
        }
        // Infallible: either the original downcast succeeded, or we just
        // replaced the bucket with a freshly constructed typed one.
        bucket
            .as_any_mut()
            .downcast_mut::<QueryBucket<T, E>>()
            .expect("QueryBucket downcast succeeds after bucket_or_recreate")
    }

    // ── Data accessors ──────────────────────────────────────────────────

    /// Read the cached data for a query key directly, without going through
    /// a hook. Returns `None` if no resource exists for the key, the entity
    /// was collected, or the resource has not completed a fetch.
    ///
    /// The ergonomic equivalent of TanStack Query's
    /// `queryClient.getQueryData(key)`.
    pub fn get_query_data<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
        key: &QueryKey,
        cx: &App,
    ) -> Option<T> {
        let entity = self.query::<T, E>(key)?;
        entity.read_with(cx, |resource, _| resource.data().cloned())
    }

    /// Read the cached data via a borrow callback, with no clone of `T`.
    ///
    /// The zero-clone counterpart to [`get_query_data`](Self::get_query_data):
    /// `f` receives `&T` for the duration of the call, for callers that only
    /// inspect the data and would discard a full `T::clone()`. Returns
    /// `None` under the same conditions as `get_query_data`.
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

    /// Write data directly into the cache for a query key, creating the
    /// resource if it does not already exist. The previous data is saved for
    /// rollback via `rollback_to_previous()`. The write does not change the
    /// resource's status or timestamp.
    ///
    /// The ergonomic equivalent of TanStack Query's
    /// `queryClient.setQueryData(key, data)`.
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
            // Bump the dirty signal so `persist_with` schedules a save.
            // `default_global` seeds the marker if absent and pushes GPUI's
            // NotifyGlobalObservers effect, which the `persist_with` driver
            // observes; it is infallible.
            #[cfg(feature = "persist")]
            cx.default_global::<crate::client::CacheMutation>();
            #[cfg(not(feature = "persist"))]
            let _ = cx;
        });
    }
}
