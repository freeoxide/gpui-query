//! `ResourceBucket<R>` holds everything `QueryBucket` and
//! `InfiniteQueryBucket` do identically (get-or-create, eviction, GC, bulk
//! matching, diagnostics); the public bucket types only add erased-trait
//! impls and persistence specifics.

use ahash::AHashMap;
use gpui::{App, AppContext as _, Entity};

use crate::client::devtools::QueryDiagnostic;
use crate::core::{
    CachePolicy, InfiniteQueryResource, QueryKey, QueryKeyFilter, QueryResource, QueryStatus,
    RequestId, RequestPolicy,
};

use super::types::{BucketEntry, DEFAULT_MAX_ENTRIES, MIN_GC_TIME_MS, SUCCESS_GC_MULTIPLIER};

/// Runs GC every this many resource operations, so it fires in production
/// without anyone calling `gc()` by hand.
pub(crate) const GC_INTERVAL: usize = 64;

/// The resource surface `ResourceBucket` needs for both query kinds;
/// prefixed names keep the delegating impls unambiguous.
pub(crate) trait BucketResource {
    fn new_resource(key: QueryKey, cache_policy: CachePolicy, request_policy: RequestPolicy)
    -> Self;
    fn resource_status(&self) -> QueryStatus;
    fn resource_is_loading(&self) -> bool;
    fn resource_last_updated(&self) -> Option<u64>;
    fn resource_cache_policy(&self) -> CachePolicy;
    fn resource_request_policy(&self) -> RequestPolicy;
    fn set_resource_cache_policy(&mut self, policy: CachePolicy);
    fn set_resource_request_policy(&mut self, policy: RequestPolicy);
    fn resource_cache_age_ms(&self, now_ms: u64) -> Option<u64>;
    fn resource_cache_hits(&self) -> u64;
    fn resource_retry_count(&self) -> u32;
}

impl<T: 'static, E: 'static> BucketResource for QueryResource<T, E> {
    fn new_resource(
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
    ) -> Self {
        Self::new(key, cache_policy, request_policy)
    }
    fn resource_status(&self) -> QueryStatus {
        self.status()
    }
    fn resource_is_loading(&self) -> bool {
        self.is_loading()
    }
    fn resource_last_updated(&self) -> Option<u64> {
        self.last_updated_at_ms()
    }
    fn resource_cache_policy(&self) -> CachePolicy {
        self.cache_policy()
    }
    fn resource_request_policy(&self) -> RequestPolicy {
        self.request_policy()
    }
    fn set_resource_cache_policy(&mut self, policy: CachePolicy) {
        self.set_cache_policy(policy);
    }
    fn set_resource_request_policy(&mut self, policy: RequestPolicy) {
        self.set_request_policy(policy);
    }
    fn resource_cache_age_ms(&self, now_ms: u64) -> Option<u64> {
        self.cache_age_ms(now_ms)
    }
    fn resource_cache_hits(&self) -> u64 {
        self.cache_hits()
    }
    fn resource_retry_count(&self) -> u32 {
        self.retry_count()
    }
}

impl<T: 'static, E: 'static> BucketResource for InfiniteQueryResource<T, E> {
    fn new_resource(
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
    ) -> Self {
        Self::new(key, cache_policy, request_policy)
    }
    fn resource_status(&self) -> QueryStatus {
        self.status()
    }
    fn resource_is_loading(&self) -> bool {
        self.is_loading()
    }
    fn resource_last_updated(&self) -> Option<u64> {
        self.last_updated_at_ms()
    }
    fn resource_cache_policy(&self) -> CachePolicy {
        self.cache_policy()
    }
    fn resource_request_policy(&self) -> RequestPolicy {
        self.request_policy()
    }
    fn set_resource_cache_policy(&mut self, policy: CachePolicy) {
        self.set_cache_policy(policy);
    }
    fn set_resource_request_policy(&mut self, policy: RequestPolicy) {
        self.set_request_policy(policy);
    }
    fn resource_cache_age_ms(&self, now_ms: u64) -> Option<u64> {
        self.cache_age_ms(now_ms)
    }
    fn resource_cache_hits(&self) -> u64 {
        self.cache_hits()
    }
    fn resource_retry_count(&self) -> u32 {
        self.retry_count()
    }
}

pub(crate) struct ResourceBucket<R> {
    pub(crate) entries: AHashMap<QueryKey, BucketEntry<R>>,
    pub(crate) max_entries: usize,
}

impl<R: BucketResource + 'static> ResourceBucket<R> {
    pub(crate) fn new() -> Self {
        Self {
            entries: AHashMap::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    pub(crate) fn get_or_create(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Entity<R> {
        self.get_or_create_impl(key, cache_policy, request_policy, cx, false)
            .0
    }

    /// Mints the next `RequestId` from the entry's sequencer in the same
    /// lookup.
    pub(crate) fn get_or_create_with_request_id(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> (Entity<R>, RequestId) {
        let (entity, request_id) =
            self.get_or_create_impl(key, cache_policy, request_policy, cx, true);
        (entity, request_id.expect("impl inserts the entry before returning"))
    }

    /// Live entries refresh differing policies in place; a dead weak
    /// reference is overwritten in place (length unchanged, no eviction),
    /// while a vacant insert at capacity evicts the oldest entry first.
    fn get_or_create_impl(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
        mint_request_id: bool,
    ) -> (Entity<R>, Option<RequestId>) {
        if let Some(entry) = self.entries.get_mut(&key) {
            if let Some(entity) = entry.entity.upgrade() {
                let (needs_update, last_updated, loading) =
                    entity.read_with(cx, |resource, _| {
                        let needs_update = resource.resource_cache_policy() != cache_policy
                            || resource.resource_request_policy() != request_policy;
                        (
                            needs_update,
                            resource.resource_last_updated(),
                            resource.resource_is_loading(),
                        )
                    });
                entry.last_updated_ms = last_updated;
                entry.loading = loading;
                if needs_update {
                    entity.update(cx, |resource, _| {
                        resource.set_resource_cache_policy(cache_policy);
                        resource.set_resource_request_policy(request_policy);
                    });
                }
                let request_id = mint_request_id.then(|| entry.sequencer.next_request());
                return (entity, request_id);
            }
        } else if self.entries.len() >= self.max_entries {
            self.evict_oldest(cx);
        }

        let mut sequencer = crate::core::RequestSequencer::new();
        let request_id = mint_request_id.then(|| sequencer.next_request());
        let entity = cx.new(|_| R::new_resource(key.clone(), cache_policy, request_policy));
        self.entries.insert(
            key,
            BucketEntry {
                entity: entity.downgrade(),
                sequencer,
                last_updated_ms: None,
                loading: false,
            },
        );
        (entity, request_id)
    }

    /// Scans only the mirrors plus weak-ref liveness, then confirms
    /// `!is_loading()` on the winner with one entity read (the mirror can be
    /// stale if a fetch began after the last refresh). Each retry marks the
    /// stale mirror and re-picks, so the candidate set strictly shrinks.
    pub(crate) fn evict_oldest(&mut self, cx: &App) {
        loop {
            let target = self
                .entries
                .iter()
                .filter_map(|(key, entry)| {
                    if entry.loading {
                        return None;
                    }
                    entry.entity.upgrade()?;
                    Some((key, entry.last_updated_ms.unwrap_or(0)))
                })
                .min_by_key(|&(_, age)| age);

            let Some((key, _)) = target else {
                return; // every live entry is loading: nothing safe to evict
            };

            let key = key.clone();
            let still_loading = self
                .entries
                .get(&key)
                .and_then(|e| e.entity.upgrade())
                .map(|entity| entity.read(cx).resource_is_loading());

            match still_loading {
                Some(true) => {
                    if let Some(entry) = self.entries.get_mut(&key) {
                        entry.loading = true;
                    }
                }
                _ => {
                    self.entries.remove(&key);
                    return;
                }
            }
        }
    }

    pub(crate) fn get(&self, key: &QueryKey) -> Option<Entity<R>> {
        self.entries.get(key).and_then(|e| e.entity.upgrade())
    }

    pub(crate) fn sequencer_mut(&mut self, key: &QueryKey) -> Option<&mut crate::core::RequestSequencer> {
        self.entries.get_mut(key).map(|e| &mut e.sequencer)
    }

    pub(crate) fn all_entities(&self) -> Vec<Entity<R>> {
        self.entries
            .values()
            .filter_map(|e| e.entity.upgrade())
            .collect()
    }

    /// Collects matching entities up front, then runs `action` on each
    /// outside the map borrow. GPUI defers observer effects to the
    /// outermost update, so no action can re-enter this bucket mid-loop.
    pub(crate) fn for_each_matching_entry(
        &mut self,
        filter: &QueryKeyFilter,
        cx: &mut App,
        mut action: impl FnMut(&Entity<R>, &mut App),
    ) {
        let entities: Vec<Entity<R>> = self
            .entries
            .iter()
            .filter(|(key, _)| filter.matches(key))
            .filter_map(|(_, entry)| entry.entity.upgrade())
            .collect();

        for entity in &entities {
            action(entity, cx);
        }
    }

    /// Loading always survives; `Success` survives while its cache policy
    /// can still serve it and until `SUCCESS_GC_MULTIPLIER * gc_time_ms`;
    /// `Idle`/`Failure`/`Cancelled` survive `gc_time_ms`. Entries without a
    /// completion timestamp count as fully aged.
    pub(crate) fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &App) {
        let gc_threshold = gc_time_ms.max(MIN_GC_TIME_MS);
        let success_threshold = gc_threshold.saturating_mul(SUCCESS_GC_MULTIPLIER as u64);

        self.entries.retain(|_key, entry| {
            let Some(entity) = entry.entity.upgrade() else {
                return false;
            };
            let resource = entity.read(cx);

            entry.last_updated_ms = resource.resource_last_updated();
            entry.loading = resource.resource_is_loading();

            if resource.resource_is_loading() {
                return true;
            }

            let status = resource.resource_status();

            let age_ms = resource
                .resource_last_updated()
                .map(|updated| now_ms.saturating_sub(updated))
                .unwrap_or(gc_threshold);

            if status == QueryStatus::Success {
                let cache_policy = resource.resource_cache_policy();
                if cache_policy.can_serve_stale() && !cache_policy.is_expired(age_ms) {
                    return true;
                }
                return age_ms < success_threshold;
            }

            if !matches!(
                status,
                QueryStatus::Idle | QueryStatus::Failure | QueryStatus::Cancelled
            ) {
                return true;
            }

            age_ms < gc_threshold
        });
    }

    pub(crate) fn collect_diagnostics_into(
        &self,
        now_ms: u64,
        cx: &App,
        out: &mut Vec<QueryDiagnostic>,
    ) {
        for (key, entry) in self.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push(QueryDiagnostic {
                key: key.to_path(),
                status: resource.resource_status(),
                cache_policy: resource.resource_cache_policy().label(),
                cache_age_ms: resource.resource_cache_age_ms(now_ms),
                cache_hits: resource.resource_cache_hits(),
                retry_count: resource.resource_retry_count(),
            });
        }
    }

    /// Key/status pairs without the per-entry allocations of full
    /// diagnostics; used by `dehydrate`.
    #[cfg(feature = "persist")]
    pub(crate) fn collect_key_status_into(&self, cx: &App, out: &mut Vec<(String, QueryStatus)>) {
        for (key, entry) in self.entries.iter() {
            let Some(entity) = entry.entity.upgrade() else {
                continue;
            };
            let resource = entity.read(cx);
            out.push((key.to_path(), resource.resource_status()));
        }
    }
}
