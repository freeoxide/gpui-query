//! Core operations for `QueryBucket`: construction, get-or-create, and
//! sequencer access.

use ahash::AHashMap;
use gpui::{App, AppContext as _};

use crate::core::{
    CachePolicy, QueryKey, QueryResource, QueryStatus, RequestPolicy, RequestSequencer,
};

use super::shared::should_run_opportunistic_gc;
use super::types::{BucketEntry, DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS};

/// Type-partitioned storage for query resources of a specific `(T, E)` type pair.
pub struct QueryBucket<T, E> {
    pub(crate) entries: AHashMap<QueryKey, BucketEntry<T, E>>,
    /// Maximum number of entries allowed in this bucket.
    /// When exceeded, the oldest entry (by `last_updated_ms`) is evicted.
    pub(crate) max_entries: usize,
    /// Timestamp (ms since UNIX epoch) of the last GC sweep on this bucket.
    /// Used by the opportunistic GC trigger to debounce sweeps (CL1/#105).
    pub(crate) last_gc_ms: u128,
}

impl<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static> QueryBucket<T, E> {
    /// Create a new bucket with the default max entry limit.
    pub(crate) fn new() -> Self {
        Self {
            entries: AHashMap::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
            last_gc_ms: 0,
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
                Some((key.clone(), age))
            })
            .min_by_key(|&(_, age)| age);

        if let Some((key, _)) = target {
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
            self.entries.remove(&key);
        }

        if self.entries.len() >= self.max_entries {
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

    /// Opportunistic GC — run GC every `GC_INTERVAL` insertions if enough time
    /// has elapsed since the last sweep (CL1/#105). This makes GC actually
    /// fire in production without requiring hooks to call `gc()` explicitly.
    pub(crate) fn maybe_gc(&mut self, now_ms: u128, gc_time_ms: u64, cx: &App) {
        if !should_run_opportunistic_gc(self.entries.len(), gc_time_ms, self.last_gc_ms, now_ms) {
            return;
        }
        self.gc(now_ms, gc_time_ms, cx);
    }

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
        self.last_gc_ms = now_ms;
        let gc_time_ms = gc_time_ms.max(MIN_GC_TIME_MS);
        let gc_threshold = gc_time_ms as u128;
        let success_threshold = gc_threshold * (super::types::SUCCESS_GC_MULTIPLIER as u128);

        self.entries.retain(|_key, entry| {
            let entity = match entry.entity.upgrade() {
                Some(e) => e,
                None => return false,
            };
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
