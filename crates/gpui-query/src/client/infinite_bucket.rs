//! Type-partitioned bucket for infinite query resources.
//!
//! Mirrors [`QueryBucket`] but for [`InfiniteQueryResource`]. Co-locates a
//! [`RequestSequencer`] with each entity so that request IDs are monotonic
//! across the lifetime of the resource (audit fix: persistent sequencer).
//!
//! **Audit fixes (this pass)**:
//! - GC reads entity state directly via `entity.read(cx)` (CL2/#106) instead
//!   of a stale `StatusSnapshot`.
//! - `max_entries` cap + `evict_oldest` added (#1) — previously successful
//!   infinite resources were never evicted, causing unbounded memory growth.
//! - `Cancelled` added to the evictable set (#107).
//! - `retain`/`release`/`observer_count`/`update_status_snapshot` removed
//!   (#8, #75) — `observer_count` was never incremented from production.
//! - `invalidate_matching` collects `QueryKey`s instead of pinning strong
//!   `Entity` handles (#18).
//! - Opportunistic GC trigger added (CL1/#105).

use ahash::AHashMap;
use gpui::{App, AppContext as _, WeakEntity};

use super::bucket::types::{DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS, SUCCESS_GC_MULTIPLIER};
use crate::core::{
    CachePolicy, InfiniteQueryResource, QueryKey, QueryKeyFilter, QueryStatus, RequestPolicy,
    RequestSequencer,
};

/// Entry co-locating weak entity reference and sequencer.
struct InfiniteBucketEntry<T, E> {
    entity: WeakEntity<InfiniteQueryResource<T, E>>,
    sequencer: RequestSequencer,
}

/// Type-partitioned storage for infinite query resources of a specific `(T, E)` type pair.
pub struct InfiniteQueryBucket<T, E> {
    entries: AHashMap<QueryKey, InfiniteBucketEntry<T, E>>,
    /// Maximum number of entries allowed in this bucket (#1 fix).
    max_entries: usize,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> InfiniteQueryBucket<T, E> {
    pub(crate) fn new() -> Self {
        Self {
            entries: AHashMap::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    /// Get an existing entity or create a new one.
    ///
    /// If the key already exists and the weak reference can be upgraded, returns
    /// the existing entity. If the entity was collected, the stale entry is replaced.
    ///
    /// When the key already exists and the policies differ, updates in-place.
    pub(crate) fn get_or_create(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> gpui::Entity<InfiniteQueryResource<T, E>> {
        // Audit fix #58: collapse the dead-ref path from get → remove → insert
        // (3 hashes) to get → insert-overwrite (2 hashes). `entry()` cannot
        // span the eviction (borrow conflict with `self.evict_oldest`), so the
        // create path re-probes via `insert` — best-effort per the audit.
        if let Some(entry) = self.entries.get(&key) {
            if let Some(entity) = entry.entity.upgrade() {
                let needs_update = entity.read_with(cx, |resource, _| {
                    resource.cache_policy() != cache_policy
                        || resource.request_policy() != request_policy
                });
                if needs_update {
                    entity.update(cx, |resource, _| {
                        resource.set_cache_policy(cache_policy);
                        resource.set_request_policy(request_policy);
                    });
                }
                return entity;
            }
            // Dead occupant: fall through to the insert path, which overwrites
            // the stale entry in place. Length unchanged, no eviction.
        } else if self.entries.len() >= self.max_entries {
            self.evict_oldest(cx);
        }

        let entity = cx.new(|_| InfiniteQueryResource::new(key.clone(), cache_policy, request_policy));
        self.entries.insert(
            key,
            InfiniteBucketEntry {
                entity: entity.downgrade(),
                sequencer: RequestSequencer::new(),
            },
        );
        entity
    }

    /// Evict the oldest (least-recently-updated) entry to make room (#1 fix).
    ///
    /// Mirrors `QueryBucket::evict_oldest`: skips in-flight entries (#109),
    /// clones the chosen key once (#59).
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        // Audit fix #59: select by reference first, clone once for removal.
        let target = self
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                if resource.is_loading() {
                    return None;
                }
                let age = resource.last_updated_at_ms().unwrap_or(0);
                Some((key, age))
            })
            .min_by_key(|&(_, age)| age);

        // Clone the winning key out of the immutable borrow so `remove` can
        // take `&mut self.entries` (E0502: the iterator above still holds the
        // shared borrow through `key`). One clone total — same as audit fix #59.
        if let Some((key, _)) = target {
            let key = key.clone();
            self.entries.remove(&key);
        }
    }

    // (Audit cleanup: `maybe_gc` was dead code — never called from production.
    // The real trigger is `QueryClient::maybe_opportunistic_gc`. Removed with
    // `should_run_opportunistic_gc` and the now-write-only `last_gc_ms` field.)

    /// Get an existing entity by key.
    pub(crate) fn get(&self, key: &QueryKey) -> Option<gpui::Entity<InfiniteQueryResource<T, E>>> {
        self.entries.get(key).and_then(|e| e.entity.upgrade())
    }

    /// Get a mutable reference to the sequencer for an entry.
    pub(crate) fn sequencer_mut(&mut self, key: &QueryKey) -> Option<&mut RequestSequencer> {
        self.entries.get_mut(key).map(|e| &mut e.sequencer)
    }

    /// All entities in this bucket that are still alive.
    ///
    /// Allocates a `Vec` per call — callers on hot render paths should cache
    /// the result (#60).
    pub(crate) fn all_entities(&self) -> Vec<gpui::Entity<InfiniteQueryResource<T, E>>> {
        self.entries
            .values()
            .filter_map(|e| e.entity.upgrade())
            .collect()
    }

    /// Run garbage collection on this bucket.
    ///
    /// Reads entity state directly via `entity.read(cx)` (CL2/#106). Evicts
    /// `Idle`/`Failure`/`Cancelled` entries older than `gc_time_ms` and
    /// `Success` entries older than `SUCCESS_GC_MULTIPLIER * gc_time_ms` (#1).
    pub(crate) fn gc(&mut self, now_ms: u128, gc_time_ms: u64, cx: &App) {
        let gc_time_ms = gc_time_ms.max(MIN_GC_TIME_MS);
        let gc_threshold = gc_time_ms as u128;
        let success_threshold = gc_threshold * (SUCCESS_GC_MULTIPLIER as u128);

        self.entries.retain(|_key, entry| {
            let Some(entity) = entry.entity.upgrade() else { return false };
            let resource = entity.read(cx);

            if resource.is_loading() {
                return true;
            }

            let status = resource.status();

            let age_ms = resource
                .last_updated_at_ms()
                .map(|updated| now_ms.saturating_sub(updated))
                .unwrap_or(gc_threshold);

            if status == QueryStatus::Success {
                let cache_policy = resource.cache_policy();
                if cache_policy.can_serve_stale() && !cache_policy.is_expired(age_ms) {
                    return true;
                }
                return age_ms < success_threshold;
            }

            let evictable = matches!(
                status,
                QueryStatus::Idle | QueryStatus::Failure | QueryStatus::Cancelled
            );
            if !evictable {
                return true;
            }

            age_ms < gc_threshold
        });
    }
}

// Implement the erased trait so InfiniteQueryBucket can live in QueryClient's
// heterogeneous map.
use super::ErasedInfiniteBucket;
use super::devtools::QueryDiagnostic;

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> ErasedInfiniteBucket
    for InfiniteQueryBucket<T, E>
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn gc(&mut self, now_ms: u128, gc_time_ms: u64, cx: &App) {
        InfiniteQueryBucket::gc(self, now_ms, gc_time_ms, cx);
    }

    fn count(&self) -> usize {
        self.entries.len()
    }

    /// Collect `QueryKey`s (cheap Arc increments) and upgrade individually
    /// inside the loop (#18 fix) — avoids pinning strong `Entity` handles in
    /// a `Vec` while iterating the map.
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
                    let is_loading = entity.read_with(cx, |r, _| r.status().is_loading());
                    if is_loading {
                        entity.update(cx, |resource, _| {
                            if let Some(signal) = resource.signal() {
                                signal.cancel();
                            }
                        });
                    }
                }
        }
    }

    /// Collect per-resource diagnostic details for all live infinite query entries.
    fn collect_diagnostics(&self, now_ms: u128, cx: &App) -> Vec<QueryDiagnostic> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                let age_ms = resource
                    .last_updated_at_ms()
                    .map(|updated| now_ms.saturating_sub(updated));
                Some(QueryDiagnostic {
                    key: key.to_path(),
                    status: resource.status(),
                    cache_policy: resource.cache_policy().label(),
                    cache_age_ms: age_ms,
                    cache_hits: resource.cache_hits(),
                    retry_count: resource.retry_count(),
                })
            })
            .collect()
    }

    /// Lightweight key/status pairs (#9).
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
