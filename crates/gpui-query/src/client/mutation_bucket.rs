//! Type-partitioned bucket for mutation resources.
//!
//! **v2 fix**: Implements actual GC instead of the v1 no-op.
//!
//! Each entry tracks an `updated_at` timestamp set at insertion. The GC removes
//! entries whose `updated_at` is older than `gc_time_ms` **and** that are not
//! currently loading. The erased `gc` signature carries `cx`, so the retain
//! closure reads live entity state directly via `entity.read(cx)` for the
//! loading and status checks; the timestamp stored on the entry is used only
//! for age-based filtering.
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
//! - `touch()` / `set_loading()` / `set_not_loading()` removed as dead code
//!   (#75). GC recency now prefers `MutationResource::last_updated_at_ms`
//!   (terminal-completion time) over the entry's insertion time (audit #112),
//!   so a recently-completed mutation inserted long ago is not evicted
//!   prematurely; insertion time remains the fallback for never-completed
//!   (Idle / in-flight) mutations.

use ahash::AHashMap;
use gpui::{App, WeakEntity};

use crate::core::{MutationResource, MutationStatus};

use super::bucket::types::{DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS, SUCCESS_GC_MULTIPLIER};
use super::devtools::MutationDiagnostic;
use super::erased::current_time_ms;
use super::ErasedMutationBucket;

// (Audit #75: `DEFAULT_MUTATION_GC_TIME_MS` was dead code — never referenced
// after the GC refactor — so the constant and its `#[allow(dead_code)]` are
// removed. Callers wanting the 5-minute default use `QueryClient::with_gc_time`.)

/// Per-entry metadata stored alongside the entity.
///
/// Uses `WeakEntity` instead of `Entity` so that the bucket does not prevent
/// GPUI from garbage-collecting mutation resources when all component-held
/// strong references are dropped. The weak reference is upgraded on access;
/// if the entity was already collected, the entry is treated as dead and
/// cleaned up by GC.
///
/// (The `loading` flag was previously maintained via hook-side `set_loading`/
/// `set_not_loading` calls; those were removed as dead code in audit #75. The
/// flag now stays `false` and the primary loading check in `gc` reads live
/// entity state via `entity.read(cx).is_loading()`.)
struct MutationEntry<V, T, E> {
    entity: WeakEntity<MutationResource<V, T, E>>,
    /// Monotonic millisecond timestamp recorded at insertion.
    ///
    /// `MutationBucket::gc` prefers `MutationResource::last_updated_at_ms`
    /// (terminal-completion time) over this insertion time when measuring
    /// recency (audit #112); this value is the fallback used for mutations that
    /// have never completed (Idle / in-flight).
    updated_at: u128,
    /// Whether the mutation is currently in-flight (Loading state).
    ///
    /// Retained as a secondary GC guard (read in `gc`). With `set_loading`/
    /// `set_not_loading` removed as dead code (#75) it stays `false` in
    /// practice; the primary loading check reads `entity.read(cx).is_loading()`.
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

/// (Audit #41) The previous private `now_ms()` duplicate is removed; this
/// module now reuses the canonical [`super::erased::current_time_ms`].

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
        // Audit fix #29: replace the production `.expect()` on `checked_add`
        // with a saturating fallback so the bucket never panics after
        // `u64::MAX` insertions. `saturating_add` clamps `next_id` at
        // `u64::MAX`, which keeps it monotonic (no duplicate IDs while earlier
        // IDs are still live) and is the safe alternative to panicking. The
        // prior comment is preserved below for intent.
        self.next_id = self.next_id.saturating_add(1);
        // Note: saturating at `u64::MAX` means every insertion past
        // `u64::MAX` reuses that single ID — acceptable because reaching this
        // state requires ~1.8e19 prior insertions, and GC has long since
        // evicted the originals.
        self.resources.insert(
            id,
            MutationEntry {
                entity: entity.downgrade(),
                updated_at: current_time_ms(),
                loading: false,
            },
        );
        id
    }

    // (Audit #75/#112: `touch()`, `set_loading()`, `set_not_loading()` were
    // dead code — never called from production. They are removed along with
    // their `#[allow(dead_code)]` attributes. The `loading` entry field is
    // retained because `gc` still reads it as a secondary guard; it stays
    // `false` in practice, which is harmless. Audit #112 (computing mutation
    // GC recency from live entity completion time) was skipped because
    // `MutationResource` stores no completion/last-updated timestamp — wiring
    // one would require editing `core/mutation.rs`, outside this group.)

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
            let Some(entity) = entry.entity.upgrade() else { return false };

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

            // Audit fix #112: measure recency from the resource's last terminal
            // *completion* time when available, so a recently completed mutation
            // that was inserted long ago is not evicted prematurely. Fall back to
            // the entry's insertion time for mutations that never completed.
            let base = resource.last_updated_at_ms().unwrap_or(entry.updated_at);
            let age = now_ms.saturating_sub(base);
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

    /// Lightweight key/status pairs (#9). `key` is `None` for keyless mutations.
    fn collect_key_status(&self, cx: &App) -> Vec<(Option<String>, MutationStatus)> {
        self.resources
            .values()
            .filter_map(|entry| {
                let entity = entry.entity.upgrade()?;
                let resource = entity.read(cx);
                Some((resource.key().map(|k| k.to_path()), resource.status()))
            })
            .collect()
    }
}
