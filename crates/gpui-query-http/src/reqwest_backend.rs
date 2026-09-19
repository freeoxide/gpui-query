//! The optional `reqwest`-based [`HttpBackend`] implementation, compiled when
//! the `reqwest` cargo feature is enabled:
//!
//! ```toml
//! [dependencies]
//! gpui-query-http = { version = "0.1", features = ["reqwest"] }
//! ```
//!
//! `reqwest` is just one backend; any client that can perform a conditional
//! `GET` can implement [`crate::backend::HttpBackend`] instead.

use std::future::Future;

use crate::backend::{BackendResponse, Conditionals, HttpBackend, MaybeSend};

/// A [`HttpBackend`] backed by a caller-configured [`reqwest::Client`],
/// reused across requests as `reqwest` intends.
pub struct ReqwestBackend(pub reqwest::Client);

impl ReqwestBackend {
    /// Wrap a pre-configured client; `ReqwestBackend(client)` works too.
    pub fn from_client(client: reqwest::Client) -> Self {
        Self(client)
    }
}

impl HttpBackend for ReqwestBackend {
    type Error = reqwest::Error;

    fn fetch(
        &self,
        url: &str,
        conditionals: Conditionals,
    ) -> impl Future<Output = Result<BackendResponse, reqwest::Error>> + MaybeSend {
        // Build eagerly so the returned future stays `Send` on native targets
        // even where RequestBuilder is not.
        let mut req = self.0.get(url);
        if let Some(etag) = conditionals.if_none_match {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(since) = conditionals.if_modified_since {
            req = req.header(reqwest::header::IF_MODIFIED_SINCE, since);
        }
        async move {
            let mut resp = req.send().await?;
            let status = resp.status().as_u16();
            // Take the map: HeaderMap::clone would copy every entry.
            let headers = std::mem::take(resp.headers_mut());
            let body = resp.bytes().await?;
            Ok(BackendResponse {
                status,
                headers,
                body,
            })
        }
    }
}
