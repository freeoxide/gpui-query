//! Neutral time helper shared across the client layer.
//!
//! Previously co-located with the type-erased bucket traits in `erased.rs`, this
//! helper moved to its own module so the `persist` feature gate (which now owns
//! `erased.rs`'s persistence symbols) does not pull `current_time_ms` behind a
//! `cfg`: the GC subsystem and several non-persistence call sites depend on it.

/// Returns the current time as milliseconds since the UNIX epoch.
///
/// Used internally by `gc()` and other time-sensitive operations.
/// Exposed so callers can cache the value and pass it to `gc_with_time()`
/// to avoid repeated syscalls.
///
/// # Clock-before-epoch fallback
///
/// `duration_since(UNIX_EPOCH)` errors if the system clock reports a time
/// *before* the Unix epoch (1970-01-01 UTC) — e.g. a misconfigured RTC or a
/// clock skewed backwards on cold boot. The `.unwrap_or_default()` silently
/// clamps that case to a `Duration::ZERO`, i.e. this function returns `0`.
/// That `0` is treated as "ancient" by GC, so the only observable effect is
/// that entries become immediately eligible for garbage collection for the
/// duration of the clock anomaly; no panic, no error propagation. This is a
/// deliberate silent clamp rather than a propagating error because every
/// caller treats `now_ms` as infallible and time-sensitive operations
/// degrading to "collect now" is the safest default under a broken clock.
pub fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
