//! Lifecycle operations on `QueryClient`: GC, diagnostics, serialization,
//! legacy persistence, and imperative fetch/prefetch.

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
    /// Calls `current_time_ms()` internally; if you already have a cached
    /// time value, use [`gc_with_time`](Self::gc_with_time) to avoid the
    /// syscall.
    pub fn gc(&mut self, cx: &App) {
        let now_ms = current_time_ms();
        self.gc_with_time(now_ms, cx);
    }

    /// Run garbage collection with a pre-computed time value (milliseconds
    /// since the UNIX epoch), amortizing `SystemTime::now()` across calls.
    ///
    /// Also stamps `last_gc_ms` so a manual GC debounces the next
    /// opportunistic sweep.
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
        // Metadata for keys whose entries were evicted can never be collected
        // again; drop it so churning keys cannot grow the map without bound.
        #[cfg(feature = "persist")]
        if let Some(meta) = self.persisted_meta.as_mut() {
            meta.retain(|key, _| {
                self.buckets.values().any(|b| b.contains_key(key))
                    || self.infinite_buckets.values().any(|b| b.contains_key(key))
            });
        }
    }

    // ── Diagnostics ─────────────────────────────────────────────────────

    /// Get diagnostics for all queries and mutations.
    ///
    /// Returns aggregate counts plus per-resource details, collected by
    /// iterating bucket entries, upgrading weak references, and reading
    /// entity state. Dead entries (collected entities) are skipped, so the
    /// counts are an upper bound on the returned vectors.
    pub fn diagnostics(&self, cx: &App) -> ClientDiagnostic {
        let now_ms = current_time_ms();
        // Pre-size from the bucket counts (entries.len()) so the per-bucket
        // pushes never reallocate.
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

    // ── Serialization / hydration ───────────────────────────────────────

    /// Serialize cached query state into a portable format: keys, status,
    /// and type information for every live `Success` resource (other
    /// statuses are skipped). The resulting [`DehydratedState`] can be
    /// persisted or restored via [`hydrate`](Self::hydrate).
    ///
    /// Full data serialization needs type-specific code at the call site:
    /// use [`get_query_data`](Self::get_query_data) to extract typed data
    /// and serialize it externally. `DehydratedState` carries the metadata
    /// (keys, type IDs) needed for typed restoration.
    #[cfg(feature = "persist")]
    pub fn dehydrate(&self, cx: &App) -> DehydratedState {
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

        // Two scratch buffers reused across buckets; drained per bucket so
        // they never grow and the keys move into `entries` without cloning.
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
    /// Full hydration requires type-specific deserialization:
    /// `DehydratedState` stores `type_id` keys, but downcasting needs the
    /// concrete types at the call site. Callers should iterate
    /// `state.entries` and call `set_query_data::<T, E>()` for each entry
    /// whose types they know. This hook point mirrors TanStack Query's
    /// `queryClient.hydrate()`.
    #[cfg(feature = "persist")]
    pub fn hydrate(&mut self, _state: DehydratedState, _cx: &mut App) {}

    // ── Persistence ─────────────────────────────────────────────────────

    /// Persist the dehydrated state via the provided persister. Can be
    /// called periodically (e.g. during GC) or on app shutdown.
    #[cfg(feature = "persist")]
    pub fn persist(&self, persister: &dyn QueryPersister, cx: &App) {
        let state = self.dehydrate(cx);
        persister.save(state.entries);
    }

    /// Load entries from a persister. Type information is erased in the
    /// persister, so callers iterate and restore typed data themselves via
    /// `set_query_data`. An associated function: it reads no client state,
    /// so it needs no borrow on the client.
    #[cfg(feature = "persist")]
    pub fn restore(persister: &dyn QueryPersister) -> Vec<DehydratedEntry> {
        persister.load()
    }

    // ── Imperative fetch ────────────────────────────────────────────────

    /// Prepare an imperative fetch for a query key, creating the resource if
    /// needed, and begin a forced request. Returns a [`PreparedFetch`] with
    /// the entity, request ID, and signal; the caller runs the fetcher and
    /// completes the request via `complete_success` / `complete_failure`.
    ///
    /// The equivalent of TanStack Query's `queryClient.fetchQuery()`.
    /// Unlike `use_query`, this does not subscribe or create an observer.
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
        let now_ms = current_time_ms();
        let (entity, request_id) = self.resource_with_request_id::<T, E>(
            key,
            self.default_cache_policy,
            self.default_request_policy,
            cx,
        );

        // Begin the request and pull the signal from the same update.
        let (request_id, signal) = entity.update(cx, |resource, _| {
            let _ = resource.begin_request_with_id(
                Some(request_id),
                now_ms,
                crate::core::QueryFetchMode::Force,
            );
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

    // ── Prefetch ────────────────────────────────────────────────────────

    /// Prepare a prefetch for a key that will be needed soon: creates (or
    /// reuses) the resource and begins a request if the cache is stale or
    /// empty. No observer is attached; a later `use_query` with the same key
    /// finds the prefetched data.
    ///
    /// The equivalent of TanStack Query's `queryClient.prefetchQuery()`.
    /// Returns `None` on a fresh cache hit (use
    /// [`get_query_data`](Self::get_query_data) to read it) or when the
    /// request policy ignored the start.
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
        let now_ms = current_time_ms();
        let (entity, request_id) =
            self.resource_with_request_id::<T, E>(key, cache_policy, request_policy, cx);

        // Normal mode respects the cache policy; only Started and
        // StaleCacheHit mean a fetch is actually wanted.
        let (request_id, signal) = entity.update(cx, |resource, _| {
            let started = matches!(
                resource.begin_request_with_id(
                    Some(request_id),
                    now_ms,
                    crate::core::QueryFetchMode::Normal
                ),
                crate::core::QueryBeginResult::Started { .. }
                    | crate::core::QueryBeginResult::StaleCacheHit { .. }
            );
            if !started {
                return None;
            }
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
