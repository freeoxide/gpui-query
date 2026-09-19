//! Infinite query, mutation, and bulk operations on `QueryClient`.

use std::any::TypeId;

use gpui::{App, Entity};

use crate::client::infinite_bucket::InfiniteQueryBucket;
use crate::client::mutation_bucket::MutationBucket;
use crate::core::{
    CachePolicy, InfiniteQueryResource, MutationResource, QueryKey, QueryKeyFilter, RequestPolicy,
};

use super::QueryClient;

impl QueryClient {
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

        let typed = Self::infinite_bucket_or_recreate::<T, E>(bucket);
        let entity = typed.get_or_create(key.into(), cache_policy, request_policy, cx);
        self.maybe_opportunistic_gc(cx);
        entity
    }

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

    pub fn next_request_id_for_infinite_key<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: &QueryKey,
    ) -> Option<crate::core::RequestId> {
        let type_id = TypeId::of::<(T, E)>();
        let bucket = self.infinite_buckets.get_mut(&type_id)?;
        let typed = Self::infinite_bucket_or_recreate::<T, E>(bucket);
        typed.sequencer_mut(key).map(|seq| seq.next_request())
    }

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

        let now_ms = crate::client::time::current_time_ms();
        let typed = Self::mutation_bucket_or_recreate::<V, T, E>(bucket);
        typed.insert(entity, now_ms, cx);
        self.maybe_opportunistic_gc(cx);
    }

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

    /// Data is kept but marked stale.
    pub fn invalidate_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.invalidate_matching(filter, cx),
            EitherBucket::Infinite(b) => b.invalidate_matching(filter, cx),
        });
    }

    /// Data and status are cleared.
    pub fn reset_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.reset_matching(filter, cx),
            EitherBucket::Infinite(b) => b.reset_matching(filter, cx),
        });
    }

    /// Entries are removed from the cache entirely.
    pub fn remove_queries(&mut self, filter: &QueryKeyFilter) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.remove_matching(filter),
            EitherBucket::Infinite(b) => b.remove_matching(filter),
        });
    }

    /// Cancels matching in-flight requests with a
    /// [`QueryError::cancelled`](crate::core::QueryError::cancelled) error;
    /// TanStack `queryClient.cancelQueries()`.
    pub fn cancel_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_query_bucket_mut(|b| match b {
            EitherBucket::Query(b) => b.cancel_matching(filter, cx),
            EitherBucket::Infinite(b) => b.cancel_matching(filter, cx),
        });
    }

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

enum EitherBucket<'a> {
    Query(&'a mut dyn crate::client::erased::ErasedBucket),
    Infinite(&'a mut dyn crate::client::erased::ErasedInfiniteBucket),
}
