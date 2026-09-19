use gpui::App;

use crate::client::devtools::QueryDiagnostic;
use crate::client::erased::ErasedBucket;
use crate::core::QueryKeyFilter;

use super::ops::QueryBucket;

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> ErasedBucket
    for QueryBucket<T, E>
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        self.inner.gc(now_ms, gc_time_ms, cx);
    }

    fn count(&self) -> usize {
        self.inner.entries.len()
    }

    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.inner.for_each_matching_entry(filter, cx, |entity, cx| {
            // invalidate() only clears last_updated_at; skip the no-op update, which still notifies observers.
            let needs_invalidate =
                entity.read_with(cx, |r, _| r.last_updated_at_ms().is_some());
            if needs_invalidate {
                entity.update(cx, |resource, _| resource.invalidate());
            }
        });
    }

    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.inner.for_each_matching_entry(filter, cx, |entity, cx| {
            entity.update(cx, |resource, _| resource.reset());
        });
    }

    fn remove_matching(&mut self, filter: &QueryKeyFilter) {
        self.inner.entries.retain(|k, _| !filter.matches(k));
    }

    /// `entity.update` notifies observers even when the closure mutates
    /// nothing, so gate on the authoritative `is_loading()` read (the entry
    /// mirror could be stale and skip an in-flight cancel).
    fn cancel_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.inner.for_each_matching_entry(filter, cx, |entity, cx| {
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
        self.inner.collect_diagnostics_into(now_ms, cx, out);
    }

    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &App, out: &mut Vec<(String, crate::core::QueryStatus)>) {
        self.inner.collect_key_status_into(cx, out);
    }

    #[cfg(feature = "persist")]
    fn contains_key(&self, key: &crate::core::QueryKey) -> bool {
        self.inner.entries.contains_key(key)
    }

    /// Only `Success` entries whose `T` has a registered serializer are
    /// pushed; everything else is skipped.
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
        for (key, entry) in self.inner.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            if resource.status() != QueryStatus::Success {
                continue;
            }
            let Some(data) = resource.data() else {
                continue;
            };
            // Downcast failure is unreachable by construction; skip rather than persist junk.
            let Some(value) = serialize_fn(data as &dyn std::any::Any) else {
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
