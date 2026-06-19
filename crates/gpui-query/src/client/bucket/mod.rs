//! Type-partitioned bucket for query resources.
//!
//! **v2 improvements**:
//! - Uses `AHashMap` instead of `std::collections::HashMap`
//! - Co-locates `RequestSequencer` with entity in `BucketEntry`
//! - Collect-then-update pattern avoids nested entity borrows
//!
//! **Audit fixes (findings 1-5)**:
//! - Uses `WeakEntity` to avoid preventing GC of unused resources (finding 3)
//! - Enforces minimum GC time of 1000ms to avoid aggressive eviction (finding 1)
//! - Implements actual policy updates and calls them from `get_or_create` (findings 2, 5)
//!
//! **Audit 3 fixes (this pass)**:
//! - GC reads entity state directly via `entity.read(cx)` instead of a cached
//!   `StatusSnapshot` (CL2/#106). The snapshot machinery was never refreshed
//!   from production, so it was stale — GC now reads the source of truth.
//! - `observer_count` removed (#8): it was never incremented from production
//!   hooks (Drop has no `cx` in GPUI), so it was always 0. `WeakEntity::upgrade()`
//!   is the sole liveness probe.
//! - `retain`/`release`/`update_status_snapshot` removed as dead code (#75).
//! - Opportunistic GC trigger added (CL1/#105): see `bucket::shared`.
//! - `MIN_GC_TIME_MS` exported once from `types` and reused everywhere (#69).
//! - Shared constants/helpers extracted to `bucket::shared` (#10, conservative).

mod erased_ops;
mod ops;
pub(crate) mod shared;
pub(crate) mod types;

pub use ops::QueryBucket;
