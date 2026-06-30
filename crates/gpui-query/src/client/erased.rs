//! Type-erased bucket traits and persistence adapter.
//!
//! These traits allow `QueryClient` to store heterogeneous query and mutation
//! buckets in a single `AHashMap<TypeId, Box<dyn Erased*>>` map, dispatching
//! to concrete types only when the caller provides generic parameters.
//!
//! # Feature gating
//!
//! The persistence-only surface (`collect_key_status_into`, the
//! value-carrying `collect_persistable_into`, and the legacy synchronous
//! [`QueryPersister`] trait) is gated behind the `persist` feature. The
//! non-persistence methods (`gc`, `invalidate_matching`, `diagnostics`, …)
//! remain ungated so the default build is unchanged.

use crate::client::devtools::{MutationDiagnostic, QueryDiagnostic};
#[cfg(feature = "persist")]
use crate::client::persist::{PersistedEntry, SerializerRegistry};
use crate::core::QueryKeyFilter;
#[cfg(feature = "persist")]
use crate::core::{MutationStatus, QueryStatus};

// `current_time_ms` moved to `client/time.rs` (ungated) so the `persist`
// feature gate on this module's persistence symbols does not drag the GC
// clock helper behind a `cfg`. See [`crate::client::time::current_time_ms`].

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
    /// Push each live entry's diagnostic into the caller-supplied `out` Vec
    /// instead of allocating a fresh `Vec` per bucket. Callers
    /// (`QueryClient::diagnostics`) can pre-size a single destination Vec and
    /// let every bucket push into it, avoiding the per-bucket allocation +
    /// `extend` a returning variant would force.
    fn collect_diagnostics_into(&self, now_ms: u64, cx: &gpui::App, out: &mut Vec<QueryDiagnostic>);
    /// Push each live entry's `(key, status)` pair into `out` without building
    /// full `QueryDiagnostic`s (#9). Used by `dehydrate`, which only needs the
    /// key and status, avoiding the per-entry allocations of
    /// [`collect_diagnostics_into`](ErasedBucket::collect_diagnostics_into).
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &gpui::App, out: &mut Vec<(String, QueryStatus)>);
    /// Push each `Success` entry's `(key, entry)` pair into `out`, serializing
    /// the typed data via the caller-supplied [`SerializerRegistry`]. Entries
    /// whose `T` has no registered serializer are skipped (metadata-only
    /// fallback, matching the legacy `dehydrate` behavior). Used by
    /// [`persist_with`](crate::client::QueryClient::persist_with) to build a
    /// value-carrying [`PersistSnapshot`](crate::client::PersistSnapshot).
    #[cfg(feature = "persist")]
    fn collect_persistable_into(
        &self,
        cx: &gpui::App,
        serializers: &SerializerRegistry,
        now_ms: u64,
        out: &mut Vec<(crate::core::QueryKey, PersistedEntry)>,
    );
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
    /// Push each live entry's `(key, status)` pair into `out` without building
    /// full `QueryDiagnostic`s (#9). See
    /// [`ErasedBucket::collect_key_status_into`].
    #[cfg(feature = "persist")]
    fn collect_key_status_into(&self, cx: &gpui::App, out: &mut Vec<(String, QueryStatus)>);
    /// Value-carrying variant for persistence. See
    /// [`ErasedBucket::collect_persistable_into`].
    #[cfg(feature = "persist")]
    fn collect_persistable_into(
        &self,
        cx: &gpui::App,
        serializers: &SerializerRegistry,
        now_ms: u64,
        out: &mut Vec<(crate::core::QueryKey, PersistedEntry)>,
    );
}

/// Type-erased mutation bucket trait.
pub(crate) trait ErasedMutationBucket {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn gc(&mut self, now_ms: u64, gc_time_ms: u64, cx: &gpui::App);
    fn count(&self) -> usize;
    /// Push each live entry's `MutationDiagnostic` into `out` instead of
    /// allocating a fresh `Vec` per bucket. See
    /// [`ErasedBucket::collect_diagnostics_into`] for the rationale.
    fn collect_diagnostics_into(&self, cx: &gpui::App, out: &mut Vec<MutationDiagnostic>);
    /// Push each live entry's `(key, status)` pair into `out` without building
    /// full `MutationDiagnostic`s (#9). `key` is `None` for keyless mutations,
    /// mirroring [`MutationDiagnostic::key`]. Used by `dehydrate`.
    #[cfg(feature = "persist")]
    fn collect_key_status_into(
        &self,
        cx: &gpui::App,
        out: &mut Vec<(Option<String>, MutationStatus)>,
    );
}

/// Legacy synchronous persistence adapter trait for query cache persistence
/// across app restarts.
///
/// **Note**: this is the shipped metadata-only skeleton. The richer async,
/// value-carrying surface lives in [`crate::client::persist`] (the
/// [`Persister`](crate::client::persist::Persister) trait +
/// [`persist_with`](crate::client::QueryClient::persist_with)). This trait is
/// retained for the existing `dehydrate`/`hydrate`/`persist`/`restore` methods
/// and is feature-gated behind `persist`.
///
/// Implementations can store cached data in any backend (filesystem, database, etc.).
/// Entries are serialized as JSON strings to avoid generic bounds on the persister.
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
