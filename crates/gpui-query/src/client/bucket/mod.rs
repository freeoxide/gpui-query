//! Type-partitioned buckets for query resources.
//!
//! `ResourceBucket` in `shared` holds the machinery shared by
//! [`QueryBucket`] and [`InfiniteQueryBucket`](crate::client::InfiniteQueryBucket):
//! weak-entity entries with co-located request sequencers, capacity-bounded
//! eviction, GC, bulk key-filter operations, and diagnostics.

mod erased_ops;
mod ops;
pub(crate) mod shared;
pub(crate) mod types;

pub use ops::QueryBucket;
