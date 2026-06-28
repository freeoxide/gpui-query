//! `ErasedBucket` trait implementation for `QueryBucket`.
//!
//! Contains garbage collection, bulk invalidation/reset/cancel, and
//! diagnostic collection — all methods dispatched through the type-erased
//! bucket trait.

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

    /// Garbage-collect stale query resources.
    ///
    /// Reads entity state directly via `entity.read(cx)` (CL2/#106 fix) —
    /// the cached `StatusSnapshot` was never refreshed from production and
    /// was therefore stale. Evicts entries where:
    /// 1. The weak reference is dead (entity was collected), OR
    /// 2. The resource is in an evictable state (`Idle`, `Failure`, or
    ///    `Cancelled` — #107) and data age exceeds `gc_time_ms`, OR
    /// 3. The resource is `Success` and data age exceeds
    ///    `SUCCESS_GC_MULTIPLIER * gc_time_ms`, OR
    /// 4. The resource is `Success` with a `StaleWhileRevalidate` policy
    ///    and data age exceeds the total valid window.
    ///
    /// Resources that are actively loading are always retained.
    ///
    /// Uses `HashMap::retain()` to avoid the intermediate `Vec<QueryKey>`
    /// allocation.
    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        QueryBucket::gc(self, now_ms, gc_time_ms, cx);
    }

    fn count(&self) -> usize {
        self.entries.len()
    }

    /// Collect keys (cheap Arc increments) then upgrade individually, deferring
    /// the `upgrade()` cost and avoiding upgrades for entities that may have
    /// been collected between collection and update.
    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_matching_entry(filter, cx, |entity, cx| {
            entity.update(cx, |resource, _| resource.invalidate());
        });
    }

    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_matching_entry(filter, cx, |entity, cx| {
            entity.update(cx, |resource, _| resource.reset());
        });
    }

    fn remove_matching(&mut self, filter: &QueryKeyFilter) {
        self.entries.retain(|k, _| !filter.matches(k));
    }

    /// Cancel in-flight requests for entries matching the filter.
    ///
    /// **M7 (deliberate trade-off, documented)**: this does a
    /// `read_with`-then-`update` (two entity lock acquisitions) per match rather
    /// than a single unconditional `update`. The reason is that
    /// `entity.update` *always* notifies observers even when the closure mutates
    /// nothing, so updating every matching entry would spam observers with
    /// no-op notifications for entries that aren't loading. Instead we read the
    /// authoritative `is_loading()` flag (the M2 entry `loading` mirror is
    /// *intentionally not consulted here* — a stale mirror could skip an
    /// in-flight cancel) and only pay for the `update` when we will actually
    /// mutate. This is the accepted form of the audit's refined fix (option a/b).
    fn cancel_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        self.for_each_matching_entry(filter, cx, |entity, cx| {
            let is_loading = entity.read_with(cx, |r, _| r.is_loading());
            if is_loading {
                entity.update(cx, |resource, _| {
                    if let Some(signal) = resource.signal() {
                        signal.cancel();
                    }
                    resource.mark_ignored_result();
                });
            }
        });
    }

    /// Push each live entry's diagnostic into `out` instead of allocating a
    /// fresh `Vec`.
    fn collect_diagnostics_into(&self, now_ms: u64, cx: &App, out: &mut Vec<QueryDiagnostic>) {
        for (key, entry) in self.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push(QueryDiagnostic {
                key: key.to_path(),
                status: resource.status(),
                cache_policy: resource.cache_policy().label(),
                cache_age_ms: resource.cache_age_ms(now_ms),
                cache_hits: resource.cache_hits(),
                retry_count: resource.retry_count(),
            });
        }
    }

    /// Lightweight key/status pairs (#9). Pushes each live entry's `(key,
    /// status)` pair into `out`, avoiding the `String` allocations of
    /// `cache_policy`/`retry_count` and the `now_ms` syscall for callers that
    /// only need the key and status (e.g. `dehydrate`).
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &App, out: &mut Vec<(String, crate::core::QueryStatus)>) {
        for (key, entry) in self.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push((key.to_path(), resource.status()));
        }
    }

    /// Value-carrying variant for persistence. For each `Success` entry whose
    /// `(T, E)` has a registered serializer, push `(key, PersistedEntry)` into
    /// `out`. Entries without a serializer (or not in `Success`) are skipped.
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

        let type_id = std::any::TypeId::of::<(T, E)>();
        let Some(serialize_fn) = serializers.get(type_id) else {
            return;
        };
        for (key, entry) in self.entries.iter() {
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
            let value = serialize_fn(data as &dyn std::any::Any);
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
