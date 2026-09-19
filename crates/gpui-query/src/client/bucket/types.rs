//! Core types and constants for the resource buckets.

use gpui::WeakEntity;

use crate::core::RequestSequencer;

/// Floor for `gc_time_ms`. A value of 0 (or anything below this) would evict
/// every `Idle`/`Failure` resource on every GC pass, since `age >= 0` always
/// holds for unsigned ages.
pub(crate) const MIN_GC_TIME_MS: u64 = 1_000;

/// Entries per bucket before the oldest one is evicted. Bounds memory when a
/// component registers unbounded unique keys.
pub(crate) const DEFAULT_MAX_ENTRIES: usize = 10_000;

/// `Success` resources survive this many times `gc_time_ms` before GC may
/// evict them, so valuable data outlives transient failures.
pub(crate) const SUCCESS_GC_MULTIPLIER: u32 = 2;

/// Weak entity handle co-located with the key's request sequencer.
///
/// `last_updated_ms` / `loading` mirror the entity's
/// `last_updated_at_ms()` / `is_loading()` and are refreshed whenever the
/// bucket already reads the entity, so `evict_oldest` can scan cheap fields
/// and do a single confirming entity read on its winner.
pub(crate) struct BucketEntry<R> {
    pub entity: WeakEntity<R>,
    pub sequencer: RequestSequencer,
    pub(crate) last_updated_ms: Option<u64>,
    pub(crate) loading: bool,
}
