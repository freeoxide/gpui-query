//! Cache and request policies for query resources.

use serde::{Deserialize, Serialize};

use super::{QueryStatus, RequestId};

/// How cached data is treated when a query is accessed.
///
/// - [`NoCache`](CachePolicy::NoCache): Always fetch fresh data.
/// - [`Ttl`](CachePolicy::Ttl): Use cached data if fresh (within TTL).
/// - [`StaleWhileRevalidate`](CachePolicy::StaleWhileRevalidate): Return stale
///   data immediately while fetching fresh data in the background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CachePolicy {
    /// Never cache — always fetch fresh data.
    NoCache,
    /// Cache with a time-to-live. Data is considered fresh within the TTL.
    ///
    /// `ttl_ms` should be greater than zero; a value of 0 behaves like
    /// [`NoCache`](Self::NoCache) because data is only "fresh" at the instant
    /// it is stored. Not validated in release builds (a `debug_assert` fires
    /// in debug builds).
    Ttl { ttl_ms: u64 },
    /// Return stale data immediately while revalidating in the background.
    ///
    /// Data within `ttl_ms` is served as a fresh cache hit (no refetch). Data
    /// between `ttl_ms` and `ttl_ms + stale_ms` is served as stale data **and**
    /// a background revalidation is triggered. After `ttl_ms + stale_ms`, data
    /// is expired and a normal fetch is performed (no stale data served).
    ///
    /// Both fields should be greater than zero: `ttl_ms = 0` behaves like
    /// [`NoCache`](Self::NoCache), and `stale_ms = 0` degenerates to pure TTL
    /// behavior (empty stale window). Not validated in release builds
    /// (`debug_assert`s fire in debug builds).
    StaleWhileRevalidate { ttl_ms: u64, stale_ms: u64 },
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self::Ttl { ttl_ms: 60_000 } // 1 minute default
    }
}

impl CachePolicy {
    /// Human-readable label.
    ///
    /// Sub-second values are shown with millisecond precision (e.g. "500ms")
    /// rather than truncating to "0s" via integer division. Allocates a
    /// `String`; `format!("{policy}")` writes the same text without the heap
    /// allocation.
    pub fn label(self) -> String {
        self.to_string()
    }

    /// Whether this policy can short-circuit (return cached data without fetching).
    ///
    /// Returns `true` for `Ttl` and `StaleWhileRevalidate` since both can serve
    /// cached data when it is fresh (within the TTL window). The actual freshness
    /// check is done separately in [`is_fresh`](Self::is_fresh).
    pub fn can_short_circuit(self) -> bool {
        matches!(self, Self::Ttl { .. } | Self::StaleWhileRevalidate { .. })
    }

    /// Whether this policy allows serving stale data while revalidating.
    pub fn can_serve_stale(self) -> bool {
        matches!(self, Self::StaleWhileRevalidate { .. })
    }

    /// The TTL in milliseconds, if applicable.
    pub fn ttl_ms(self) -> Option<u64> {
        match self {
            Self::NoCache => None,
            Self::Ttl { ttl_ms } | Self::StaleWhileRevalidate { ttl_ms, .. } => Some(ttl_ms),
        }
    }

    /// The stale-while-revalidate window in milliseconds beyond TTL.
    ///
    /// Returns `None` for policies that are not `StaleWhileRevalidate`.
    pub fn stale_ms(self) -> Option<u64> {
        match self {
            Self::StaleWhileRevalidate { stale_ms, .. } => Some(stale_ms),
            _ => None,
        }
    }

    /// Total valid window (TTL + stale) in milliseconds.
    ///
    /// This is the maximum age at which data can still be served under this policy.
    /// For `Ttl`, this equals `ttl_ms`. For `StaleWhileRevalidate`, it equals
    /// `ttl_ms + stale_ms`. Returns `None` for `NoCache`.
    ///
    /// On overflow (extremely large `ttl_ms + stale_ms`), saturates to `u64::MAX`,
    /// effectively treating the data as indefinitely valid.
    pub fn total_valid_ms(self) -> Option<u64> {
        match self {
            Self::NoCache => None,
            Self::Ttl { ttl_ms } => {
                debug_assert!(
                    ttl_ms > 0,
                    "CachePolicy::Ttl with ttl_ms=0 behaves like NoCache"
                );
                Some(ttl_ms)
            }
            Self::StaleWhileRevalidate { ttl_ms, stale_ms } => {
                debug_assert!(
                    ttl_ms > 0,
                    "CachePolicy::StaleWhileRevalidate with ttl_ms=0 behaves like NoCache"
                );
                debug_assert!(
                    stale_ms > 0,
                    "CachePolicy::StaleWhileRevalidate with stale_ms=0 degenerates to Ttl-only behavior"
                );
                Some(ttl_ms.saturating_add(stale_ms))
            }
        }
    }

    /// Whether the data is fresh (within the TTL window).
    ///
    /// Returns `false` if the policy has no TTL or the data age exceeds TTL.
    pub fn is_fresh(self, age_ms: u64) -> bool {
        self.ttl_ms().map(|ttl| age_ms <= ttl).unwrap_or(false)
    }

    /// Whether the data is stale but still within the stale-while-revalidate window.
    ///
    /// Data is "stale-but-serveable" when:
    /// - The policy is `StaleWhileRevalidate`
    /// - Data age is past TTL but within `ttl_ms + stale_ms`
    pub fn is_stale_but_serveable(self, age_ms: u64) -> bool {
        match self {
            Self::StaleWhileRevalidate { ttl_ms, stale_ms } => {
                let total = ttl_ms.saturating_add(stale_ms);
                age_ms > ttl_ms && age_ms <= total
            }
            _ => false,
        }
    }

    /// Whether the data is expired (past the total valid window).
    ///
    /// Returns `true` if the data age exceeds the total valid window for this policy.
    pub fn is_expired(self, age_ms: u64) -> bool {
        self.total_valid_ms()
            .map(|total| age_ms > total)
            .unwrap_or(true) // NoCache always considers data expired
    }
}

impl std::fmt::Display for CachePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCache => write!(f, "No cache"),
            Self::Ttl { ttl_ms } => {
                write!(f, "Cache TTL ")?;
                write_duration(f, *ttl_ms)
            }
            Self::StaleWhileRevalidate { ttl_ms, stale_ms } => {
                write!(f, "Stale-while-revalidate TTL ")?;
                write_duration(f, *ttl_ms)?;
                write!(f, " stale ")?;
                write_duration(f, *stale_ms)
            }
        }
    }
}

/// Write a duration: seconds for `>= 1000ms`, milliseconds otherwise, so
/// sub-second values do not collapse to "0s" through integer division.
fn write_duration(f: &mut std::fmt::Formatter<'_>, ms: u64) -> std::fmt::Result {
    if ms >= 1_000 {
        write!(f, "{}s", ms / 1_000)
    } else {
        write!(f, "{ms}ms")
    }
}

/// How concurrent requests are handled.
///
/// - [`LatestWins`](RequestPolicy::LatestWins): New requests cancel in-flight ones.
/// - [`IgnoreWhileLoading`](RequestPolicy::IgnoreWhileLoading): New requests are
///   ignored if one is already in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestPolicy {
    /// New requests replace in-flight ones (default).
    #[default]
    LatestWins,
    /// Ignore new requests while one is already loading.
    IgnoreWhileLoading,
}

impl RequestPolicy {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::LatestWins => "Latest wins",
            Self::IgnoreWhileLoading => "Ignore while loading",
        }
    }
}

/// Whether the fetch is a normal request or forced (ignoring cache).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryFetchMode {
    /// Normal fetch — respects cache policy.
    #[default]
    Normal,
    /// Force fetch — ignores cache freshness.
    Force,
}

/// The result of calling `begin_request` on a query resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum QueryBeginResult {
    /// A new request was started.
    Started {
        request_id: RequestId,
        status: QueryStatus,
        replaced_request_id: Option<RequestId>,
    },
    /// Cache is fresh — no fetch needed.
    CacheHit,
    /// Stale data was served and a background revalidation was started.
    ///
    /// The caller should:
    /// 1. Return the existing stale data to the consumer immediately.
    /// 2. Use the `request_id` to perform a background fetch.
    /// 3. Complete the request normally via `complete_success`/`complete_failure`.
    StaleCacheHit {
        request_id: RequestId,
        status: QueryStatus,
        replaced_request_id: Option<RequestId>,
    },
    /// A request is already loading and the policy is `IgnoreWhileLoading`.
    IgnoredWhileLoading { active_request_id: RequestId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_zero_label_uses_ms() {
        let policy = CachePolicy::Ttl { ttl_ms: 0 };
        assert_eq!(policy.label(), "Cache TTL 0ms");
    }

    #[test]
    fn swr_zero_values_label_uses_ms() {
        let policy = CachePolicy::StaleWhileRevalidate {
            ttl_ms: 0,
            stale_ms: 0,
        };
        assert_eq!(policy.label(), "Stale-while-revalidate TTL 0ms stale 0ms");
    }

    #[test]
    fn total_valid_ms_saturates_on_overflow() {
        let policy = CachePolicy::StaleWhileRevalidate {
            ttl_ms: u64::MAX,
            stale_ms: u64::MAX,
        };
        assert_eq!(policy.total_valid_ms(), Some(u64::MAX));
    }
}
