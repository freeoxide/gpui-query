//! Diagnostic types for query and mutation DevTools.

#[cfg(feature = "persist")]
use std::any::TypeId;

use crate::core::{MutationStatus, QueryStatus};

/// Diagnostic information about a single query resource.
#[derive(Clone, Debug)]
pub struct QueryDiagnostic {
    /// Full key path (e.g., "users::42::posts").
    pub key: String,
    /// Current status.
    pub status: QueryStatus,
    /// Cache policy label.
    pub cache_policy: String,
    /// Cache age in milliseconds, if available.
    pub cache_age_ms: Option<u64>,
    /// Number of cache hits.
    pub cache_hits: u64,
    /// Number of retries.
    pub retry_count: u32,
}

/// Diagnostic information about a single mutation resource.
#[derive(Clone, Debug)]
pub struct MutationDiagnostic {
    /// Optional key associated with this mutation.
    pub key: Option<String>,
    /// Current status.
    pub status: MutationStatus,
    /// Number of retries.
    pub retry_count: u32,
}

/// Aggregate diagnostic for the entire QueryClient.
#[derive(Clone, Debug, Default)]
pub struct ClientDiagnostic {
    /// Total number of tracked query resources.
    pub query_count: usize,
    /// Total number of tracked mutation resources.
    pub mutation_count: usize,
    /// Per-query diagnostics.
    pub queries: Vec<QueryDiagnostic>,
    /// Per-mutation diagnostics.
    pub mutations: Vec<MutationDiagnostic>,
}

// Dehydration types, gated behind `persist` alongside the
// dehydrate/hydrate/persist/restore methods and the `QueryPersister` trait.

/// A single entry in a dehydrated query cache snapshot, identified by its
/// key and the `TypeId` of its `(T, E)` type pair. `kind` distinguishes
/// queries, infinite queries, and mutations so consumers can deserialize
/// appropriately.
#[cfg(feature = "persist")]
#[derive(Clone, Debug)]
pub struct DehydratedEntry {
    /// Full key path (e.g., "users::42::posts").
    pub key: String,
    /// `TypeId` of the resource's `(T, E)` type pair; used to match entries
    /// to concrete types during hydration.
    pub type_id: TypeId,
    /// Whether this entry is a query, an infinite query, or a mutation.
    pub kind: &'static str,
}

/// A portable snapshot of all cached query state, produced by
/// [`QueryClient::dehydrate`](super::QueryClient::dehydrate) and consumed by
/// [`QueryClient::hydrate`](super::QueryClient::hydrate). Persist it to disk
/// or send it over a network for state restoration.
///
/// Because `QueryClient` uses type-erased buckets, `DehydratedState` stores
/// `TypeId` values but cannot deserialize typed data itself: callers that
/// know the concrete types should iterate `entries` and use
/// `QueryClient::set_query_data` for each matching entry.
///
/// # Example
///
/// ```
/// use gpui_query::client::{DehydratedState, DehydratedEntry};
/// use std::any::TypeId;
///
/// // DehydratedState can be constructed directly
/// let state = DehydratedState::default();
/// assert!(state.entries.is_empty());
///
/// // Entries can be created and added
/// let entries = vec![DehydratedEntry {
///     key: "users".to_string(),
///     type_id: TypeId::of::<(String, String)>(),
///     kind: "query",
/// }];
/// let state = DehydratedState { entries };
/// assert_eq!(state.entries.len(), 1);
/// ```
#[cfg(feature = "persist")]
#[derive(Clone, Debug, Default)]
pub struct DehydratedState {
    /// All dehydrated cache entries.
    pub entries: Vec<DehydratedEntry>,
}
