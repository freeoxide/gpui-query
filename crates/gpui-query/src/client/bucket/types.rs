//! Core types and constants for the query bucket.

use gpui::WeakEntity;

use crate::core::{QueryResource, RequestSequencer};

/// Minimum GC time in milliseconds.
///
/// A `gc_time_ms` of 0 would cause every `Idle` and `Failure` resource to be
/// evicted on every GC pass (since `age_ms >= 0` is always true for unsigned
/// values). This effectively disables caching for non-active resources.
/// Enforcing a 1-second minimum prevents this footgun.
pub(crate) const MIN_GC_TIME_MS: u64 = 1_000;

/// Default maximum number of entries per bucket.
///
/// Prevents unbounded memory growth from malicious or buggy components that
/// register unlimited unique query keys. When the limit is reached, the
/// oldest (least-recently-updated) entry is evicted to make room.
pub(crate) const DEFAULT_MAX_ENTRIES: usize = 10_000;

/// Multiplier applied to `gc_time_ms` to determine the maximum age for
/// `Success` resources before they become eligible for eviction.
///
/// A `Success` resource whose data age exceeds
/// `SUCCESS_GC_MULTIPLIER * gc_time_ms` will be evicted even though it
/// holds valuable data. This prevents memory leaks from queries that
/// succeeded once but were never observed again.
pub(crate) const SUCCESS_GC_MULTIPLIER: u32 = 2;

/// Entry co-locating weak entity reference and sequencer.
///
/// Uses `WeakEntity` instead of `Entity` so that the bucket does not prevent
/// GPUI from garbage-collecting unused query resources. The weak reference is
/// upgraded on access; if the entity was already collected, the entry is
/// treated as missing and re-created on next `get_or_create`.
///
/// GC reads entity state directly via `entity.read(cx)` (CL2/#106 fix) rather
/// than trusting a cached snapshot, so no status snapshot is stored here.
///
/// # M2 eviction mirror
///
/// `last_updated_ms` and `loading` are a *mirror* of the entity's live
/// `last_updated_at_ms()` / `is_loading()` state, refreshed wherever the
/// bucket already reads the entity (zero extra reads). `evict_oldest` scans
/// these cheap fields + `WeakEntity::upgrade` liveness — with **one**
/// `entity.read` on the winning entry to confirm `!is_loading()` (guards audit
/// #109 against a stale mirror where a fetch began after the last refresh) and
/// read the authoritative timestamp — turning the previous O(n) entity reads
/// into O(1) typical.
pub(crate) struct BucketEntry<T, E> {
    pub entity: WeakEntity<QueryResource<T, E>>,
    pub sequencer: RequestSequencer,
    /// Mirror of `QueryResource::last_updated_at_ms()`. `None` until first
    /// refresh after a terminal completion (or for a freshly-created Idle
    /// resource that has never completed).
    pub(crate) last_updated_ms: Option<u64>,
    /// Mirror of `QueryResource::is_loading()`. `false` until a fetch is
    /// observed to start; refreshed on every bucket read of the entity.
    pub(crate) loading: bool,
}
