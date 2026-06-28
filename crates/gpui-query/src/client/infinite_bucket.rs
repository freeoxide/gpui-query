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
///
/// `last_updated_ms` / `loading` form the M2 eviction mirror — see
/// [`BucketEntry`](super::bucket::types::BucketEntry) for the design.
struct InfiniteBucketEntry<T, E> {
    entity: WeakEntity<InfiniteQueryResource<T, E>>,
    sequencer: RequestSequencer,
    last_updated_ms: Option<u64>,
    loading: bool,
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
        if let Some(entry) = self.entries.get_mut(&key) {
            if let Some(entity) = entry.entity.upgrade() {
                // M2: refresh the mirror from the same read we already do for
                // the policy check (zero extra reads).
                let (needs_update, last_updated, loading) = entity.read_with(cx, |resource, _| {
                    let needs_update = resource.cache_policy() != cache_policy
                        || resource.request_policy() != request_policy;
                    (
                        needs_update,
                        resource.last_updated_at_ms(),
                        resource.is_loading(),
                    )
                });
                entry.last_updated_ms = last_updated;
                entry.loading = loading;
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

        let entity =
            cx.new(|_| InfiniteQueryResource::new(key.clone(), cache_policy, request_policy));
        self.entries.insert(
            key,
            InfiniteBucketEntry {
                entity: entity.downgrade(),
                sequencer: RequestSequencer::new(),
                last_updated_ms: None,
                loading: false,
            },
        );
        entity
    }

    /// Evict the oldest (least-recently-updated) entry to make room (#1 fix).
    ///
    /// **M2 (O(n)→O(1) entity reads)**: scans entry MIRRORS (no `entity.read`)
    /// + `WeakEntity::upgrade` liveness, then one `entity.read` on the winner
    /// to confirm `!is_loading()` (guards #109 against a stale mirror) and
    /// read the authoritative timestamp. Mirrors
    /// [`QueryBucket::evict_oldest`](super::QueryBucket::evict_oldest).
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        loop {
            let target = self
                .entries
                .iter()
                .filter_map(|(key, entry)| {
                    if entry.loading {
                        return None;
                    }
                    if entry.entity.upgrade().is_none() {
                        return None;
                    }
                    Some((key, entry.last_updated_ms.unwrap_or(0)))
                })
                .min_by_key(|&(_, age)| age);

            let Some((key, _)) = target else {
                return;
            };

            let key = key.clone();
            let still_loading = self
                .entries
                .get(&key)
                .and_then(|e| e.entity.upgrade())
                .map(|entity| entity.read(cx).is_loading());

            match still_loading {
                Some(true) => {
                    // Stale mirror: a fetch began after the last refresh. Mark
                    // and re-pick so #109 is honored.
                    if let Some(entry) = self.entries.get_mut(&key) {
                        entry.loading = true;
                    }
                    continue;
                }
                _ => {
                    self.entries.remove(&key);
                    return;
                }
            }
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
    pub(crate) fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        let gc_time_ms = gc_time_ms.max(MIN_GC_TIME_MS);
        let gc_threshold = gc_time_ms;
        let success_threshold = gc_threshold * (SUCCESS_GC_MULTIPLIER as u64);

        self.entries.retain(|_key, entry| {
            let Some(entity) = entry.entity.upgrade() else {
                return false;
            };
            let resource = entity.read(cx);

            // M2: refresh the eviction mirror from this read (gc walks every
            // entry anyway, so this is the canonical refresh point).
            entry.last_updated_ms = resource.last_updated_at_ms();
            entry.loading = resource.is_loading();

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

    /// **L9**: shared "collect matching keys -> upgrade weak ref -> per-entry
    /// action" driver used by `invalidate_matching` / `reset_matching` /
    /// `cancel_matching` (the erased-trait impl below). Mirrors
    /// [`QueryBucket::for_each_matching_entry`](super::QueryBucket::for_each_matching_entry).
    ///
    /// `remove_matching` is *not* routed through here — it is just
    /// `HashMap::retain`. The `action` closure receives the upgraded strong
    /// entity and `cx` so each caller picks its own lock pattern
    /// (unconditional `update` for invalidate/reset; `read_with`-then-`update`
    /// only-if-loading for cancel — see M7). Byte-identical to the prior
    /// inlined loops.
    fn for_each_matching_entry(
        &mut self,
        filter: &QueryKeyFilter,
        cx: &mut App,
        mut action: impl FnMut(&gpui::Entity<InfiniteQueryResource<T, E>>, &mut App),
    ) {
        let keys: Vec<QueryKey> = self
            .entries
            .keys()
            .filter(|key| filter.matches(key))
            .cloned()
            .collect();

        for key in keys {
            if let Some(entry) = self.entries.get(&key)
                && let Some(entity) = entry.entity.upgrade()
            {
                action(&entity, cx);
            }
        }
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

    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        InfiniteQueryBucket::gc(self, now_ms, gc_time_ms, cx);
    }

    fn count(&self) -> usize {
        self.entries.len()
    }

    /// Collect `QueryKey`s (cheap Arc increments) and upgrade individually
    /// inside the loop (#18 fix) — avoids pinning strong `Entity` handles in
    /// a `Vec` while iterating the map.
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
    /// **L15**: uses `is_loading()` directly (matching
    /// [`QueryBucket::cancel_matching`](super::bucket::erased_ops)) rather than
    /// the equivalent `status().is_loading()` — one style across both buckets.
    ///
    /// **M5**: after `signal.cancel()`, calls `resource.mark_ignored_result()`
    /// so the infinite resource bumps `ignored_results` exactly like the regular
    /// query path (the core half of M5 added the accessor).
    ///
    /// **M7 (deliberate trade-off, documented)**: `read_with`-then-`update`
    /// (two lock acquisitions) per match, matching `QueryBucket::cancel_matching`.
    /// `entity.update` always notifies observers even on a no-op closure, so
    /// updating every match would spam observers for non-loading entries. We
    /// gate on the authoritative `entity.read_with` `is_loading()` (NOT the M2
    /// entry `loading` mirror, which could be stale and skip an in-flight
    /// cancel) and only `update` when we will actually mutate. Accepted form
    /// of the audit's refined fix.
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
            // L6: use the accessor (checked_sub → None on clock skew) so
            // the diagnostic matches QueryBucket's cache_age_ms behavior.
            let age_ms = resource.cache_age_ms(now_ms);
            out.push(QueryDiagnostic {
                key: key.to_path(),
                status: resource.status(),
                cache_policy: resource.cache_policy().label(),
                cache_age_ms: age_ms,
                cache_hits: resource.cache_hits(),
                retry_count: resource.retry_count(),
            });
        }
    }

    /// Lightweight key/status pairs (#9). Pushes each live entry's `(key,
    /// status)` pair into `out`.
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &App, out: &mut Vec<(String, QueryStatus)>) {
        for (key, entry) in self.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push((key.to_path(), resource.status()));
        }
    }

    /// Value-carrying variant for persistence. Infinite resources dehydrate
    /// their first page only (the full page Vec is opaque to core; a richer
    /// multi-page serialization can layer on top of `meta` later). Entries
    /// without a registered serializer, or not in `Success`, are skipped.
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
            let Some(page) = resource.first_page_arc() else {
                continue;
            };
            // Serialize the single `Arc<T>` page as the opaque value.
            let value = serialize_fn(&*page as &dyn std::any::Any);
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
