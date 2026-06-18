//! Type-partitioned bucket for mutation resources.
//!
//! **v2 fix**: Implements actual GC instead of the v1 no-op.
//!
//! Each entry tracks its own `updated_at` timestamp (set on insertion and
//! whenever the hook layer calls `touch`). The GC removes entries whose
//! `updated_at` is older than `gc_time_ms` **and** that are not currently
//! loading. Because the erased `gc` signature has no `cx` (and therefore
//! cannot read entity state), the loading check is handled by a secondary
//! `retain_with_cx` pass — but the primary age-based filtering works purely
//! from the timestamp stored in the entry.
//!
//! **Audit fixes (this pass)**:
//! - `max_entries` cap + `evict_oldest` added (#2) — previously successful
//!   mutation resources were never evicted, causing unbounded memory growth.
//! - `observer_count` / `retain()` / `release()` removed (#8) — they were
//!   never incremented from production and are now redundant with
//!   `WeakEntity::upgrade()` liveness, matching `QueryBucket`.
//! - GC rewritten as a single `HashMap::retain` closure (#91) — it only reads
//!   `entity.read(cx)` (no update), so it is GPUI-safe and avoids the
//!   collect-ids-then-remove two-phase dance.
//! - `Success` mutations now evictable (#108) when their age exceeds
//!   `SUCCESS_GC_MULTIPLIER * gc_time_ms`, mirroring `QueryBucket::gc`.
//! - Local `MIN_GC_TIME_MS` removed (#69); imported from `bucket::types`.
//! - `insert` now takes a live `cx` (renamed from `_cx`) and evicts the oldest
//!   entry before inserting when at capacity (#114).
//! - `updated_at` is insertion-time only today; a future hook-side `touch()`
//!   call would refresh it (#112 — deferred, cross-file).

use ahash::AHashMap;
use gpui::{App, WeakEntity};

use crate::core::{MutationResource, MutationStatus};

use super::bucket::types::{DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS, SUCCESS_GC_MULTIPLIER};
use super::devtools::MutationDiagnostic;
use super::ErasedMutationBucket;

/// Default garbage-collection time for idle mutations (5 minutes).
#[allow(dead_code)]
pub const DEFAULT_MUTATION_GC_TIME_MS: u64 = 300_000;

/// Per-entry metadata stored alongside the entity.
///
/// Uses `WeakEntity` instead of `Entity` so that the bucket does not prevent
/// GPUI from garbage-collecting mutation resources when all component-held
/// strong references are dropped. The weak reference is upgraded on access;
/// if the entity was already collected, the entry is treated as dead and
/// cleaned up by GC.
///
/// The `loading` flag is maintained by the hook layer via `set_loading()`
/// and `set_not_loading()`. It provides a `cx`-free check so that the GC
/// can avoid evicting mid-flight mutations without needing to upgrade the
/// weak reference and read entity state.
struct MutationEntry<V, T, E> {
    entity: WeakEntity<MutationResource<V, T, E>>,
    /// Monotonic millisecond timestamp of the last state transition.
    ///
    /// Set at insertion time only today. A future hook-side `touch()` call
    /// (audit #112 — deferred, cross-file) would refresh it on mutation
    /// completion (transition to `Success`/`Failure`) so that the GC timer
    /// restarts from the completion moment rather than from insertion.
    updated_at: u128,
    /// Whether the mutation is currently in-flight (Loading state).
    /// Set to `true` on `begin()`, `false` on completion or reset.
    /// This allows the GC to protect mid-flight mutations without
    /// needing to read entity state via `cx`.
    loading: bool,
}

/// Type-partitioned storage for mutation resources.
pub struct MutationBucket<V, T, E> {
    resources: AHashMap<u64, MutationEntry<V, T, E>>,
    next_id: u64,
    /// Maximum number of entries allowed in this bucket (#2 fix).
    /// When exceeded, the oldest entry (by `updated_at`) is evicted.
    max_entries: usize,
}

/// Returns the current time as milliseconds since UNIX epoch.
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

impl<
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
> MutationBucket<V, T, E>
{
    pub(crate) fn new() -> Self {
        Self {
            resources: AHashMap::new(),
            next_id: 0,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    /// Evict the oldest (least-recently-updated) entry to make room for a new one.
    ///
    /// Called when `insert` would exceed `max_entries`. Selects the entry with
    /// the smallest `updated_at` whose entity is **not** currently loading
    /// (upgrade the weak entity, call `entity.read(cx).is_loading()` and skip
    /// if loading), mirroring `QueryBucket::evict_oldest`. The chosen id is
    /// captured once before removal.
    ///
    /// **Audit fix #2**.
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        let target = self
            .resources
            .iter()
            .filter_map(|(id, entry)| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                if resource.is_loading() {
                    return None;
                }
                Some((*id, entry.updated_at))
            })
            .min_by_key(|&(_, updated_at)| updated_at);

        if let Some((id, _)) = target {
            self.resources.remove(&id);
        }
    }

    /// Insert a mutation entity, recording the current time as `updated_at`.
    ///
    /// Returns the generated numeric id for the entry.
    ///
    /// When the bucket is at capacity, the oldest non-loading entry is evicted
    /// first (#2). **Audit fix #114**: the `cx` param (previously unused
    /// `_cx`) is now passed to `evict_oldest`.
    ///
    /// **Audit fix #3**: Uses `checked_add` on `next_id` to prevent wraparound
    /// after `u64::MAX` insertions. If the counter overflows, the method
    /// panics — consistent with the principle that IDs must be unique and
    /// wraparound would cause data loss.
    pub(crate) fn insert(
        &mut self,
        entity: &gpui::Entity<MutationResource<V, T, E>>,
        cx: &App,
    ) -> u64 {
        if self.resources.len() >= self.max_entries {
            self.evict_oldest(cx);
        }

        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1)
            .expect("MutationBucket::insert: next_id overflow after u64::MAX insertions");
        self.resources.insert(
            id,
            MutationEntry {
                entity: entity.downgrade(),
                updated_at: now_ms(),
                loading: false,
            },
        );
        id
    }

    /// Refresh the `updated_at` timestamp for an entry.
    ///
    /// The hook layer should call this whenever the mutation completes
    /// (transitions to `Success` or `Failure`) so that the GC timer
    /// restarts from the completion moment rather than from insertion.
    ///
    /// **Audit #112**: cross-file fix deferred — the hook layer does not yet
    /// call this on completion, so `updated_at` remains insertion-time only.
    #[allow(dead_code)]
    pub(crate) fn touch(&mut self, id: u64) {
        if let Some(entry) = self.resources.get_mut(&id) {
            entry.updated_at = now_ms();
        }
    }

    /// Mark a mutation entry as currently loading (in-flight).
    ///
    /// The hook layer should call this when `begin()` is called on the
    /// mutation resource. This sets a `loading` flag on the entry that the
    /// GC checks without needing `cx` to read entity state, preventing
    /// mid-flight eviction of long-running mutations.
    #[allow(dead_code)]
    pub(crate) fn set_loading(&mut self, id: u64) {
        if let Some(entry) = self.resources.get_mut(&id) {
            entry.loading = true;
        }
    }

    /// Mark a mutation entry as no longer loading (completed or reset).
    ///
    /// The hook layer should call this when the mutation reaches a terminal
    /// state (`Success`, `Failure`, or `Idle` via `reset()`).
    #[allow(dead_code)]
    pub(crate) fn set_not_loading(&mut self, id: u64) {
        if let Some(entry) = self.resources.get_mut(&id) {
            entry.loading = false;
        }
    }

    /// All entities in this bucket that are still alive.
    ///
    /// **Audit 3 fix (finding 1)**: This method allocates a `Vec` of all
    /// mutation entities by upgrading weak references. Callers that invoke
    /// this on every render (e.g., `use_mutation_state()`) will allocate a
    /// new `Vec` each time. If this becomes a performance concern, consider
    /// caching the result or calling this less frequently.
    pub(crate) fn all_entities(&self) -> Vec<gpui::Entity<MutationResource<V, T, E>>> {
        self.resources
            .values()
            .filter_map(|e| e.entity.upgrade())
            .collect()
    }
}

impl<
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
> ErasedMutationBucket for MutationBucket<V, T, E>
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Garbage-collect stale mutation resources.
    ///
    /// **Audit fix #91**: Rewritten as a single `HashMap::retain` closure
    /// that returns `false` to evict. It only reads `entity.read(cx)` (no
    /// `update`), so it is GPUI-safe and avoids the two-phase collect-then-
    /// remove dance. Preserves the exact eviction semantics.
    ///
    /// **Audit fix #4**: Uses `cx` to read entity state and check
    /// `is_loading()`, matching the pattern used in `QueryBucket::gc`.
    ///
    /// **Audit fix #2**: Also checks the entry-level `loading` flag as a
    /// secondary guard, protecting mid-flight mutations even when the weak
    /// reference cannot be upgraded.
    ///
    /// **Audit fix #5**: Removes dead entries whose `WeakEntity` can no
    /// longer be upgraded (all strong references dropped).
    ///
    /// **Audit fix #108**: `Success` mutations are now evictable when their
    /// age exceeds `SUCCESS_GC_MULTIPLIER * gc_time_ms`, mirroring how
    /// `QueryBucket::gc` computes the success threshold. `Idle` and
    /// `Failure` remain evictable at `gc_threshold`.
    ///
    /// Evicts entries where:
    /// 1. The entity has been collected (weak ref dead) and the entry is not
    ///    flagged loading, OR
    /// 2. The entry is not loading (flag and entity state), is in an evictable
    ///    status (`Idle`/`Failure`/`Success`), and the age exceeds the
    ///    threshold for that status (`gc_threshold` for Idle/Failure,
    ///    `SUCCESS_GC_MULTIPLIER * gc_threshold` for Success).
    fn gc(&mut self, now_ms: u128, gc_time_ms: u64, cx: &App) {
        let gc_time_ms = gc_time_ms.max(MIN_GC_TIME_MS);
        let gc_threshold = gc_time_ms as u128;
        let success_threshold = gc_threshold * (SUCCESS_GC_MULTIPLIER as u128);

        self.resources.retain(|_id, entry| {
            // Audit fix (finding 2): Never evict entries flagged as loading.
            // A dead entry with loading=true means the mutation is in-flight
            // but the weak ref couldn't upgrade — keep it for one more GC
            // cycle as a safety measure.
            if entry.loading {
                return true;
            }

            // Audit fix (finding 5): Remove dead entries whose entity has
            // already been collected.
            let entity = match entry.entity.upgrade() {
                Some(e) => e,
                None => return false,
            };

            let resource = entity.read(cx);

            // Audit fix (finding 4): Never evict resources that are actively
            // loading, even if the entry flag disagrees.
            if resource.is_loading() {
                return true;
            }

            let status = resource.status();

            // Audit fix #108: Success is evictable on its own (longer) age
            // threshold; Idle and Failure evict at gc_threshold.
            let (evictable, threshold) = match status {
                MutationStatus::Success => (true, success_threshold),
                MutationStatus::Idle | MutationStatus::Failure => (true, gc_threshold),
                MutationStatus::Loading => (false, gc_threshold),
            };
            if !evictable {
                return true;
            }

            let age = now_ms.saturating_sub(entry.updated_at);
            age < threshold
        });
    }

    fn count(&self) -> usize {
        self.resources.len()
    }

    /// Collect per-resource diagnostic details for all live mutation entries.
    ///
    /// Iterates all entries, upgrades weak references, reads entity state,
    /// and constructs a `MutationDiagnostic` for each live resource.
    /// Dead entries (collected entities) are skipped.
    fn collect_diagnostics(&self, cx: &App) -> Vec<MutationDiagnostic> {
        self.resources
            .values()
            .filter_map(|entry| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                Some(MutationDiagnostic {
                    key: resource.key().map(|k| k.to_path()),
                    status: resource.status(),
                    retry_count: resource.retry_count(),
                })
            })
            .collect()
    }
}
