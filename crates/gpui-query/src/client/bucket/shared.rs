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
pub(crate) const GC_INTERVAL: usize = 64;

/// Minimum number of entries before opportunistic GC activates.
///
/// Below this threshold the bucket is small enough that GC is wasted work —
/// `evict_oldest` already caps growth via `max_entries`.
pub(crate) const GC_MIN_ENTRIES: usize = GC_INTERVAL;

/// Returns `true` if an opportunistic GC sweep should run now.
///
/// Combines the insertion-count cadence with a time-based debounce so that a
/// burst of insertions does not trigger multiple sweeps within
/// `MIN_GC_TIME_MS`. `gc_time_ms == 0` disables GC entirely.
pub(crate) fn should_run_opportunistic_gc(
    count: usize,
    gc_time_ms: u64,
    last_gc_ms: u128,
    now_ms: u128,
) -> bool {
    if gc_time_ms == 0 {
        return false;
    }
    if count < GC_MIN_ENTRIES || count % GC_INTERVAL != 0 {
        return false;
    }
    now_ms.saturating_sub(last_gc_ms) >= super::types::MIN_GC_TIME_MS as u128
}
