//! Library-agnostic HTTP backend abstraction.
//!
//! [`HttpBackend`] abstracts a single conditional `GET` so [`crate::HttpCache`]
//! is not tied to one HTTP client. The crate ships
//! [`crate::reqwest_backend::ReqwestBackend`] behind the `reqwest` feature;
//! implement this trait to plug in any other client.
//!
//! The trait returns `impl Future + MaybeSend` instead of using `async fn` so
//! the futures are `Send` on native targets (usable from any executor) while
//! `wasm32` still works (see [`MaybeSend`]). That makes it non-object-safe;
//! dispatch is static via `HttpCache<B: HttpBackend>`.

use std::future::Future;

use bytes::Bytes;
use http::HeaderMap;

use crate::CacheMeta;

/// Conditional request headers for a revalidation fetch, mirroring the two
/// validators [`crate::CacheMeta`] tracks. Attach whichever are `Some` to the
/// outgoing request; a server that still matches them answers `304 Not
/// Modified`, which [`crate::HttpCache`] turns into a cheap cache hit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conditionals {
    /// The `If-None-Match` header value (sourced from a cached `ETag`).
    pub if_none_match: Option<String>,
    /// The `If-Modified-Since` header value (sourced from a cached
    /// `Last-Modified`).
    pub if_modified_since: Option<String>,
}

impl Conditionals {
    /// Validators from cached `meta`, or [`Conditionals::default`] when
    /// `meta` is `None` (first fetch, nothing cached yet).
    pub fn from_meta(meta: Option<&CacheMeta>) -> Self {
        let Some(meta) = meta else {
            return Self::default();
        };
        Self {
            if_none_match: meta.etag.clone(),
            if_modified_since: meta.last_modified.clone(),
        }
    }
}

/// An owned, library-agnostic HTTP response.
///
/// Backends translate their native response into this shape so
/// [`crate::HttpCache`] can reason about status, headers, and body without
/// depending on any client crate: `http::HeaderMap` headers and owned
/// [`Bytes`] let the response outlive the underlying connection.
#[derive(Clone, Debug)]
pub struct BackendResponse {
    /// The HTTP status code (e.g. `200`, `304`).
    pub status: u16,
    /// The response headers, as an [`http::HeaderMap`].
    pub headers: HeaderMap,
    /// The response body, owned.
    pub body: Bytes,
}

/// Marker alias for [`Send`], relaxed to a no-op on `wasm32`.
///
/// Bounds [`HttpBackend::fetch`]'s future: on native targets the bound is
/// exactly [`Send`] (any executor may move the future across threads), and
/// every `Send` type implements it.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized + Send> MaybeSend for T {}

/// Marker alias for [`Send`], relaxed to a no-op on `wasm32`.
///
/// On `wasm32` every type implements it: execution is single-threaded and
/// JS interop types (including `reqwest`'s browser-fetch futures) are
/// `!Send` by design, so no `Send` requirement is imposed.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSend for T {}

/// A library-agnostic conditional `GET` backend.
///
/// Implement this for your HTTP client (the crate ships
/// [`crate::reqwest_backend::ReqwestBackend`] behind the `reqwest` feature)
/// and hand an instance to [`crate::HttpCache::new`]. Implementations must
/// attach the [`Conditionals`] validator headers when present, perform the
/// `GET`, and translate the native response into [`BackendResponse`].
///
/// The returned future must be [`MaybeSend`] (`Send` everywhere except
/// `wasm32`), which makes the trait non-object-safe; dispatch is static via
/// `HttpCache<B>`.
pub trait HttpBackend: Send + Sync {
    /// The native error type returned by the underlying client.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Perform a conditional `GET` against `url`.
    fn fetch(
        &self,
        url: &str,
        conditionals: Conditionals,
    ) -> impl Future<Output = Result<BackendResponse, Self::Error>> + MaybeSend;
}
