//! gpui-query-legacy (DEPRECATED)
//!
//! This is the legacy v1 crate. It is no longer maintained.
//! Migrate to gpui-query (v2) for:
//! - Options-first API with sensible defaults
//! - Tuple return types for explicit control
//! - Signal-always fetchers with cooperative cancellation
//! - Fixed retry loops and signal lifecycle
//! - Efficient re-renders with status deduplication
//!
//! See https://gpui-query.hmziq.xyz/docs/ for migration guidance.
#![deprecated(
    since = "0.1.0",
    note = "gpui-query-legacy is deprecated. Use gpui-query (v2) instead. See https://gpui-query.hmziq.xyz/docs/ for migration guidance."
)]

#[cfg(feature = "core")]
pub mod core;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "hook")]
pub mod hook;

// Convenience re-exports from core (always available when core is enabled)
#[cfg(feature = "core")]
pub use core::*;

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "tests/core_cache.rs"]
mod core_cache;

#[cfg(test)]
#[path = "tests/core_lifecycle.rs"]
mod core_lifecycle;

#[cfg(test)]
#[path = "tests/core_request.rs"]
mod core_request;

#[cfg(test)]
#[path = "tests/core_data_retention.rs"]
mod core_data_retention;

#[cfg(test)]
#[path = "tests/core_retry.rs"]
mod core_retry;

#[cfg(test)]
#[path = "tests/core_mutation.rs"]
mod core_mutation;

#[cfg(test)]
#[path = "tests/core_infinite_query.rs"]
mod core_infinite_query;

#[cfg(test)]
#[path = "tests/core_select.rs"]
mod core_select;

#[cfg(test)]
#[path = "tests/core_network_mode.rs"]
mod core_network_mode;

// Integration tests (require GPUI test-support, available via dev-dep)
#[cfg(test)]
#[path = "tests/integration_client_fixtures.rs"]
mod integration_client_fixtures;

#[cfg(test)]
#[path = "tests/integration_client_bucket.rs"]
mod integration_client_bucket;

#[cfg(test)]
#[path = "tests/integration_client_client/mod.rs"]
mod integration_client_client;

#[cfg(test)]
#[path = "tests/integration_client_advanced/mod.rs"]
mod integration_client_advanced;
