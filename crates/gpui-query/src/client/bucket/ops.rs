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
    /// entry with the smallest mirrored `last_updated_ms`. Entries that are
    /// mirrored as loading are skipped (#109) so in-flight requests are never
    /// evicted.
    ///
    /// **M2 (O(n)→O(1) entity reads)**: the scan reads only the entry MIRRORS
    /// (cheap field reads + `WeakEntity::upgrade` liveness, no `entity.read`),
    /// then performs **one** `entity.read` on the single winner to confirm
    /// `!is_loading()` (guards #109 against a stale mirror where a fetch began
    /// after the last refresh) and read the authoritative
    /// `last_updated_at_ms`. If the winner is actually loading, its mirror is
    /// marked and we re-pick. Typical cost: 1 entity read.
    ///
    /// The chosen key is cloned once (#59) rather than on every iteration.
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        loop {
            // Mirror scan: NO entity.read here. Pick min mirror-timestamp
            // among entries whose mirror says !loading AND whose weak ref is
            // still live (a dead entry is GC's job, not eviction's, but it
            // also can't be the "oldest live" winner).
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
                // Every live entry is mirrored as loading: nothing safe to evict.
                return;
            };

            // Single confirm read on the winner. Cloned out of the shared
            // borrow so `remove` can take `&mut self.entries` (E0502).
            let key = key.clone();
            let still_loading = self
                .entries
                .get(&key)
                .and_then(|e| e.entity.upgrade())
                .map(|entity| entity.read(cx).is_loading());

            match still_loading {
                Some(true) => {
                    // Mirror was stale: a fetch began after the last refresh.
                    // Mark it and re-pick so #109 is honored.
                    if let Some(entry) = self.entries.get_mut(&key) {
                        entry.loading = true;
                    }
                    continue;
                }
                _ => {
                    // Confirmed not loading (or already dead/collected between
                    // the scan and the confirm): evict.
                    self.entries.remove(&key);
                    return;
                }
            }
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
                // Fresh resource: never completed, not loading.
                last_updated_ms: None,
                loading: false,
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

    /// **L9**: shared "collect matching keys -> upgrade weak ref -> per-entry
    /// action" driver used by `invalidate_matching` / `reset_matching` /
    /// `cancel_matching` (the erased-trait impls in `erased_ops.rs`).
    ///
    /// `remove_matching` is *not* routed through here — it is just
    /// `HashMap::retain`, with no upgrade or `update`.
    ///
    /// The `action` closure receives the upgraded strong entity and the `cx`,
    /// so each caller chooses its own lock pattern: `invalidate`/`reset` do a
    /// single unconditional `entity.update`, while `cancel` does a
    /// `read_with`-then-`update`-only-if-loading (see M7). Semantics are
    /// byte-identical to the prior inlined loops: same filter, same per-key
    /// `get` + `upgrade`, same per-entry closure body.
    pub(crate) fn for_each_matching_entry(
        &mut self,
        filter: &crate::core::QueryKeyFilter,
        cx: &mut App,
        mut action: impl FnMut(&gpui::Entity<QueryResource<T, E>>, &mut App),
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

    /// Run garbage collection on this bucket.
    ///
    /// Reads entity state directly via `entity.read(cx)` (CL2/#106) rather
    /// than trusting a cached snapshot. The snapshot machinery was never
    /// refreshed from production hooks, so it was always stale.
    pub(crate) fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        let gc_time_ms = gc_time_ms.max(MIN_GC_TIME_MS);
        let gc_threshold = gc_time_ms;
        let success_threshold = gc_threshold * (super::types::SUCCESS_GC_MULTIPLIER as u64);

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
}
