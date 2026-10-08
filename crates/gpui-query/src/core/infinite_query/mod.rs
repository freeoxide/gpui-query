//! Infinite query resource for paginated data. Pages live in a
//! `VecDeque<Arc<T>>` (O(1) append/prepend); `max_pages` (default 50) evicts
//! from the opposite side, and `set_max_pages(Some(0))` means unbounded.

mod accessors;
mod lifecycle;
mod page_management;
mod resource;

pub use resource::{FetchDirection, InfiniteQueryResource};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CachePolicy, QueryKey, RequestPolicy};

    fn make_resource() -> InfiniteQueryResource<Vec<String>> {
        InfiniteQueryResource::new(
            QueryKey::from("items"),
            CachePolicy::Ttl { ttl_ms: 60_000 },
            RequestPolicy::LatestWins,
        )
    }

    #[test]
    fn retry_count_accessors() {
        let mut r = make_resource();
        assert_eq!(r.retry_count(), 0);
        r.increment_retry();
        r.increment_retry();
        assert_eq!(r.retry_count(), 2);
        r.reset_retry_count();
        assert_eq!(r.retry_count(), 0);
    }
}
