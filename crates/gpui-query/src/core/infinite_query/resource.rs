//! Struct definition, serde helpers, constants, and constructors for
//! [`InfiniteQueryResource`].

use std::collections::VecDeque;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::{
    CachePolicy, QueryError, QueryKey, QuerySignal, QueryStatus, QueryTimestamp, RequestId,
    RequestPolicy, RetryPolicy,
};

/// Default maximum number of pages to retain.
const DEFAULT_MAX_PAGES: usize = 50;

/// Direction mode for an infinite query.
///
/// Controls the default assumptions for `has_next_page` and
/// `has_previous_page` on construction and after `reset()`.
///
/// - **ForwardOnly** (default): `has_next_page` starts `true`, `has_previous_page` starts `false`.
///   This is the common case for feed-style pagination where you only fetch next pages.
///   The `true` default for `has_next_page` assumes more pages exist until the fetcher says
///   otherwise.
///
/// - **Bidirectional**: Both `has_next_page` and `has_previous_page` start `false`.
///   The query will not attempt to fetch in either direction until the caller explicitly
///   sets `has_next_page(true)` or `has_previous_page(true)`, or the fetcher returns
///   `has_more = true` from a successful completion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FetchDirection {
    /// Fetch next pages only. `has_next_page` defaults to `true`.
    #[default]
    ForwardOnly,
    /// Fetch in both directions. Both flags default to `false`.
    Bidirectional,
}

/// An infinite query resource that manages paginated data.
///
/// Inspired by TanStack Query's `useInfiniteQuery`. Each "page" is a `T` —
/// typically a batch of items fetched from an API.
///
/// Pages are stored internally as `Arc<T>` so that [`last_page_arc`](Self::last_page_arc)
/// and [`first_page_arc`](Self::first_page_arc) can hand the fetcher a cheap
/// `Arc::clone` instead of cloning the full page data.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "T: serde::Serialize, E: serde::Serialize"))]
#[serde(bound(deserialize = "T: serde::de::DeserializeOwned, E: serde::de::DeserializeOwned"))]
pub struct InfiniteQueryResource<T, E = QueryError> {
    pub(super) key: QueryKey,
    #[serde(with = "vec_deque_serde")]
    pub(super) pages: VecDeque<Arc<T>>,
    pub(super) status: QueryStatus,
    pub(super) error: Option<E>,
    pub(super) active_request_id: Option<RequestId>,
    pub(super) cache_policy: CachePolicy,
    pub(super) request_policy: RequestPolicy,
    pub(super) started_at: Option<QueryTimestamp>,
    pub(super) last_updated_at: Option<QueryTimestamp>,
    pub(super) cache_hits: u64,
    pub(super) cancelled_count: u64,
    pub(super) ignored_results: u64,
    pub(super) retry_count: u32,
    pub(super) has_next_page: bool,
    pub(super) has_previous_page: bool,
    pub(super) is_fetching_next_page: bool,
    pub(super) is_fetching_previous_page: bool,
    pub(super) max_pages: Option<usize>,
    pub(super) direction: FetchDirection,
    pub(super) retry_policy: RetryPolicy,
    #[serde(skip)]
    pub(super) signal: Option<QuerySignal>,
    #[cfg(feature = "client")]
    #[serde(skip)]
    pub(crate) current_task: crate::core::current_task::CurrentTask,
}

/// Serde helpers for `VecDeque<Arc<T>>` — serializes as a plain sequence and
/// deserializes into `VecDeque<Arc<T>>`. This keeps the wire format identical
/// to the old `Vec<T>` representation so existing cached data remains
/// compatible (`Arc<T>` serializes transparently as `T`).
pub(super) mod vec_deque_serde {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use serde::de::{Deserialize, DeserializeOwned};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S, T>(deque: &VecDeque<Arc<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: serde::Serialize,
    {
        let mut seq = serializer.serialize_seq(Some(deque.len()))?;
        for item in deque {
            // Serialize the inner `T` directly (`&**item`) rather than the
            // `Arc<T>`. This avoids requiring `Arc<T>: Serialize` (which is only
            // available with serde's `rc` feature / certain configs) and keeps
            // the wire format identical to the old `Vec<T>` representation.
            seq.serialize_element(&**item)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<VecDeque<Arc<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: DeserializeOwned,
    {
        let vec: Vec<T> = Vec::<T>::deserialize(deserializer)?;
        Ok(vec.into_iter().map(Arc::new).collect())
    }
}

impl<T, E> InfiniteQueryResource<T, E> {
    /// Create a new infinite query resource.
    ///
    /// **v2**: `max_pages` defaults to `Some(50)` to prevent unbounded memory growth.
    ///
    /// **Audit 3**: Uses `FetchDirection::ForwardOnly` by default, meaning
    /// `has_next_page` starts `true`. Use [`new_bidirectional`](Self::new_bidirectional)
    /// for queries that paginate in both directions.
    pub fn new(
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
    ) -> Self {
        Self::with_direction(key, cache_policy, request_policy, FetchDirection::ForwardOnly)
    }

    /// Create a new infinite query resource configured for bidirectional paging.
    ///
    /// Both `has_next_page` and `has_previous_page` default to `false`. The
    /// query will not attempt to fetch in either direction until the caller
    /// explicitly enables it.
    pub fn new_bidirectional(
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
    ) -> Self {
        Self::with_direction(key, cache_policy, request_policy, FetchDirection::Bidirectional)
    }

    /// Create a new infinite query resource with an explicit [`FetchDirection`].
    pub(crate) fn with_direction(
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        direction: FetchDirection,
    ) -> Self {
        let (has_next, has_prev) = match direction {
            FetchDirection::ForwardOnly => (true, false),
            FetchDirection::Bidirectional => (false, false),
        };
        Self {
            key: key.into(),
            pages: VecDeque::new(),
            status: QueryStatus::Idle,
            error: None,
            active_request_id: None,
            cache_policy,
            request_policy,
            started_at: None,
            last_updated_at: None,
            cache_hits: 0,
            cancelled_count: 0,
            ignored_results: 0,
            retry_count: 0,
            has_next_page: has_next,
            has_previous_page: has_prev,
            is_fetching_next_page: false,
            is_fetching_previous_page: false,
            max_pages: Some(DEFAULT_MAX_PAGES),
            direction,
            retry_policy: RetryPolicy::default(),
            signal: None,
            #[cfg(feature = "client")]
            current_task: crate::core::current_task::CurrentTask::default(),
        }
    }
}

#[cfg(feature = "client")]
impl<T, E> InfiniteQueryResource<T, E> {
    /// Store a new background task, cancelling any previously stored task.
    pub(crate) fn set_current_task(&mut self, task: gpui::Task<()>) {
        self.current_task.set(task);
    }

    /// Abort the stored background task, if any.
    pub(crate) fn abort_current_task(&mut self) {
        self.current_task.abort();
    }
}
