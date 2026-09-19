//! Type-partitioned bucket for infinite query resources.
//!
//! Shares its machinery with [`QueryBucket`] through
//! [`ResourceBucket`](super::bucket::shared::ResourceBucket); only the erased
//! trait impl and the first-page persistence path are specific to infinite
//! queries.

use gpui::{App, Entity};

use crate::core::{
    CachePolicy, InfiniteQueryResource, QueryKey, QueryKeyFilter, RequestPolicy,
    RequestSequencer,
};

use super::bucket::shared::ResourceBucket;
use super::devtools::QueryDiagnostic;
use super::ErasedInfiniteBucket;

/// Type-partitioned storage for infinite query resources of a specific `(T, E)` type pair.
pub struct InfiniteQueryBucket<T, E> {
    entries: ResourceBucket<InfiniteQueryResource<T, E>>,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> InfiniteQueryBucket<T, E> {
    pub(crate) fn new() -> Self {
        Self {
            entries: ResourceBucket::new(),
        }
    }

    pub(crate) fn get_or_create(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Entity<InfiniteQueryResource<T, E>> {
        self.entries.get_or_create(key, cache_policy, request_policy, cx)
    }

    pub(crate) fn get(&self, key: &QueryKey) -> Option<Entity<InfiniteQueryResource<T, E>>> {
        self.entries.get(key)
    }

    pub(crate) fn sequencer_mut(&mut self, key: &QueryKey) -> Option<&mut RequestSequencer> {
        self.entries.sequencer_mut(key)
    }

    pub(crate) fn all_entities(&self) -> Vec<Entity<InfiniteQueryResource<T, E>>> {
        self.entries.all_entities()
    }
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> ErasedInfiniteBucket
    for InfiniteQueryBucket<T, E>
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        self.entries.gc(now_ms, gc_time_ms, cx);
    }

    fn count(&self) -> usize {
        self.entries.entries.len()
    }

    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.entries.for_each_matching_entry(filter, cx, |entity, cx| {
            // Skip the notify when last_updated_at is already None (invalidate
            // only clears that one field).
            let needs_invalidate =
                entity.read_with(cx, |r, _| r.last_updated_at_ms().is_some());
            if needs_invalidate {
                entity.update(cx, |resource, _| resource.invalidate());
            }
        });
    }

    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.entries.for_each_matching_entry(filter, cx, |entity, cx| {
            entity.update(cx, |resource, _| resource.reset());
        });
    }

    fn remove_matching(&mut self, filter: &QueryKeyFilter) {
        self.entries.entries.retain(|k, _| !filter.matches(k));
    }

    /// Gate on the authoritative `is_loading()` read; see
    /// `QueryBucket::cancel_matching`. Also bumps `ignored_results` so
    /// cancelled infinite fetches match the regular query path.
    fn cancel_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.entries.for_each_matching_entry(filter, cx, |entity, cx| {
            if entity.read_with(cx, |r, _| r.is_loading()) {
                entity.update(cx, |resource, _| {
                    if let Some(signal) = resource.signal() {
                        signal.cancel();
                    }
                    resource.mark_ignored_result();
                });
            }
        });
    }

    fn collect_diagnostics_into(&self, now_ms: u64, cx: &App, out: &mut Vec<QueryDiagnostic>) {
        self.entries.collect_diagnostics_into(now_ms, cx, out);
    }

    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &App, out: &mut Vec<(String, crate::core::QueryStatus)>) {
        self.entries.collect_key_status_into(cx, out);
    }

    #[cfg(feature = "persist")]
    fn contains_key(&self, key: &crate::core::QueryKey) -> bool {
        self.entries.entries.contains_key(key)
    }

    /// Persists the first page only; the full page vector is opaque here.
    /// Entries without a registered serializer, or not in `Success`, are
    /// skipped.
    #[cfg(feature = "persist")]
    fn collect_persistable_into(
        &self,
        cx: &App,
        serializers: &crate::client::persist::SerializerRegistry,
        now_ms: u64,
        out: &mut Vec<(
            crate::core::QueryKey,
            crate::client::persist::PersistedEntry,
        )>,
    ) {
        use crate::core::QueryStatus;

        // Serializers are registered by `T` alone, not the `(T, E)` pair.
        let type_id = std::any::TypeId::of::<T>();
        let Some(serialize_fn) = serializers.get(type_id) else {
            return;
        };
        for (key, entry) in self.entries.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            if resource.status() != QueryStatus::Success {
                continue;
            }
            let Some(page) = resource.first_page_arc() else {
                continue;
            };
            let Some(value) = serialize_fn(&*page as &dyn std::any::Any) else {
                continue;
            };
            out.push((
                key.clone(),
                crate::client::persist::PersistedEntry {
                    value,
                    cached_at: resource.last_updated_at_ms().unwrap_or(now_ms),
                    cache_policy: resource.cache_policy(),
                    meta: None,
                },
            ));
        }
    }
}
