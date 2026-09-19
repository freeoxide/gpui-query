//! The URL-keyed HTTP cache, [`HttpCache`].
//!
//! [`HttpCache`] wraps any [`crate::backend::HttpBackend`] with an in-memory
//! cache keyed by URL string. Fresh entries skip the network; stale entries
//! revalidate with `If-None-Match` / `If-Modified-Since`, and a `304`
//! re-serves the cached body.
//!
//! Concurrency: two [`std::sync::Mutex`]es (meta, bodies), always locked in
//! that order, never held across an `.await`. The cache is `Send + Sync` and
//! runtime-agnostic.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use gpui_query::core::CachePolicy;
use http::HeaderMap;
use thiserror::Error;

use crate::backend::{BackendResponse, Conditionals, HttpBackend};
use crate::{CacheMeta, ParseError, cache_policy_from_headers};

/// Errors raised by [`HttpCache::fetch`].
#[derive(Debug, Error)]
pub enum HttpError {
    /// The underlying backend failed to perform the request; the source error
    /// is preserved for downcasting or cause-chain walks.
    #[error("backend request failed")]
    Backend {
        /// The source error from the backend.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    /// Response cache headers could not be parsed into a [`CachePolicy`].
    /// [`HttpCache::fetch`] itself degrades unparseable headers to
    /// [`CachePolicy::NoCache`] instead of failing, so this surfaces only for
    /// direct users of [`cache_policy_from_headers`].
    #[error(transparent)]
    InvalidPolicy(#[from] ParseError),
    /// The server returned `304 Not Modified` but the cache holds no body for
    /// this URL to fall back on.
    #[error("received 304 without a cached body for {url:?}")]
    NotModifiedWithoutCachedBody {
        /// The URL that produced the spurious `304`.
        url: String,
    },
    /// A cache [`Mutex`] was poisoned; surfaced as a typed error so one
    /// poisoned cache fails a request instead of panicking the caller.
    #[error("cache mutex poisoned")]
    Poisoned,
}

/// A URL-keyed HTTP cache layered over a [`HttpBackend`].
///
/// Generic over the backend so dispatch is static (no `Box<dyn>` overhead).
/// Entries are keyed by the exact URL string: no normalization, and `Vary` is
/// ignored. There is no eviction: the cache grows with every distinct URL,
/// so scope instances accordingly.
pub struct HttpCache<B: HttpBackend> {
    backend: B,
    meta: Mutex<HashMap<String, CacheMeta>>,
    bodies: Mutex<HashMap<String, Bytes>>,
}

impl<B: HttpBackend> HttpCache<B> {
    /// Create a new cache backed by `backend`, starting empty.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            meta: Mutex::new(HashMap::new()),
            bodies: Mutex::new(HashMap::new()),
        }
    }

    /// Fetch `url`: a fresh cached entry skips the network, otherwise the
    /// backend revalidates.
    ///
    /// Returns `(body, policy, meta)`. Only a cacheable `200` populates the
    /// cache and yields `meta`; every other status (and any `no-store`,
    /// absent, or unparseable `Cache-Control`) returns the body with
    /// [`CachePolicy::NoCache`] and `None`. A `304` re-serves the cached body.
    pub async fn fetch(
        &self,
        url: &str,
    ) -> Result<(Bytes, CachePolicy, Option<CacheMeta>), HttpError> {
        let cached_meta = {
            let guard = self.meta.lock().map_err(|_| HttpError::Poisoned)?;
            guard.get(url).cloned()
        };

        // Fresh hit: no backend call at all. checked_add: never panic if a
        // future serde-hydrated CacheMeta carries an extreme stored_at.
        if let Some(meta) = cached_meta.as_ref()
            && meta.stored_at.checked_add(meta.fresh_for).is_none_or(|t| t > SystemTime::now())
            && let Some(body) = self.cached_body(url)?
        {
            return Ok((body, policy_from_meta(meta), cached_meta.clone()));
        }
        // Not fresh, or meta without a body: revalidate.

        let conditionals = Conditionals::from_meta(cached_meta.as_ref());
        let resp = self
            .backend
            .fetch(url, conditionals)
            .await
            .map_err(|e| HttpError::Backend {
                source: Box::new(e),
            })?;

        if resp.status == 304 {
            let Some(body) = self.cached_body(url)? else {
                return Err(HttpError::NotModifiedWithoutCachedBody {
                    url: url.to_string(),
                });
            };
            let policy = cached_meta
                .as_ref()
                .map(policy_from_meta)
                .unwrap_or(CachePolicy::NoCache);
            return Ok((body, policy, cached_meta));
        }

        if resp.status == 200 {
            return self.store_fresh(url, resp);
        }

        // No other status is stored (conservative subset of RFC 9111 §3).
        Ok((resp.body, CachePolicy::NoCache, None))
    }

    fn cached_body(&self, url: &str) -> Result<Option<Bytes>, HttpError> {
        let guard = self.bodies.lock().map_err(|_| HttpError::Poisoned)?;
        Ok(guard.get(url).cloned())
    }

    /// Parse the policy from a `200`, store body + meta, return the triple.
    fn store_fresh(
        &self,
        url: &str,
        resp: BackendResponse,
    ) -> Result<(Bytes, CachePolicy, Option<CacheMeta>), HttpError> {
        let BackendResponse {
            headers, body, ..
        } = resp;
        // A malformed cache hint must never fail the data fetch itself:
        // serve the body uncacheable.
        let Ok(policy) = cache_policy_from_headers(&headers) else {
            return Ok((body, CachePolicy::NoCache, None));
        };

        if policy == CachePolicy::NoCache {
            return Ok((body, CachePolicy::NoCache, None));
        }

        let meta = CacheMeta {
            etag: header_str(&headers, "etag"),
            last_modified: header_str(&headers, "last-modified"),
            stored_at: SystemTime::now(),
            fresh_for: fresh_for_from_policy(policy),
            stale_for: stale_for_from_policy(policy),
        };

        // Lock order is meta -> bodies everywhere.
        {
            let mut guard = self.meta.lock().map_err(|_| HttpError::Poisoned)?;
            guard.insert(url.to_string(), meta.clone());
        }
        {
            let mut guard = self.bodies.lock().map_err(|_| HttpError::Poisoned)?;
            guard.insert(url.to_string(), body.clone());
        }

        Ok((body, policy, Some(meta)))
    }
}

/// Read a single header value as an owned [`String`], or `None` if absent or
/// non-ASCII.
fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// `fresh_for` from a policy's TTL window.
fn fresh_for_from_policy(policy: CachePolicy) -> Duration {
    Duration::from_millis(policy.ttl_ms().unwrap_or(0))
}

/// `stale_for` from a policy's SWR window.
fn stale_for_from_policy(policy: CachePolicy) -> Duration {
    Duration::from_millis(policy.stale_ms().unwrap_or(0))
}

/// Invert the two helpers above: non-zero `stale_for` selects
/// [`CachePolicy::StaleWhileRevalidate`], non-zero `fresh_for` selects
/// [`CachePolicy::Ttl`], both-zero collapses to [`CachePolicy::NoCache`].
fn policy_from_meta(meta: &CacheMeta) -> CachePolicy {
    let ttl_ms = u64::try_from(meta.fresh_for.as_millis()).unwrap_or(0);
    let stale_ms = u64::try_from(meta.stale_for.as_millis()).unwrap_or(0);
    if stale_ms > 0 {
        CachePolicy::StaleWhileRevalidate { ttl_ms, stale_ms }
    } else if ttl_ms > 0 {
        CachePolicy::Ttl { ttl_ms }
    } else {
        CachePolicy::NoCache
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{BackendResponse, Conditionals, HttpBackend, MaybeSend};
    use bytes::Bytes;
    use http::HeaderMap;
    use std::collections::VecDeque;
    use std::future::Future;

    /// Mock backend: pops canned responses from a FIFO queue and counts
    /// calls so tests can assert short-circuit behavior.
    struct MockBackend {
        responses: Mutex<VecDeque<Result<BackendResponse, MockError>>>,
        calls: Mutex<usize>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("mock backend error")]
    struct MockError;

    impl MockBackend {
        fn new(responses: Vec<Result<BackendResponse, MockError>>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                calls: Mutex::new(0),
            }
        }

        fn calls(&self) -> usize {
            *self.calls.lock().unwrap()
        }

        fn remaining(&self) -> usize {
            self.responses.lock().unwrap().len()
        }
    }

    impl HttpBackend for MockBackend {
        type Error = MockError;

        fn fetch(
            &self,
            _url: &str,
            _conditionals: Conditionals,
        ) -> impl Future<Output = Result<BackendResponse, MockError>> + MaybeSend {
            let next = {
                let mut calls = self.calls.lock().unwrap();
                *calls += 1;
                self.responses.lock().unwrap().pop_front()
            };
            // Queue exhausted -> mock error so the test fails loudly.
            async move {
                match next {
                    Some(Ok(r)) => Ok(r),
                    Some(Err(e)) => Err(e),
                    None => Err(MockError),
                }
            }
        }
    }

    fn resp_200(body: &str, cache_control: &str) -> BackendResponse {
        let mut headers = HeaderMap::new();
        headers.insert(http::header::CACHE_CONTROL, cache_control.parse().unwrap());
        BackendResponse {
            status: 200,
            headers,
            body: Bytes::copy_from_slice(body.as_bytes()),
        }
    }

    fn resp_304_with_etag(etag: &str) -> BackendResponse {
        let mut headers = HeaderMap::new();
        headers.insert(http::header::ETAG, etag.parse().unwrap());
        BackendResponse {
            status: 304,
            headers,
            body: Bytes::new(),
        }
    }

    #[tokio::test]
    async fn two_hundred_stores_body_and_meta() {
        let backend = MockBackend::new(vec![Ok(resp_200("hello", "max-age=600"))]);
        let cache = HttpCache::new(backend);

        let (body, policy, meta) = cache.fetch("https://example.test/a").await.unwrap();
        assert_eq!(body, Bytes::from_static(b"hello"));
        assert_eq!(policy, CachePolicy::Ttl { ttl_ms: 600_000 });
        let meta = meta.expect("200 with cacheable policy yields meta");
        assert_eq!(meta.fresh_for, Duration::from_secs(600));
        assert_eq!(meta.stale_for, Duration::ZERO);
    }

    #[tokio::test]
    async fn fresh_entry_short_circuits_no_backend_call() {
        let backend = MockBackend::new(vec![
            Ok(resp_200("first", "max-age=600")),
            Ok(resp_200("should-not-happen", "max-age=1")),
        ]);
        let cache = HttpCache::new(backend);

        let (body1, policy1, _) = cache.fetch("https://example.test/b").await.unwrap();
        assert_eq!(body1, Bytes::from_static(b"first"));
        assert_eq!(policy1, CachePolicy::Ttl { ttl_ms: 600_000 });

        let (body2, policy2, _) = cache.fetch("https://example.test/b").await.unwrap();
        assert_eq!(body2, Bytes::from_static(b"first"), "fresh hit serves cached body");
        assert_eq!(policy2, CachePolicy::Ttl { ttl_ms: 600_000 });

        assert_eq!(cache.backend.calls(), 1);
        assert_eq!(cache.backend.remaining(), 1, "second canned response untouched");
    }

    #[tokio::test]
    async fn not_modified_returns_cached_body() {
        let backend = MockBackend::new(vec![
            Ok(resp_200("payload", "max-age=0, stale-while-revalidate=60")),
            Ok(resp_304_with_etag("\"v1\"")),
        ]);
        let cache = HttpCache::new(backend);

        let (body1, _, _) = cache.fetch("https://example.test/c").await.unwrap();
        assert_eq!(body1, Bytes::from_static(b"payload"));

        // max-age=0 -> not fresh -> conditional refetch -> 304.
        let (body2, _, meta2) = cache.fetch("https://example.test/c").await.unwrap();
        assert_eq!(body2, Bytes::from_static(b"payload"), "304 served cached body");
        assert!(meta2.is_some(), "304 still yields cached meta");
    }

    #[tokio::test]
    async fn no_store_returns_no_cache_and_stores_nothing() {
        let backend = MockBackend::new(vec![Ok(resp_200("ephemeral", "no-store"))]);
        let cache = HttpCache::new(backend);

        let (body, policy, meta) = cache.fetch("https://example.test/d").await.unwrap();
        assert_eq!(body, Bytes::from_static(b"ephemeral"));
        assert_eq!(policy, CachePolicy::NoCache);
        assert!(meta.is_none(), "no-store must not produce meta");
        assert!(
            cache
                .meta
                .lock()
                .unwrap()
                .get("https://example.test/d")
                .is_none()
        );
    }

    #[tokio::test]
    async fn malformed_cache_control_degrades_to_no_cache() {
        // A malformed cache hint must not fail the data fetch.
        let backend = MockBackend::new(vec![Ok(resp_200("body", "max-age=abc"))]);
        let cache = HttpCache::new(backend);

        let (body, policy, meta) = cache.fetch("https://example.test/f").await.unwrap();
        assert_eq!(body, Bytes::from_static(b"body"));
        assert_eq!(policy, CachePolicy::NoCache);
        assert!(meta.is_none());
        assert!(
            cache
                .meta
                .lock()
                .unwrap()
                .get("https://example.test/f")
                .is_none()
        );
    }

    #[tokio::test]
    async fn non_two_hundred_is_not_cached() {
        let backend = MockBackend::new(vec![Ok(BackendResponse {
            status: 404,
            headers: HeaderMap::new(),
            body: Bytes::copy_from_slice(b"missing"),
        })]);
        let cache = HttpCache::new(backend);

        let (body, policy, meta) = cache.fetch("https://example.test/g").await.unwrap();
        assert_eq!(body, Bytes::from_static(b"missing"));
        assert_eq!(policy, CachePolicy::NoCache);
        assert!(meta.is_none());
        assert!(
            cache
                .meta
                .lock()
                .unwrap()
                .get("https://example.test/g")
                .is_none()
        );
    }

    #[tokio::test]
    async fn overflow_max_age_caches_saturated() {
        // RFC 9111 §1.2.2: over-large delta-seconds saturate; the entry is
        // effectively fresh forever and later fetches short-circuit.
        let backend = MockBackend::new(vec![
            Ok(resp_200("big", "max-age=99999999999999999999999")),
            Ok(resp_200("second", "max-age=1")),
        ]);
        let cache = HttpCache::new(backend);

        let (body, policy, _) = cache.fetch("https://example.test/h").await.unwrap();
        assert_eq!(body, Bytes::from_static(b"big"));
        assert_eq!(policy, CachePolicy::Ttl { ttl_ms: u64::MAX });

        let (body2, policy2, meta2) = cache.fetch("https://example.test/h").await.unwrap();
        assert_eq!(body2, Bytes::from_static(b"big"));
        assert_eq!(policy2, CachePolicy::Ttl { ttl_ms: u64::MAX });
        assert!(meta2.is_some());
        assert_eq!(cache.backend.calls(), 1);
    }

    #[tokio::test]
    async fn not_modified_without_cached_body_is_typed_error() {
        // A 304 is only meaningful as a revalidation of a cached entry.
        let backend = MockBackend::new(vec![Ok(resp_304_with_etag("\"v1\""))]);
        let cache = HttpCache::new(backend);

        let err = cache.fetch("https://example.test/e").await.unwrap_err();
        assert!(
            matches!(err, HttpError::NotModifiedWithoutCachedBody { .. }),
            "got {err:?}"
        );
    }
}
