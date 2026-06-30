//! Fetcher result wrapper for server-derived cache policy ("server wins").
//!
//! A fetcher normally returns `Result<T, E>`. When it instead returns
//! `Result<Fetched<T>, E>` (via the `*_with_policy` hooks in the `hook` layer),
//! it can attach a server-derived [`CachePolicy`] that overrides the caller's
//! per-query policy on success — so a fetcher that just read
//! `Cache-Control: max-age=30` can push that TTL back into the resource.
//!
//! This type carries the data and an optional [`CachePolicy`]. Behind the
//! `persist` feature it also carries an optional `meta` ([`serde_json::Value`])
//! for HTTP `CacheMeta` round-trip through persistence; the ungated `core`
//! layer (built without `persist`) stays free of `serde_json`.

use crate::core::policy::CachePolicy;
#[cfg(feature = "persist")]
use serde_json::Value as JsonValue;

/// A fetcher success carrying an optional server-derived cache policy.
///
/// Return this from a `*_with_policy` fetcher to let the server override the
/// caller's [`CachePolicy`] for the resolved resource ("server wins"):
///
/// - [`Fetched::new`] — no policy override; the resource keeps the caller's policy.
/// - [`Fetched::with_policy`] — override the resource's policy with the server's.
/// - [`Fetched::with_meta`] (`persist` feature) — attach opaque metadata that
///   flows into [`PersistedEntry::meta`](crate::client::persist::PersistedEntry)
///   so it can be rehydrated on a cold start (e.g. an HTTP `CacheMeta` for
///   cheap `304` refetches after relaunch).
///
/// `cache_policy: None` (the default) keeps the caller's per-query policy
/// unchanged, matching the plain `Result<T, E>` fetcher behavior exactly.
#[derive(Debug, Clone)]
pub struct Fetched<T> {
    /// The fetched data.
    pub data: T,
    /// Server-derived cache policy. `None` keeps the caller's policy.
    pub cache_policy: Option<CachePolicy>,
    /// Opaque metadata carried through to persistence (e.g. HTTP `CacheMeta`).
    /// `None` unless set via [`Fetched::with_meta`]. Only present under the
    /// `persist` feature so the ungated `core` layer stays `serde_json`-free.
    #[cfg(feature = "persist")]
    pub meta: Option<JsonValue>,
}

impl<T> Fetched<T> {
    /// Wrap fetched data with **no** policy override (keep the caller's policy).
    ///
    /// Equivalent to returning the bare `T` from a plain `Result<T, E>` fetcher.
    pub fn new(data: T) -> Self {
        Self {
            data,
            cache_policy: None,
            #[cfg(feature = "persist")]
            meta: None,
        }
    }

    /// Wrap fetched data and override the resource's cache policy with the
    /// server's.
    pub fn with_policy(data: T, policy: CachePolicy) -> Self {
        Self {
            data,
            cache_policy: Some(policy),
            #[cfg(feature = "persist")]
            meta: None,
        }
    }

    /// Attach opaque metadata (e.g. a serialized HTTP `CacheMeta`) to this
    /// fetched value. Requires the `persist` feature; the metadata flows into
    /// [`PersistedEntry::meta`](crate::client::persist::PersistedEntry) when the
    /// resource is persisted, enabling cold-start revalidation.
    #[cfg(feature = "persist")]
    pub fn with_meta(mut self, meta: JsonValue) -> Self {
        self.meta = Some(meta);
        self
    }
}
