//! Core cache layer tests for gpui-query.
//!
//! Covers:
//! - TTL cache policy: freshness, boundary, expiry, renewal
//! - StaleWhileRevalidate: serving stale data, background refetch, full expiry
//! - NoCache: always stale/expired, no short-circuit
//! - Cache invalidation
//! - Cache reset
//! - Data retention: previous_data and rollback
//! - Cache interactions with different request policies

mod cache_policy;
mod cache_ops;
mod data_retention;
mod request_interactions;

use crate::core::*;
use crate::tests::test_support::*;

// ── Named time constants ────────────────────────────────────────────────
//
// All TTL/SWR tests seed data at STORED_AT_MS and reason about boundary
// offsets from there.  Naming these values makes the age arithmetic
// self-documenting rather than forcing the reader to reverse-engineer
// magic numbers.

/// The `stored_at` timestamp used by every seeded cache entry (ms).
pub(crate) const STORED_AT_MS: u64 = 1_000;

/// TTL duration shared by both the default TTL resource and the SWR resource.
pub(crate) const TTL_MS: u64 = 1_000;

/// Stale-window duration used by the SWR resource.
pub(crate) const STALE_MS: u64 = 2_000;

/// Total validity window for the SWR resource (TTL + stale).
pub(crate) const SWR_TOTAL_MS: u64 = TTL_MS + STALE_MS; // 3_000

// Derived boundary offsets from STORED_AT_MS:
pub(crate) const AT_TTL_BOUNDARY: u64 = STORED_AT_MS + TTL_MS as u64; // 2_000 — exactly at TTL edge
pub(crate) const ONE_MS_PAST_TTL: u64 = AT_TTL_BOUNDARY + 1; // 2_001 — just past TTL
pub(crate) const AT_SWR_BOUNDARY: u64 = STORED_AT_MS + SWR_TOTAL_MS as u64; // 4_000 — exactly at total edge
pub(crate) const ONE_MS_PAST_SWR: u64 = AT_SWR_BOUNDARY + 1; // 4_001 — fully expired

// ── Helpers ──────────────────────────────────────────────────────────────

pub(crate) fn ttl_resource() -> QueryResource<&'static str> {
    test_resource()
}

pub(crate) fn swr_resource() -> QueryResource<&'static str> {
    QueryResource::new(
        "swr-test",
        CachePolicy::StaleWhileRevalidate {
            ttl_ms: TTL_MS,
            stale_ms: STALE_MS,
        },
        RequestPolicy::LatestWins,
    )
}

pub(crate) fn nocache_test_resource() -> QueryResource<&'static str> {
    // Audit fix #122: delegate to the shared `nocache_resource` helper in
    // test_support rather than rebuilding the resource inline, so there is a
    // single source of truth for the NoCache + LatestWins test resource.
    nocache_resource("nocache-test")
}

pub(crate) fn seed_data(resource: &mut QueryResource<&'static str>, data: &'static str, stored_at_ms: u64) {
    resource.apply_success(data, stored_at_ms);
}
