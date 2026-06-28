//! Fetcher result wrapper for server-derived cache policy ("server wins").
//!
//! A fetcher normally returns `Result<T, E>`. When it instead returns
//! `Result<Fetched<T>, E>` (via the `*_with_policy` hooks in the `hook` layer),
//! it can attach a server-derived [`CachePolicy`] that overrides the caller's
//! per-query policy on success — so a fetcher that just read
//! `Cache-Control: max-age=30` can push that TTL back into the resource.
//!
//! This type is intentionally minimal: it carries only the data and an optional
//! [`CachePolicy`]. It does **not** depend on `serde_json` or any persistence
//! surface. A `meta` field (for HTTP `CacheMeta` round-trip through persistence)
//! is added behind the `persist` feature in a later phase, so the ungated
//! `core` layer stays free of `serde_json`.

use crate::core::policy::CachePolicy;

/// A fetcher success carrying an optional server-derived cache policy.
///
/// Return this from a `*_with_policy` fetcher to let the server override the
/// caller's [`CachePolicy`] for the resolved resource ("server wins"):
///
/// - [`Fetched::new`] — no policy override; the resource keeps the caller's policy.
/// - [`Fetched::with_policy`] — override the resource's policy with the server's.
///
/// `cache_policy: None` (the default) keeps the caller's per-query policy
/// unchanged, matching the plain `Result<T, E>` fetcher behavior exactly.
#[derive(Debug, Clone)]
pub struct Fetched<T> {
    /// The fetched data.
    pub data: T,
    /// Server-derived cache policy. `None` keeps the caller's policy.
    pub cache_policy: Option<CachePolicy>,
}

impl<T> Fetched<T> {
    /// Wrap fetched data with **no** policy override (keep the caller's policy).
    ///
    /// Equivalent to returning the bare `T` from a plain `Result<T, E>` fetcher.
    pub fn new(data: T) -> Self {
        Self {
            data,
            cache_policy: None,
        }
    }

    /// Wrap fetched data and override the resource's cache policy with the
    /// server's.
    pub fn with_policy(data: T, policy: CachePolicy) -> Self {
        Self {
            data,
            cache_policy: Some(policy),
        }
    }
}
