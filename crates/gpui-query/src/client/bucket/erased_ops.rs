//! `ErasedBucket` trait implementation for `QueryBucket`.
//!
//! Contains garbage collection, bulk invalidation/reset/cancel, and
//! diagnostic collection — all methods dispatched through the type-erased
//! bucket trait.

use gpui::App;

use crate::client::devtools::QueryDiagnostic;
use crate::client::erased::ErasedBucket;
use crate::core::{QueryKey, QueryKeyFilter, QueryStatus};

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
        let keys: Vec<QueryKey> = self
            .entries
            .keys()
            .filter(|key| filter.matches(key))
            .cloned()
            .collect();

        for key in keys {
            if let Some(entry) = self.entries.get(&key)
                && let Some(entity) = entry.entity.upgrade() {
                    entity.update(cx, |resource, _| {
                        resource.invalidate();
                    });
                }
        }
    }

    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        let keys: Vec<QueryKey> = self
            .entries
            .keys()
            .filter(|key| filter.matches(key))
            .cloned()
            .collect();

        for key in keys {
            if let Some(entry) = self.entries.get(&key)
                && let Some(entity) = entry.entity.upgrade() {
                    entity.update(cx, |resource, _| {
                        resource.reset();
                    });
                }
        }
    }

    fn remove_matching(&mut self, filter: &QueryKeyFilter) {
        self.entries.retain(|k, _| !filter.matches(k));
    }

    /// Cancel in-flight requests for entries matching the filter.
    ///
    /// Uses the collect-keys-then-update pattern to avoid mutating the
    /// `HashMap` during iteration. Only cancels entries that have an
    /// active request (status is loading).
    fn cancel_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        let keys: Vec<QueryKey> = self
            .entries
            .keys()
            .filter(|key| filter.matches(key))
            .cloned()
            .collect();

        for key in keys {
            if let Some(entry) = self.entries.get(&key)
                && let Some(entity) = entry.entity.upgrade() {
                    let is_loading = entity.read_with(cx, |r, _| r.is_loading());
                    if is_loading {
                        entity.update(cx, |resource, _| {
                            if let Some(signal) = resource.signal() {
                                signal.cancel();
                            }
                            resource.mark_ignored_result();
                        });
                    }
                }
        }
    }

    /// Collect per-resource diagnostic details for all live entries.
    fn collect_diagnostics(&self, now_ms: u64, cx: &App) -> Vec<QueryDiagnostic> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                Some(QueryDiagnostic {
                    key: key.to_path(),
                    status: resource.status(),
                    cache_policy: resource.cache_policy().label(),
                    cache_age_ms: resource.cache_age_ms(now_ms),
                    cache_hits: resource.cache_hits(),
                    retry_count: resource.retry_count(),
                })
            })
            .collect()
    }

    /// Lightweight key/status pairs (#9). Avoids the `String` allocations of
    /// `cache_policy`/`retry_count` and the `now_ms` syscall for callers that
    /// only need the key and status (e.g. `dehydrate`).
    fn collect_key_status(&self, cx: &App) -> Vec<(String, QueryStatus)> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                Some((key.to_path(), resource.status()))
            })
            .collect()
    }
}
