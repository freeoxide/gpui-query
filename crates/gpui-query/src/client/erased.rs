//! Type-erased bucket traits and persistence adapter.
//!
//! These traits let `QueryClient` store heterogeneous buckets in
//! `AHashMap<TypeId, Box<dyn Erased*>>` maps, dispatching to concrete types
//! only when the caller provides generic parameters. The persistence-only
//! surface is gated behind the `persist` feature.

use crate::client::devtools::{MutationDiagnostic, QueryDiagnostic};
#[cfg(feature = "persist")]
use crate::client::persist::{PersistedEntry, SerializerRegistry};
use crate::core::QueryKeyFilter;
#[cfg(feature = "persist")]
use crate::core::{MutationStatus, QueryStatus};

/// Type-erased bucket trait for storage in a homogeneous map.
pub(crate) trait ErasedBucket {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &gpui::App);
    fn count(&self) -> usize;
    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut gpui::App);
    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut gpui::App);
    fn remove_matching(&mut self, filter: &QueryKeyFilter);
    fn cancel_matching(&mut self, filter: &QueryKeyFilter, cx: &mut gpui::App);
    /// Push each live entry's diagnostic into the caller-supplied Vec so
    /// `QueryClient::diagnostics` can pre-size one destination instead of
    /// allocating per bucket.
    fn collect_diagnostics_into(&self, now_ms: u64, cx: &gpui::App, out: &mut Vec<QueryDiagnostic>);
    /// Key/status pairs without the per-entry allocations of full
    /// diagnostics; used by `dehydrate`.
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &gpui::App, out: &mut Vec<(String, QueryStatus)>);
    /// Push each `Success` entry's `(key, entry)` pair into `out`, serializing
    /// via the caller-supplied registry. Entries whose `T` has no registered
    /// serializer are skipped.
    #[cfg(feature = "persist")]
    fn collect_persistable_into(
        &self,
        cx: &gpui::App,
        serializers: &SerializerRegistry,
        now_ms: u64,
        out: &mut Vec<(crate::core::QueryKey, PersistedEntry)>,
    );
    /// Whether the bucket currently holds `key`. Used to prune the
    /// persistence metadata map of keys whose entries were evicted.
    #[cfg(feature = "persist")]
    fn contains_key(&self, key: &crate::core::QueryKey) -> bool;
}

/// Type-erased infinite query bucket trait.
pub(crate) trait ErasedInfiniteBucket {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &gpui::App);
    fn count(&self) -> usize;
    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut gpui::App);
    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut gpui::App);
    fn remove_matching(&mut self, filter: &QueryKeyFilter);
    fn cancel_matching(&mut self, filter: &QueryKeyFilter, cx: &mut gpui::App);
    /// Push each live entry's diagnostic into `out`. See
    /// [`ErasedBucket::collect_diagnostics_into`].
    fn collect_diagnostics_into(&self, now_ms: u64, cx: &gpui::App, out: &mut Vec<QueryDiagnostic>);
    /// Key/status pairs without full diagnostics; used by `dehydrate`. See
    /// [`ErasedBucket::collect_key_status_into`].
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &gpui::App, out: &mut Vec<(String, QueryStatus)>);
    /// Value-carrying persistence variant. See
    /// [`ErasedBucket::collect_persistable_into`].
    #[cfg(feature = "persist")]
    fn collect_persistable_into(
        &self,
        cx: &gpui::App,
        serializers: &SerializerRegistry,
        now_ms: u64,
        out: &mut Vec<(crate::core::QueryKey, PersistedEntry)>,
    );
    /// Whether the bucket currently holds `key`. See
    /// [`ErasedBucket::contains_key`].
    #[cfg(feature = "persist")]
    fn contains_key(&self, key: &crate::core::QueryKey) -> bool;
}

/// Type-erased mutation bucket trait.
pub(crate) trait ErasedMutationBucket {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &gpui::App);
    fn count(&self) -> usize;
    /// Push each live entry's `MutationDiagnostic` into `out`. See
    /// [`ErasedBucket::collect_diagnostics_into`] for the rationale.
    fn collect_diagnostics_into(&self, cx: &gpui::App, out: &mut Vec<MutationDiagnostic>);
    /// Key/status pairs without full diagnostics (`key` is `None` for keyless
    /// mutations); used by `dehydrate`.
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &gpui::App, out: &mut Vec<(Option<String>, MutationStatus)>);
}

/// Legacy synchronous persistence adapter trait for the metadata-only
/// `dehydrate`/`hydrate`/`persist`/`restore` methods. The richer async
/// value-carrying surface is [`Persister`](crate::client::Persister) plus
/// [`persist_with`](crate::client::QueryClient::persist_with).
///
/// Entries are serialized as JSON strings to avoid generic bounds on the
/// persister; implementations can target any backend.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use gpui_query::client::{QueryPersister, DehydratedEntry};
///
/// struct FilePersister { path: PathBuf }
///
/// impl QueryPersister for FilePersister {
///     fn load(&self) -> Vec<DehydratedEntry> { Vec::new() }
///     fn save(&self, _entries: Vec<DehydratedEntry>) {}
/// }
/// ```
#[cfg(feature = "persist")]
pub trait QueryPersister: Send + Sync {
    /// Load persisted entries from storage.
    fn load(&self) -> Vec<crate::client::devtools::DehydratedEntry>;

    /// Save entries to storage, replacing any previously stored data.
    fn save(&self, entries: Vec<crate::client::devtools::DehydratedEntry>);
}
