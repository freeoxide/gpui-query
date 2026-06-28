//! Infinite query, mutation, and bulk operations on `QueryClient`.
//!
//! This module contains `impl QueryClient` methods for:
//! - Infinite query resource management and lookups
//! - Mutation registration and lookups
//! - Bulk operations (invalidate/reset/remove/cancel) across all bucket types

use std::any::TypeId;

use gpui::{App, Entity};

use crate::client::infinite_bucket::InfiniteQueryBucket;
use crate::client::mutation_bucket::MutationBucket;
use crate::core::{
    CachePolicy, InfiniteQueryResource, MutationResource, QueryKey, QueryKeyFilter, RequestPolicy,
};

use super::QueryClient;

impl QueryClient {
    // ── Infinite query operations ───────────────────────────────────────

    /// Get or create an infinite query resource for the given key and type pair.
    pub fn infinite_resource<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: impl Into<QueryKey>,
        cx: &mut App,
    ) -> Entity<InfiniteQueryResource<T, E>> {
        self.infinite_resource_with_policies::<T, E>(
            key,
            self.default_cache_policy,
            self.default_request_policy,
            cx,
        )
    }

    /// Get or create an infinite query resource with explicit policies.
    ///
    /// Audit 3 fix (findings 3, 4): Graceful downcast recovery.
    pub fn infinite_resource_with_policies<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Entity<InfiniteQueryResource<T, E>> {
        let type_id = TypeId::of::<(T, E)>();
        let bucket = self
            .infinite_buckets
            .entry(type_id)
            .or_insert_with(|| Box::new(InfiniteQueryBucket::<T, E>::new()));

        // M4: single downcast via the shared helper (redundant TypeId
        // pre-check dropped).
        let typed = Self::infinite_bucket_or_recreate::<T, E>(bucket);
        let entity = typed.get_or_create(key.into(), cache_policy, request_policy, cx);
        // Audit fix CL1/#105: opportunistically run GC on this op.
        self.maybe_opportunistic_gc(cx);
        entity
    }

    /// Get a specific infinite query entity by key.
    pub fn infinite_query<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
        key: &QueryKey,
    ) -> Option<Entity<InfiniteQueryResource<T, E>>> {
        let type_id = TypeId::of::<(T, E)>();
        self.infinite_buckets
            .get(&type_id)
            .and_then(|b| b.as_any().downcast_ref::<InfiniteQueryBucket<T, E>>())
            .and_then(|b| b.get(key))
    }

    /// Use the infinite query bucket's co-located sequencer to generate a
    /// `RequestId` for an infinite query key.
    ///
    /// Returns `None` if no bucket entry exists for the key. The sequencer is
    /// advanced in-place so subsequent calls produce monotonically increasing IDs.
    /// This is the infinite query equivalent of [`next_request_id_for_key`](Self::next_request_id_for_key).
    ///
    /// Audit 3 fix (findings 3, 4): Graceful downcast recovery.
    pub fn next_request_id_for_infinite_key<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: &QueryKey,
    ) -> Option<crate::core::RequestId> {
        let type_id = TypeId::of::<(T, E)>();
        let bucket = self.infinite_buckets.get_mut(&type_id)?;
        // M4: single downcast via the shared helper (redundant TypeId
        // pre-check dropped).
        let typed = Self::infinite_bucket_or_recreate::<T, E>(bucket);
        typed.sequencer_mut(key).map(|seq| seq.next_request())
    }

    /// Get all infinite query entities of a given type pair.
    pub fn all_infinite_queries<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &self,
    ) -> Vec<Entity<InfiniteQueryResource<T, E>>> {
        let type_id = TypeId::of::<(T, E)>();
        self.infinite_buckets
            .get(&type_id)
            .and_then(|b| b.as_any().downcast_ref::<InfiniteQueryBucket<T, E>>())
            .map(|b| b.all_entities())
            .unwrap_or_default()
    }

    // ── Mutation operations ─────────────────────────────────────────────

    /// Register a mutation entity.
    ///
    /// Audit 3 fix (findings 3, 4): Graceful downcast recovery.
    pub fn register_mutation<
        V: Clone + Send + Sync + 'static,
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        entity: &Entity<MutationResource<V, T, E>>,
        cx: &App,
    ) {
        let type_id = TypeId::of::<(V, T, E)>();
        let bucket = self
            .mutation_buckets
            .entry(type_id)
            .or_insert_with(|| Box::new(MutationBucket::<V, T, E>::new()));

        // M6: cache now_ms once and thread it into `insert` (avoids a second
        // `current_time_ms` syscall inside `insert`); the same value is reused
        // by `maybe_opportunistic_gc` below.
        let now_ms = crate::client::time::current_time_ms();
        // M4: single downcast via the shared helper (redundant TypeId
        // pre-check dropped).
        let typed = Self::mutation_bucket_or_recreate::<V, T, E>(bucket);
        typed.insert(entity, now_ms, cx);
        // Audit fix CL1/#105: opportunistically run GC on this op so
        // completed mutations are eventually evicted without manual gc() calls.
        self.maybe_opportunistic_gc(cx);
    }

    /// Get all mutation entities of a given type triple.
    pub fn all_mutations<
        V: Clone + Send + Sync + 'static,
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &self,
    ) -> Vec<Entity<MutationResource<V, T, E>>> {
        let type_id = TypeId::of::<(V, T, E)>();
        self.mutation_buckets
            .get(&type_id)
            .and_then(|b| b.as_any().downcast_ref::<MutationBucket<V, T, E>>())
            .map(|b| b.all_entities())
            .unwrap_or_default()
    }

    // ── Bulk operations ─────────────────────────────────────────────────

    /// Apply an operation `f` to every query bucket (regular + infinite).
    /// **L10**: extracted to kill the 4x duplicated
    /// `for buckets … for infinite_buckets …` pair in the bulk-op methods
    /// below. `f` is called once per regular bucket (as `Left`) and once per
    /// infinite bucket (as `Right`); callers match on the side to invoke the
    /// correct trait method.
    fn for_each_query_bucket_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(EitherBucket<'_>),
    {
        for bucket in self.buckets.values_mut() {
            f(EitherBucket::Query(bucket.as_mut()));
        }
        for bucket in self.infinite_buckets.values_mut() {
            f(EitherBucket::Infinite(bucket.as_mut()));
        }
    }

    /// Invalidate queries matching the filter.
    ///
    /// Uses collect-then-update pattern to avoid nested entity borrows.
    pub fn invalidate_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.invalidate_matching(filter, cx),
            EitherBucket::Infinite(b) => b.invalidate_matching(filter, cx),
        });
    }

    /// Reset queries matching the filter.
    pub fn reset_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.reset_matching(filter, cx),
            EitherBucket::Infinite(b) => b.reset_matching(filter, cx),
        });
    }

    /// Remove queries matching the filter.
    pub fn remove_queries(&mut self, filter: &QueryKeyFilter) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.remove_matching(filter),
            EitherBucket::Infinite(b) => b.remove_matching(filter),
        });
    }

    /// Cancel in-flight requests matching the filter (Audit 3, Finding 5).
    ///
    /// Iterates all query and infinite query buckets, finds resources with active
    /// requests, and cancels them with a [`QueryError::cancelled`] error. This is
    /// essential for cleanup when navigating away from a page or when bulk
    /// cancellation is needed.
    ///
    /// Equivalent to TanStack Query's `queryClient.cancelQueries()`. Individual
    /// `QueryResource::cancel()` exists but this is the bulk cancellation method
    /// on the client.
    pub fn cancel_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.cancel_matching(filter, cx),
            EitherBucket::Infinite(b) => b.cancel_matching(filter, cx),
        });
    }

    // ── Erased-bucket recovery helpers (M4) ──────────────────────────────
    //
    // Mirrors `QueryClient::bucket_or_recreate` in `mod.rs` for the infinite
    // and mutation maps: downcast once (the redundant TypeId pre-check is
    // dropped — `downcast_mut` checks it internally) and recreate the bucket
    // in place on the (impossible) mismatch. Kills the 3x duplicated recovery
    // blocks that lived in `infinite_resource_with_policies`,
    // `next_request_id_for_infinite_key`, and `register_mutation`.

    fn infinite_bucket_or_recreate<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        bucket: &mut Box<dyn super::erased::ErasedInfiniteBucket>,
    ) -> &mut InfiniteQueryBucket<T, E> {
        if bucket
            .as_any_mut()
            .downcast_mut::<InfiniteQueryBucket<T, E>>()
            .is_none()
        {
            eprintln!(
                "QueryClient: type mismatch in infinite bucket downcast for {}. \
                 Replacing with a fresh bucket.",
                std::any::type_name::<(T, E)>()
            );
            *bucket = Box::new(InfiniteQueryBucket::<T, E>::new());
        }
        bucket
            .as_any_mut()
            .downcast_mut::<InfiniteQueryBucket<T, E>>()
            .expect("InfiniteQueryBucket downcast succeeds after infinite_bucket_or_recreate")
    }

    fn mutation_bucket_or_recreate<
        V: Clone + Send + Sync + 'static,
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        bucket: &mut Box<dyn super::erased::ErasedMutationBucket>,
    ) -> &mut MutationBucket<V, T, E> {
        if bucket
            .as_any_mut()
            .downcast_mut::<MutationBucket<V, T, E>>()
            .is_none()
        {
            eprintln!(
                "QueryClient: type mismatch in mutation bucket downcast for {}. \
                 Replacing with a fresh bucket.",
                std::any::type_name::<(V, T, E)>()
            );
            *bucket = Box::new(MutationBucket::<V, T, E>::new());
        }
        bucket
            .as_any_mut()
            .downcast_mut::<MutationBucket<V, T, E>>()
            .expect("MutationBucket downcast succeeds after mutation_bucket_or_recreate")
    }
}

/// One side of a query bucket iteration (L10).
enum EitherBucket<'a> {
    Query(&'a mut dyn crate::client::erased::ErasedBucket),
    Infinite(&'a mut dyn crate::client::erased::ErasedInfiniteBucket),
}
