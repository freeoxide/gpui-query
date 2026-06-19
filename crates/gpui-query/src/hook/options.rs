//! Query and mutation options with builder pattern and sensible defaults.
//!
//! **v2**: All options use `Default` and `From<&str>` so users can pass just
//! a string key for the simplest case.

use std::sync::Arc;

use crate::core::{CachePolicy, RefetchTrigger, RequestPolicy, RetryPolicy};

/// Options for `use_query` and `fetch_query`.
///
/// # Quick Start
///
/// ```no_run
/// use gpui_query::QueryOptions;
/// use gpui_query::core::{CachePolicy, RetryPolicy};
/// use gpui_query::hook::use_query;
/// # #[derive(Clone)]
/// # struct User;
/// # #[derive(Clone, Debug)]
/// # struct MyError;
/// # fn _doc(cx: &mut gpui::Context<()>) {
///
/// // Simplest: just a string key
/// let result = use_query("users", |signal| async move {
///     Ok::<Vec<User>, MyError>(vec![])
/// }, cx);
///
/// // With options:
/// let result = use_query(
///     QueryOptions::new("users")
///         .cache_policy(CachePolicy::Ttl { ttl_ms: 300_000 })
///         .retry_policy(RetryPolicy::new(5)),
///     |signal| async move {
///         Ok::<Vec<User>, MyError>(vec![])
///     },
///     cx,
/// );
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct QueryOptions {
    /// The query key. Can be a string or multi-segment key.
    pub key: crate::core::QueryKey,
    /// Cache policy. Default: Ttl { ttl_ms: 60_000 }.
    pub cache_policy: CachePolicy,
    /// Request policy. Default: LatestWins.
    pub request_policy: RequestPolicy,
    /// Retry policy. Default: 3 retries with exponential backoff.
    pub retry_policy: RetryPolicy,
    /// GC time in milliseconds. Default: 300_000 (5 minutes).
    ///
    /// **Reserved / forward-compat** (audit fix #80): settable via the
    /// `.gc_time(ms)` builder, but **not yet consumed** by `use_query`,
    /// `fetch_query`, or the bucket layer. Garbage collection currently runs
    /// off the global GC time set via [`QueryClient::with_gc_time`]; this
    /// per-query value is stored only so a future release can honor it without
    /// a breaking API change. Setting it has no effect today.
    pub gc_time_ms: u64,
    /// Whether to keep previous data when the key changes.
    ///
    /// **Reserved / forward-compat** (audit fix #80): settable via the
    /// `.keep_previous()` builder, but **not yet consumed** by `use_query` or
    /// `use_query_manual`. The `placeholderData`/`keepPreviousData` behavior is
    /// not yet implemented; the field is stored so a future release can honor
    /// it without a breaking API change. Setting it has no effect today.
    pub keep_previous_data: bool,
    /// Whether to force a fetch (ignore cache).
    ///
    /// When `true`, `use_query` passes `QueryFetchMode::Force` to
    /// `begin_request`, bypassing cache freshness checks and always starting
    /// a new fetch.
    pub force_fetch: bool,
    /// Refetch on mount trigger.
    ///
    /// **Reserved / forward-compat** (audit fix #80): settable on the struct,
    /// but **not yet consumed**. The GPUI event-system integration for
    /// automatic refetching on component mount is not yet implemented; the
    /// field is stored so a future release can honor it without a breaking API
    /// change. Setting it has no effect today.
    pub refetch_on_mount: RefetchTrigger,
    /// Refetch on window focus trigger.
    ///
    /// **Reserved / forward-compat** (audit fix #80): settable on the struct,
    /// but **not yet consumed**. The GPUI event-system integration for
    /// automatic refetching on window focus is not yet implemented; the field
    /// is stored so a future release can honor it without a breaking API
    /// change. Setting it has no effect today.
    pub refetch_on_window_focus: RefetchTrigger,
    /// Refetch on reconnect trigger.
    ///
    /// **Reserved / forward-compat** (audit fix #80): settable on the struct,
    /// but **not yet consumed**. The GPUI event-system integration for
    /// automatic refetching on reconnect is not yet implemented; the field is
    /// stored so a future release can honor it without a breaking API change.
    /// Setting it has no effect today.
    pub refetch_on_reconnect: RefetchTrigger,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            // #77: The default key cannot be a `const` because `QueryKey`
            // wraps an `Arc<[Arc<str>]>` and `Arc::from` is not const-stable,
            // so `QueryKey` itself is not const-constructable. This is a
            // single allocation per `QueryOptions::default()` call and is not
            // on a hot path (`Default` is only invoked when a caller opts out
            // of supplying a key, e.g. `use_mutation((), cx)`), so the runtime
            // cost is acceptable. The `Arc` also means cloning the resulting
            // default key is a single refcount bump.
            key: crate::core::QueryKey::from("default"),
            cache_policy: CachePolicy::default(),
            request_policy: RequestPolicy::default(),
            retry_policy: RetryPolicy::default(),
            gc_time_ms: 300_000,
            keep_previous_data: false,
            force_fetch: false,
            refetch_on_mount: RefetchTrigger::default(),
            refetch_on_window_focus: RefetchTrigger::default(),
            refetch_on_reconnect: RefetchTrigger::default(),
        }
    }
}

/// Declarative macro that generates the byte-for-byte equivalent builder
/// methods shared by [`QueryOptions`] and [`InfiniteQueryOptions`]
/// (`cache_policy`, `request_policy`, `retry_policy`, `gc_time`).
///
/// Audit fix #44: collapses the duplicated builders into a single source of
/// truth so the two option types cannot drift.
macro_rules! impl_query_options_builders {
    ($t:ident) => {
        impl $t {
            /// Set the cache policy.
            pub fn cache_policy(mut self, policy: CachePolicy) -> Self {
                self.cache_policy = policy;
                self
            }

            /// Set the request policy.
            pub fn request_policy(mut self, policy: RequestPolicy) -> Self {
                self.request_policy = policy;
                self
            }

            /// Set the retry policy.
            pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
                self.retry_policy = policy;
                self
            }

            /// Set the GC time in milliseconds.
            pub fn gc_time(mut self, ms: u64) -> Self {
                self.gc_time_ms = ms;
                self
            }
        }
    };
}

impl QueryOptions {
    /// Create options with just a key.
    pub fn new(key: impl Into<crate::core::QueryKey>) -> Self {
        Self {
            key: key.into(),
            ..Default::default()
        }
    }

    /// Force a fetch, ignoring cache.
    ///
    /// When set, `use_query` passes `QueryFetchMode::Force` to `begin_request`,
    /// which bypasses cache freshness checks and always starts a new fetch.
    pub fn force(mut self) -> Self {
        self.force_fetch = true;
        self
    }

/// Keep previous data when the key changes.
    ///
    /// **Reserved / forward-compat** (audit fix #80): sets the
    /// `keep_previous_data` field, which is **not yet consumed** by `use_query`
    /// or `use_query_manual`. The `keepPreviousData` behavior is intended for a
    /// future release (preserve the prior `data`/`previous_data` slot across a
    /// key change so the component keeps rendering the last successful result
    /// while the new fetch is in flight). The builder is provided now so callers
    /// can opt in without a future API change; calling it has no effect today.
    pub fn keep_previous(mut self) -> Self {
        self.keep_previous_data = true;
        self
    }
}

impl_query_options_builders!(QueryOptions);

impl From<&str> for QueryOptions {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

impl From<String> for QueryOptions {
    fn from(key: String) -> Self {
        Self::new(key)
    }
}

impl From<crate::core::QueryKey> for QueryOptions {
    fn from(key: crate::core::QueryKey) -> Self {
        Self::new(key)
    }
}

/// Build [`QueryOptions`] from a raw `(key, cache_policy, request_policy)`
/// triple.
///
/// Audit fix #79: this lets `use_query_manual_opts` /
/// `use_query_unsignalled_opts` accept callers that already hold the legacy
/// raw-parameter triple without forcing them to spell out `QueryOptions::new`.
/// Non-breaking: the existing constructors and `From` impls are untouched.
impl From<(crate::core::QueryKey, CachePolicy, RequestPolicy)> for QueryOptions {
    fn from((key, cache_policy, request_policy): (crate::core::QueryKey, CachePolicy, RequestPolicy)) -> Self {
        Self {
            key,
            cache_policy,
            request_policy,
            ..Default::default()
        }
    }
}

/// Options for `use_mutation`.
#[derive(Clone, Debug)]
pub struct MutationOptions {
    /// Retry policy. Default: no retries.
    pub retry_policy: RetryPolicy,
    /// GC time in milliseconds.
    pub gc_time_ms: u64,
}

impl Default for MutationOptions {
    fn default() -> Self {
        Self {
            retry_policy: RetryPolicy::no_retries(),
            gc_time_ms: 300_000,
        }
    }
}

impl MutationOptions {
    /// Set the retry policy.
    ///
    /// Audit fix #43: Mirrors the `.retry_policy(p)` builder on
    /// [`QueryOptions`] so mutation callers can configure retries without
    /// constructing `MutationOptions` via struct literal.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Set the GC time in milliseconds.
    ///
    /// Audit fix #43: Mirrors the `.gc_time(ms)` builder on [`QueryOptions`].
    pub fn gc_time(mut self, ms: u64) -> Self {
        self.gc_time_ms = ms;
        self
    }
}

/// Type alias for the `on_success` callback field on [`MutationCallbacks`].
///
/// Audit fix #96: collapses the `Option<Arc<dyn Fn(&T) + Send + Sync>>`
/// field type so `clippy::type_complexity` does not fire on the struct
/// definition.
pub type MutationSuccessCallback<T> = Option<Arc<dyn Fn(&T) + Send + Sync>>;

/// Type alias for the `on_error` callback field on [`MutationCallbacks`].
pub type MutationErrorCallback<E> = Option<Arc<dyn Fn(&E) + Send + Sync>>;

/// Type alias for the `on_settled` callback field on [`MutationCallbacks`].
pub type MutationSettledCallback<T, E> =
    Option<Arc<dyn Fn(Option<&T>, Option<&E>) + Send + Sync>>;

/// Lifecycle callbacks for mutations.
///
/// Not `Clone` because trait-object callbacks cannot be cloned.
/// Construct with `MutationCallbacks::new()` and the builder methods.
///
/// Callbacks are wrapped in `Arc` so they can be shared across concurrent
/// mutation invocations. `E` should implement `std::fmt::Debug` so that
/// callbacks can log or display error details.
pub struct MutationCallbacks<T, E> {
    /// Fired on terminal success (after all retries skipped or succeeded).
    pub on_success: MutationSuccessCallback<T>,
    /// Fired on terminal failure (after retries exhausted or cancelled).
    pub on_error: MutationErrorCallback<E>,
    /// Fired on every terminal outcome (success, failure, or discard).
    pub on_settled: MutationSettledCallback<T, E>,
    _phantom: std::marker::PhantomData<(T, E)>,
}

impl<T, E> Default for MutationCallbacks<T, E> {
    fn default() -> Self {
        Self {
            on_success: None,
            on_error: None,
            on_settled: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, E> MutationCallbacks<T, E> {
    /// Create empty callbacks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the success callback.
    pub fn on_success(mut self, f: impl Fn(&T) + Send + Sync + 'static) -> Self {
        self.on_success = Some(Arc::new(f));
        self
    }

    /// Set the error callback.
    pub fn on_error(mut self, f: impl Fn(&E) + Send + Sync + 'static) -> Self {
        self.on_error = Some(Arc::new(f));
        self
    }

    /// Set the settled callback (fires on both success and failure).
    pub fn on_settled(mut self, f: impl Fn(Option<&T>, Option<&E>) + Send + Sync + 'static) -> Self {
        self.on_settled = Some(Arc::new(f));
        self
    }
}

/// Options for infinite queries.
#[derive(Clone, Debug)]
pub struct InfiniteQueryOptions {
    /// The query key.
    pub key: crate::core::QueryKey,
    /// Cache policy.
    pub cache_policy: CachePolicy,
    /// Request policy.
    pub request_policy: RequestPolicy,
    /// Maximum pages to retain. Default: 50.
    pub max_pages: Option<usize>,
    /// Retry policy.
    pub retry_policy: RetryPolicy,
    /// GC time in milliseconds. Default: 300_000 (5 minutes).
    pub gc_time_ms: u64,
}

impl Default for InfiniteQueryOptions {
    fn default() -> Self {
        Self {
            key: crate::core::QueryKey::from("default"),
            cache_policy: CachePolicy::default(),
            request_policy: RequestPolicy::default(),
            max_pages: Some(50),
            retry_policy: RetryPolicy::default(),
            gc_time_ms: 300_000,
        }
    }
}

impl InfiniteQueryOptions {
    /// Create with just a key.
    pub fn new(key: impl Into<crate::core::QueryKey>) -> Self {
        Self {
            key: key.into(),
            ..Default::default()
        }
    }

    /// Set max pages. Pass a concrete number to cap retained pages.
    ///
    /// To allow unbounded pages, use [`InfiniteQueryOptions::unbounded_pages`]
    /// instead.
    pub fn max_pages(mut self, max: usize) -> Self {
        self.max_pages = Some(max);
        self
    }

    /// Allow unbounded page accumulation (no limit).
    ///
    /// Sets `max_pages` to `None`, meaning the infinite query will never
    /// evict old pages. Use with caution — unbounded page storage can grow
    /// without limit if the user scrolls far enough.
    pub fn unbounded_pages(mut self) -> Self {
        self.max_pages = None;
        self
    }
}

impl_query_options_builders!(InfiniteQueryOptions);

impl From<&str> for InfiniteQueryOptions {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

impl From<String> for InfiniteQueryOptions {
    fn from(key: String) -> Self {
        Self::new(key)
    }
}

impl From<crate::core::QueryKey> for InfiniteQueryOptions {
    fn from(key: crate::core::QueryKey) -> Self {
        Self::new(key)
    }
}
