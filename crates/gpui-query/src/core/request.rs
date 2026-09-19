//! Request lifecycle primitives for the query system.
//!
//! - [`RequestId`] — a unique, ordered identifier for each in-flight request.
//! - [`RequestSequencer`] — a monotonic generator of `RequestId` values, scoped
//!   per resource to guarantee uniqueness even after sequence overflow.
//! - [`RequestGuard`] — a single-use capability token that enforces the two-phase
//!   completion protocol (accept → complete).
//! - [`QueryTimestamp`] — a millisecond-precision timestamp used for cache
//!   freshness and staleness calculations.
//!
//! The two-phase protocol: accept a [`RequestId`] via
//! [`QueryResource::accept_current_request`](super::QueryResource::accept_current_request)
//! to get a [`RequestGuard`], then pass the guard (by value) to a `complete_*`
//! method. Convenience methods like `complete_current_success` combine both
//! phases into one call.
//!
//! [`QueryResource`]: super::QueryResource

use serde::{Deserialize, Serialize};
use std::num::NonZero;

/// A unique identifier for an in-flight request.
///
/// Combines a scope id (per-resource) with a monotonically increasing sequence.
/// Two `RequestId` values are equal only when both scope and sequence match.
/// Ordering is lexicographic: scope first, then sequence.
///
/// # Example
///
/// ```
/// use gpui_query::core::RequestId;
/// use std::num::NonZero;
///
/// let id = RequestId::scoped(NonZero::new(1).unwrap(), 42);
/// assert_eq!(id.scope_id(), NonZero::new(1).unwrap());
/// assert_eq!(id.value(), 42);
/// assert_eq!(id.label(), "1:42");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[must_use]
pub struct RequestId {
    scope_id: NonZero<u64>,
    sequence: u64,
}

impl RequestId {
    /// Create a request id with explicit scope and sequence.
    ///
    /// The scope must be non-zero; passing a zero scope would violate the
    /// `NonZero<u64>` niche invariant, so it is taken as `NonZero<u64>` directly.
    pub fn scoped(scope_id: NonZero<u64>, sequence: u64) -> Self {
        Self { scope_id, sequence }
    }

    /// The sequence number within this scope.
    pub fn value(self) -> u64 {
        self.sequence
    }

    /// The scope identifier.
    ///
    /// Returns the scope as `NonZero<u64>`. Use `.get()` if a plain `u64` is needed.
    pub fn scope_id(self) -> NonZero<u64> {
        self.scope_id
    }

    /// Human-readable label for diagnostics.
    ///
    /// Allocates a `String`; `format!("{id}")` writes the same text without
    /// the heap allocation.
    pub fn label(self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.scope_id, self.sequence)
    }
}

/// Monotonic request id generator scoped to a single resource.
///
/// Each `RequestSequencer` produces a stream of [`RequestId`] values that are
/// unique within the resource's lifetime. The sequence counter increments
/// from 1; when it would overflow `u64::MAX`, the scope advances via
/// [`advance_scope`](Self::advance_scope), which increments `scope_id` and
/// resets the sequence to 1.
///
/// If `scope_id` itself overflows, it wraps to 1 and the sequence resets, so
/// a fresh `RequestId(1, 1)` could theoretically collide with a very old one
/// still held by a long-running future. Reaching `u64::MAX` requests per scope
/// is out of reach in practice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSequencer {
    pub(crate) scope_id: NonZero<u64>,
    pub(crate) next_request_id: u64,
}

impl Default for RequestSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestSequencer {
    /// Create a new sequencer starting at scope 1, sequence 1.
    pub fn new() -> Self {
        Self {
            scope_id: NonZero::new(1).unwrap(),
            next_request_id: 1,
        }
    }

    /// Generate the next request id.
    ///
    /// When the sequence counter reaches `u64::MAX`, the scope advances
    /// before the next call can produce a duplicate.
    pub fn next_request(&mut self) -> RequestId {
        let request_id = RequestId::scoped(self.scope_id, self.next_request_id);
        if self.next_request_id == u64::MAX {
            self.advance_scope();
        } else {
            self.next_request_id += 1;
        }
        request_id
    }

    /// Advance to a new scope when the sequence overflows.
    pub fn advance_scope(&mut self) {
        self.scope_id = NonZero::new(self.scope_id.get().checked_add(1).unwrap_or(1))
            .unwrap_or(NonZero::<u64>::MIN);
        self.next_request_id = 1;
    }

    /// Whether the given request id belongs to the current scope.
    pub fn is_current_scope(&self, request_id: RequestId) -> bool {
        request_id.scope_id == self.scope_id
    }
}

/// A timestamp for query operations, in milliseconds since UNIX epoch.
///
/// Used for cache freshness checks (TTL, stale-while-revalidate) and for
/// recording when data was last updated. Obtain the current time via
/// `QueryTimestamp::from_millis(...)` using your application's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QueryTimestamp(u64);

impl QueryTimestamp {
    /// Create a timestamp from milliseconds.
    pub fn from_millis(value: u64) -> Self {
        Self(value)
    }

    /// The timestamp in milliseconds.
    pub fn as_millis(self) -> u64 {
        self.0
    }

    /// Compute elapsed time since an earlier timestamp.
    pub(super) fn elapsed_since(self, earlier: Self) -> Option<u64> {
        self.0.checked_sub(earlier.0)
    }
}

impl From<u64> for QueryTimestamp {
    fn from(value: u64) -> Self {
        Self::from_millis(value)
    }
}

/// A single-use capability token proving the holder owns the current request.
///
/// Created by [`QueryResource::accept_current_request`], consumed by one of the
/// `complete_*` methods. The guard is **moved** (not copied) into the
/// completion method, which enforces the two-phase protocol at the type level:
/// once a guard is used, it cannot be used again.
///
/// [`QueryResource`]: super::QueryResource
/// [`QueryResource::accept_current_request`]: super::QueryResource::accept_current_request
#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub struct RequestGuard {
    request_id: RequestId,
}

impl RequestGuard {
    pub(super) fn new(request_id: RequestId) -> Self {
        Self { request_id }
    }

    /// The request id this guard protects (borrowed).
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Consume the guard and return the request id.
    ///
    /// Useful when you want to extract the id and discard the guard.
    pub fn into_request_id(self) -> RequestId {
        self.request_id
    }
}
