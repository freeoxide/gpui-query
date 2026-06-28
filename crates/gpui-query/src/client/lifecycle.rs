//! Lifecycle operations on `QueryClient`: GC, diagnostics, serialization,
//! persistence, and imperative fetch/prefetch.
//!
//! This module contains `impl QueryClient` methods for:
//! - Garbage collection (`gc`, `gc_with_time`)
//! - Test helpers for deterministic GC (snapshot updates, retain/release)
//! - Diagnostics
//! - Dehydration/hydration for state serialization
//! - Persistence via `QueryPersister`
//! - Imperative fetch and prefetch operations

use gpui::App;

use crate::client::devtools::ClientDiagnostic;
#[cfg(feature = "persist")]
use crate::client::devtools::{DehydratedEntry, DehydratedState};
use crate::client::prepared_fetch::PreparedFetch;
use crate::client::time::current_time_ms;
use crate::core::{CachePolicy, QueryKey, RequestPolicy};
#[cfg(feature = "persist")]
use crate::core::{MutationStatus, QueryStatus};

use super::QueryClient;
#[cfg(feature = "persist")]
use crate::client::erased::QueryPersister;

impl QueryClient {
    // ── Garbage collection ──────────────────────────────────────────────

    /// Run garbage collection on all buckets.
    ///
    /// Calls `current_time_ms()` internally to get the current time. If you
    /// already have a cached time value, use [`gc_with_time`] to avoid the
    /// syscall overhead (Audit 3, Finding 2).
    pub fn gc(&mut self, cx: &App) {
        let now_ms = current_time_ms();
        self.gc_with_time(now_ms, cx);
    }

    /// Run garbage collection with a pre-computed time value (Audit 3, Finding 2).
    ///
    /// Use this when you call GC frequently and want to amortize the cost of
    /// `SystemTime::now()` across multiple calls. The `now_ms` parameter should
    /// be milliseconds since the UNIX epoch (as returned by [`current_time_ms`]).
    ///
    /// **L5**: sets `self.last_gc_ms = now_ms` at the top so a *manual* GC call
    /// debounces the next opportunistic GC sweep (otherwise the caller's
    /// explicit `gc()` would not push back the `MIN_GC_TIME_MS` window and the
    /// next op could immediately re-trigger GC).
    pub fn gc_with_time(&mut self, now_ms: u64, cx: &App) {
        self.last_gc_ms = now_ms;
        for bucket in self.buckets.values_mut() {
            bucket.gc(now_ms, self.gc_time_ms, cx);
        }
        for bucket in self.infinite_buckets.values_mut() {
            bucket.gc(now_ms, self.gc_time_ms, cx);
        }
        for bucket in self.mutation_buckets.values_mut() {
            bucket.gc(now_ms, self.gc_time_ms, cx);
        }
    }

    // ── Test helpers ───────────────────────────────────────────────────
    //
    // The previous `update_*_snapshot` / `retain_*` / `release_*` helpers were
    // removed: GC now reads live entity state directly via `entity.read(cx)`
    // (audit #CL2/#106), so there is no cached `StatusSnapshot` to set; and
    // `observer_count` was removed (audit #8) in favor of `WeakEntity::upgrade()`
    // liveness, so there is no retain/release to drive. Tests that need a
    // specific GC state now simply transition the entity itself (e.g.
    // `apply_success`, `begin_fetch_next`) — GC observes that real state.

    // ── Diagnostics (Audit 3, Finding 7) ────────────────────────────────

    /// Get diagnostics for all queries and mutations.
    ///
    /// Returns aggregate counts and per-resource diagnostic details. The
    /// `queries` and `mutations` vectors are populated by iterating all bucket
    /// entries, upgrading weak references, and reading entity state. Dead
    /// entries (collected entities) are skipped.
    ///
    /// **Audit 3 fix**: Previously returned empty `queries: Vec::new()` and
    /// `mutations: Vec::new()` vectors. Now fully populates per-resource
    /// diagnostics via `collect_diagnostics` on each erased bucket.
    pub fn diagnostics(&self, cx: &App) -> ClientDiagnostic {
        let now_ms = current_time_ms();
        // L1: pre-size the diagnostic Vecs from the bucket `count()` sums so the
        // per-bucket `collect_diagnostics_into` pushes (L3) don't repeatedly
        // reallocate the destination Vec as it grows. `count()` is
        // `entries.len()` — exact for live entries, an upper bound for the
        // diagnostics (dead entries are skipped), so this never under-allocates.
        let mut query_count = 0;
        let mut mutation_count = 0;
        for bucket in self.buckets.values() {
            query_count += bucket.count();
        }
        for bucket in self.infinite_buckets.values() {
            query_count += bucket.count();
        }
        for bucket in self.mutation_buckets.values() {
            mutation_count += bucket.count();
        }
        let mut queries = Vec::with_capacity(query_count);
        let mut mutations = Vec::with_capacity(mutation_count);

        // L3: push each bucket's diagnostics straight into the single pre-sized
        // Vec via the sink variant — avoids the per-bucket `Vec` allocation +
        // `extend` that the returning `collect_diagnostics` variant forces.
        for bucket in self.buckets.values() {
            bucket.collect_diagnostics_into(now_ms, cx, &mut queries);
        }
        for bucket in self.infinite_buckets.values() {
            bucket.collect_diagnostics_into(now_ms, cx, &mut queries);
        }
        for bucket in self.mutation_buckets.values() {
            bucket.collect_diagnostics_into(cx, &mut mutations);
        }

        ClientDiagnostic {
            query_count,
            mutation_count,
            queries,
            mutations,
        }
    }

    // ── Serialization / Hydration (Audit 3, Finding 8) ──────────────────

    /// Serialize all cached query state into a portable format.
    ///
    /// Extracts all live query resources, recording their keys, status, and
    /// type information. The resulting [`DehydratedState`] can be persisted
    /// to disk or stored for later restoration via [`hydrate`].
    ///
    /// Only resources with `Success` status are included. Resources in
    /// `Idle`, `Loading`, `Failure`, or `Cancelled` states are skipped.
    ///
    /// **Note**: Full data serialization requires type-specific code at the
    /// call site. Use `get_query_data::<T, E>(key, cx)` to extract typed
    /// data and serialize it externally. The `DehydratedState` provides
    /// the metadata (keys, type IDs) needed for typed restoration.
    #[cfg(feature = "persist")]
    pub fn dehydrate(&self, cx: &App) -> DehydratedState {
        // L2: pre-size the entries Vec from the bucket `count()` sums. Only
        // `Success` entries are pushed, so this is an upper bound — never
        // under-allocates, avoids reallocation churn as entries accumulate.
        // (The three maps hold different erased trait objects, so they are
        // summed separately rather than chained.)
        let cap = self.buckets.values().map(|b| b.count()).sum::<usize>()
            + self
                .infinite_buckets
                .values()
                .map(|b| b.count())
                .sum::<usize>()
            + self
                .mutation_buckets
                .values()
                .map(|b| b.count())
                .sum::<usize>();
        let mut entries = Vec::with_capacity(cap);

        // Audit fix #L13: collapse all three loops (query / infinite / mutation)
        // into a single helper. The previous shape used a `push_status_queries`
        // closure that handled only the two query-shaped loops (both
        // `Vec<(String, QueryStatus)>`) and left the mutation loop inlined
        // separately — its items are `(Option<String>, MutationStatus)`, so it
        // couldn't reuse the closure. `push_status` below is generic over the
        // status type and the success sentinel, so all three kinds share one
        // code path. The emitted `DehydratedState` JSON shape is byte-identical
        // to the previous implementation (only entries whose real status equals
        // the success sentinel are pushed).
        //
        // Audit fix #L14: the `data_json` field is gone (it was always `None`),
        // so we no longer pass the dead initializer here.
        //
        // Audit fix #94 / #9: this still drives the lightweight `collect_key_status`
        // (key + status only), skipping the per-entry allocations that
        // `collect_diagnostics` builds (`cache_policy`, `cache_age_ms`,
        // `cache_hits`, `retry_count`).
        //
        // Audit fix #113: mutations are included; keyless mutations are skipped
        // (a keyless mutation can't be meaningfully addressed for typed
        // restoration).
        fn push_status<S>(
            entries: &mut Vec<DehydratedEntry>,
            type_id: std::any::TypeId,
            pairs: impl IntoIterator<Item = (Option<String>, S)>,
            success: S,
            kind: &'static str,
        ) where
            S: PartialEq,
        {
            for (key, status) in pairs {
                if status == success
                    && let Some(key) = key
                {
                    entries.push(DehydratedEntry { key, type_id, kind });
                }
            }
        }

        // L3: reuse two buffers across all buckets instead of allocating a fresh
        // `Vec` per bucket (the returning `collect_key_status` variant). Each
        // bucket appends into the shared buffer via the sink; the buffer is
        // drained per bucket so it never grows unbounded and the keys move
        // (no clone) into `entries`.
        let mut q_pairs: Vec<(String, QueryStatus)> = Vec::new();
        let mut m_pairs: Vec<(Option<String>, MutationStatus)> = Vec::new();

        for (type_id, bucket) in &self.buckets {
            bucket.collect_key_status_into(cx, &mut q_pairs);
            push_status(
                &mut entries,
                *type_id,
                q_pairs.drain(..).map(|(k, s)| (Some(k), s)),
                QueryStatus::Success,
                "query",
            );
        }
        for (type_id, bucket) in &self.infinite_buckets {
            bucket.collect_key_status_into(cx, &mut q_pairs);
            push_status(
                &mut entries,
                *type_id,
                q_pairs.drain(..).map(|(k, s)| (Some(k), s)),
                QueryStatus::Success,
                "infinite",
            );
        }
        for (type_id, bucket) in &self.mutation_buckets {
            bucket.collect_key_status_into(cx, &mut m_pairs);
            push_status(
                &mut entries,
                *type_id,
                m_pairs.drain(..),
                MutationStatus::Success,
                "mutation",
            );
        }

        DehydratedState { entries }
    }

    /// Restore query state from a previously dehydrated snapshot.
    ///
    /// Full hydration requires type-specific deserialization. The `DehydratedState`
    /// contains `type_id` keys but downcasting requires knowing the concrete types
    /// at the call site. Callers should iterate `state.entries` and call
    /// `set_query_data::<T, E>()` for each entry where they know the types.
    ///
    /// This method is provided as a hook point for typed hydration and to
    /// document the intended API shape matching TanStack Query's
    /// `queryClient.hydrate()`.
    #[cfg(feature = "persist")]
    pub fn hydrate(&mut self, _state: DehydratedState, _cx: &mut App) {
        // Full hydration requires type-specific deserialization. The DehydratedState
        // contains type_id keys but downcasting requires knowing the concrete types
        // at the call site. Callers should iterate state.entries and call
        // set_query_data::<T, E> for each entry where they know the types.
    }

    // ── Persistence (Audit 3, Finding 9) ────────────────────────────────

    /// Persist all cached data using the provided persister.
    ///
    /// Dehydrates the current state and saves it via the persister. This can
    /// be called periodically (e.g., during GC) or on app shutdown to ensure
    /// cached data survives across app restarts.
    #[cfg(feature = "persist")]
    pub fn persist(&self, persister: &dyn QueryPersister, cx: &App) {
        let state = self.dehydrate(cx);
        persister.save(state.entries);
    }

    /// Restore cached data from a persister.
    ///
    /// Loads entries from the persister. Since type information is erased in
    /// the persister, callers must iterate and restore typed data themselves
    /// using `set_query_data`. This method loads the raw entries and returns
    /// them for inspection and typed restoration.
    ///
    /// **L4**: this is an *associated* function rather than a method — it does
    /// not read any `&self` state, so callers invoke it as
    /// `QueryClient::restore(&persister)` instead of `client.restore(...)`,
    /// avoiding the need for a borrow on the client.
    #[cfg(feature = "persist")]
    pub fn restore(persister: &dyn QueryPersister) -> Vec<DehydratedEntry> {
        persister.load()
    }

    // ── Imperative fetch (Audit 3, Finding 10) ──────────────────────────

    /// Prepare an imperative fetch for a query key, creating the resource if needed.
    ///
    /// This creates (or reuses) the resource entity and begins a forced request,
    /// returning a [`PreparedFetch`] containing the entity, request ID, and signal.
    /// The caller is responsible for calling the fetcher and completing the request
    /// using `complete_fetch` or by directly calling `complete_current_success` /
    /// `complete_current_failure` on the entity.
    ///
    /// This is the equivalent of TanStack Query's `queryClient.fetchQuery()`.
    /// Unlike `use_query`, this does not subscribe or create an observer.
    ///
    /// Returns `None` if the cache is fresh (cache hit) and no fetch is needed.
    /// In that case, use `get_query_data` to read the cached data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gpui_query::client::QueryClient;
    /// use gpui_query::core::QueryKey;
    /// # #[derive(Clone)]
    /// # struct UserData;
    /// # #[derive(Clone, Debug)]
    /// # struct QueryError;
    /// # fn _doc(client: &mut QueryClient, cx: &mut gpui::App) {
    ///
    /// if let Some(prepared) = client.prepare_fetch_query::<UserData, QueryError>(
    ///     QueryKey::from("user/42"),
    ///     cx,
    /// ) {
    ///     // prepared.entity, prepared.signal, and prepared.request_id are now available.
    ///     // Use cx.spawn() to run your async fetcher, then call
    ///     // prepared.complete_success(data, cx) or prepared.complete_failure(e, cx).
    /// }
    /// # }
    /// ```
    pub fn prepare_fetch_query<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: impl Into<QueryKey>,
        cx: &mut App,
    ) -> Option<PreparedFetch<T, E>> {
        let key = key.into();
        let entity = self.resource::<T, E>(key.clone(), cx);
        let now_ms = current_time_ms();

        // Get or create a request ID via the bucket's sequencer
        let request_id = self.next_request_id_for_key::<T, E>(&key);

        // Begin the request on the resource purely for its side effect.
        // **L7**: the previous code captured a `started` boolean from
        // `begin_request_with_id`, matched it exhaustively, and then discarded
        // it via `let _ = started;` — `prepare_fetch_query` (force mode)
        // always returns a `PreparedFetch` regardless, so the value was
        // useless. We now call `begin_request_with_id` for its side effect
        // only, dropping the dead match + binding.
        entity.update(cx, |resource, _| {
            if let Some(rid) = request_id {
                let _ = resource.begin_request_with_id(
                    Some(rid),
                    now_ms,
                    crate::core::QueryFetchMode::Force,
                );
            }
        });

        // Re-read to get the signal and request ID
        let (request_id, signal) = entity.read_with(cx, |resource, _| {
            let rid = resource.active_request_id()?;
            let signal = resource.signal().cloned()?;
            Some((rid, signal))
        })?;

        Some(PreparedFetch {
            entity,
            request_id,
            signal,
            now_ms,
        })
    }

    // ── Prefetch (Audit 3, Finding 11) ──────────────────────────────────

    /// Prepare a prefetch for a key that will be needed soon.
    ///
    /// Creates the resource entity (or reuses an existing one) and begins a
    /// request if the cache is stale or empty. The resource is NOT subscribed
    /// -- no observer is attached. When a component later calls `use_query`
    /// with the same key, it will find the prefetched data in the cache.
    ///
    /// This is the equivalent of TanStack Query's `queryClient.prefetchQuery()`.
    ///
    /// If the resource already has fresh data (cache hit), returns `None`.
    /// Use `prepare_fetch_query` with forced mode to override this behavior.
    ///
    /// Returns a [`PreparedFetch`] containing the entity, request ID, and
    /// signal. The caller is responsible for calling the fetcher and completing
    /// the request.
    pub fn prepare_prefetch_query<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Option<PreparedFetch<T, E>> {
        let key = key.into();
        let entity =
            self.resource_with_policies::<T, E>(key.clone(), cache_policy, request_policy, cx);
        let now_ms = current_time_ms();

        // Get request ID from sequencer
        let request_id = self.next_request_id_for_key::<T, E>(&key);

        // Begin the request (respects cache policy — will skip if fresh)
        let started = entity.update(cx, |resource, _| {
            if let Some(rid) = request_id {
                let result = resource.begin_request_with_id(
                    Some(rid),
                    now_ms,
                    crate::core::QueryFetchMode::Normal,
                );
                matches!(
                    result,
                    crate::core::QueryBeginResult::Started { .. }
                        | crate::core::QueryBeginResult::StaleCacheHit { .. }
                )
            } else {
                false
            }
        });

        if !started {
            return None;
        }

        // Re-read to get the signal and request ID
        let (request_id, signal) = entity.read_with(cx, |resource, _| {
            let rid = resource.active_request_id()?;
            let signal = resource.signal().cloned()?;
            Some((rid, signal))
        })?;

        Some(PreparedFetch {
            entity,
            request_id,
            signal,
            now_ms,
        })
    }
}
