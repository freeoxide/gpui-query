//! Core operations for `QueryBucket`: construction, get-or-create, and
//! sequencer access.

use ahash::AHashMap;
use gpui::{App, AppContext as _};

use crate::core::{
    CachePolicy, QueryKey, QueryResource, QueryStatus, RequestPolicy, RequestSequencer,
};

use super::types::{BucketEntry, DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS};

/// Type-partitioned storage for query resources of a specific `(T, E)` type pair.
pub struct QueryBucket<T, E> {
    pub(crate) entries: AHashMap<QueryKey, BucketEntry<T, E>>,
    /// Maximum number of entries allowed in this bucket.
    /// When exceeded, the oldest entry (by `last_updated_ms`) is evicted.
    pub(crate) max_entries: usize,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> QueryBucket<T, E> {
    /// Create a new bucket with the default max entry limit.
    pub(crate) fn new() -> Self {
        Self {
            entries: AHashMap::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    /// Evict the oldest (least-recently-updated) entry to make room for a new one.
    ///
    /// Called when `get_or_create` would exceed `max_entries`. Selects the
    /// entry with the smallest `last_updated_at_ms` (reading entity state
    /// directly — CL2/#106). Entries that are actively loading are skipped
    /// (#109) so in-flight requests are never evicted. The chosen key is
    /// cloned once (#59) rather than on every iteration.
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        // Audit fix #59: find the winning key by reference first (no clone in
        // the filter), then clone exactly once for the `remove`. Previously
        // every candidate cloned its `QueryKey` just to feed `min_by_key`.
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

    /// Get an existing entity or create a new one.
    ///
    /// If the key already exists and the weak reference can be upgraded, the
    /// existing entity is returned. If the entity was already collected (all
    /// strong references dropped), the stale entry is replaced with a fresh one.
    ///
    /// When the key already exists and the policies differ from the stored
    /// resource's current policies, the resource is updated in-place via
    /// `set_cache_policy` / `set_request_policy`.
    ///
    /// When creating a new entry would exceed `max_entries`, the oldest entry
    /// is evicted first (finding 4 fix).
    pub(crate) fn get_or_create(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> gpui::Entity<QueryResource<T, E>> {
        // Audit fix #58: previously the dead-ref path hashed the key up to 3x
        // (get → remove → insert). We collapse it to 2x: one probe to resolve
        // the hit/dead/miss outcome, then a single `insert` for the create
        // path. `entry()` cannot span the eviction (its borrow of
        // `self.entries` conflicts with `self.evict_oldest`'s borrow of `self`),
        // so the dead/miss path re-probes via `insert` — best-effort, noted in
        // the audit. The common alive-hit path is still a single hash.
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
            // the stale entry in place. Length is unchanged so no eviction.
        } else if self.entries.len() >= self.max_entries {
            // Vacant and at capacity: evict before creating.
            self.evict_oldest(cx);
        }

        let entity = cx.new(|_| QueryResource::new(key.clone(), cache_policy, request_policy));
        self.entries.insert(
            key,
            BucketEntry {
                entity: entity.downgrade(),
                sequencer: RequestSequencer::new(),
            },
        );
        entity
    }

    // (Audit cleanup: `maybe_gc` was dead code — never called from production.
    // The real opportunistic trigger is `QueryClient::maybe_opportunistic_gc`,
    // which drives `gc` directly every `GC_INTERVAL` ops. Removed alongside
    // `should_run_opportunistic_gc` and the now-write-only `last_gc_ms` field.)

    /// Get an existing entity by key.
    ///
    /// Returns `None` if the key is not in the bucket or if the weak reference
    /// can no longer be upgraded (the entity was collected).
    pub(crate) fn get(&self, key: &QueryKey) -> Option<gpui::Entity<QueryResource<T, E>>> {
        self.entries.get(key).and_then(|e| e.entity.upgrade())
    }

    /// All entities in this bucket that are still alive.
    ///
    /// Allocates a `Vec` per call — callers on hot render paths should cache
    /// the result (#60).
    pub(crate) fn all_entities(&self) -> Vec<gpui::Entity<QueryResource<T, E>>> {
        self.entries
            .values()
            .filter_map(|e| e.entity.upgrade())
            .collect()
    }

    /// Get a mutable reference to the sequencer for an entry.
    ///
    /// Returns `None` if the key is not found.
    pub(crate) fn sequencer_mut(&mut self, key: &QueryKey) -> Option<&mut RequestSequencer> {
        self.entries.get_mut(key).map(|e| &mut e.sequencer)
    }

    /// Run garbage collection on this bucket.
    ///
    /// Reads entity state directly via `entity.read(cx)` (CL2/#106) rather
    /// than trusting a cached snapshot. The snapshot machinery was never
    /// refreshed from production hooks, so it was always stale.
    pub(crate) fn gc(&mut self, now_ms: u128, gc_time_ms: u64, cx: &App) {
        let gc_time_ms = gc_time_ms.max(MIN_GC_TIME_MS);
        let gc_threshold = gc_time_ms as u128;
        let success_threshold = gc_threshold * (super::types::SUCCESS_GC_MULTIPLIER as u128);

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
