//! Mutations have no query key: entries are keyed by a generated numeric id,
//! and GC measures recency from the completion time (insertion time for
//! mutations that never completed).

use ahash::AHashMap;
use gpui::{App, WeakEntity};

use crate::core::{MutationResource, MutationStatus};

use super::ErasedMutationBucket;
use super::bucket::types::{DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS, SUCCESS_GC_MULTIPLIER};
use super::devtools::MutationDiagnostic;

/// `last_updated_ms` / `loading` mirror the entity, refreshed wherever the
/// bucket already reads it, so `evict_oldest` scans cheap fields and
/// confirms its winner with a single entity read.
struct MutationEntry<V, T, E> {
    entity: WeakEntity<MutationResource<V, T, E>>,
    updated_at: u64,
    last_updated_ms: Option<u64>,
    loading: bool,
}

pub struct MutationBucket<V, T, E> {
    resources: AHashMap<u64, MutationEntry<V, T, E>>,
    next_id: u64,
    /// Entries allowed before the oldest one is evicted.
    max_entries: usize,
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

    /// Skips loading entries; the winner gets one confirming entity read
    /// (the mirror can be stale if a fetch began after the last refresh),
    /// and each stale re-check marks the mirror and re-picks.
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        loop {
            let target = self
                .resources
                .iter()
                .filter_map(|(id, entry)| {
                    if entry.loading {
                        return None;
                    }
                    entry.entity.upgrade()?;
                    Some((*id, entry.last_updated_ms.unwrap_or(entry.updated_at)))
                })
                .min_by_key(|&(_, age)| age);

            let Some((id, _)) = target else {
                return;
            };

            let still_loading = self
                .resources
                .get(&id)
                .and_then(|e| e.entity.upgrade())
                .map(|entity| entity.read(cx).is_loading());

            match still_loading {
                Some(true) => {
                    if let Some(entry) = self.resources.get_mut(&id) {
                        entry.loading = true;
                    }
                }
                _ => {
                    self.resources.remove(&id);
                    return;
                }
            }
        }
    }

    /// `next_id` saturates at `u64::MAX`: staying monotonic matters more
    /// than uniqueness after ~1.8e19 insertions, which GC has long outlived.
    pub(crate) fn insert(
        &mut self,
        entity: &gpui::Entity<MutationResource<V, T, E>>,
        now_ms: u64,
        cx: &App,
    ) -> u64 {
        if self.resources.len() >= self.max_entries {
            self.evict_oldest(cx);
        }

        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.resources.insert(
            id,
            MutationEntry {
                entity: entity.downgrade(),
                updated_at: now_ms,
                last_updated_ms: None,
                loading: false,
            },
        );
        id
    }

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

    /// Loading always survives; `Success` survives
    /// `SUCCESS_GC_MULTIPLIER * gc_time_ms`, `Idle`/`Failure` survive
    /// `gc_time_ms`. The `loading` mirror is checked first so a mid-flight
    /// mutation whose weak ref cannot upgrade survives one cycle.
    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        let gc_threshold = gc_time_ms.max(MIN_GC_TIME_MS);
        let success_threshold = gc_threshold.saturating_mul(SUCCESS_GC_MULTIPLIER as u64);

        self.resources.retain(|_id, entry| {
            if entry.loading {
                return true;
            }

            let Some(entity) = entry.entity.upgrade() else {
                return false;
            };

            let resource = entity.read(cx);
            let last_updated = resource.last_updated_at_ms();
            entry.last_updated_ms = last_updated;
            entry.loading = resource.is_loading();

            if entry.loading {
                return true;
            }

            let threshold = match resource.status() {
                MutationStatus::Success => success_threshold,
                MutationStatus::Idle | MutationStatus::Failure => gc_threshold,
                MutationStatus::Loading => return true,
            };

            let base = last_updated.unwrap_or(entry.updated_at);
            now_ms.saturating_sub(base) < threshold
        });
    }

    fn count(&self) -> usize {
        self.resources.len()
    }

    fn collect_diagnostics_into(&self, cx: &App, out: &mut Vec<MutationDiagnostic>) {
        for entry in self.resources.values() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push(MutationDiagnostic {
                key: resource.key().map(|k| k.to_path()),
                status: resource.status(),
                retry_count: resource.retry_count(),
            });
        }
    }

    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &App, out: &mut Vec<(Option<String>, MutationStatus)>) {
        for entry in self.resources.values() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push((resource.key().map(|k| k.to_path()), resource.status()));
        }
    }
}
