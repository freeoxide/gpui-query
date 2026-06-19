//! Shared constants and helpers for bucket implementations.
//!
//! This module hosts logic that is structurally duplicated across
//! `QueryBucket`, `InfiniteQueryBucket`, and `MutationBucket` (finding #10).
//! The collapse is intentionally conservative: only clearly-safe shared
//! constants and pure helpers live here. A full generic `Bucket<K, R>`
//! collapse was deemed too risky for this pass.

/// Opportunistic GC interval — run GC every `GC_INTERVAL` insertions.
///
/// This makes GC actually run in production without requiring hooks to call
/// `gc()` explicitly (CL1/#105). The interval is chosen to amortize GC cost:
/// at 64 insertions between sweeps, the per-insert overhead is one `len()`
/// check and one modulo.
///
/// (Audit cleanup: `GC_MIN_ENTRIES` and `should_run_opportunistic_gc` were dead
/// code — `QueryClient::maybe_opportunistic_gc` drives GC directly with
/// `GC_INTERVAL` and `MIN_GC_TIME_MS` and never called the shared helper. Both
/// were removed; `GC_INTERVAL` is retained because `maybe_opportunistic_gc`
/// reads it.)
pub(crate) const GC_INTERVAL: usize = 64;
