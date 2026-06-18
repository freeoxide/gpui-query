//! Shared test infrastructure for gpui-query.
//!
//! Provides:
//! - [`TestAppContext`] setup helpers via [`setup_test`] / [`setup_query_client`]
//! - [`QueryClient`] as a [`Global`] for tests via [`setup_query_client`]
//! - Core resource constructors: [`test_resource`], [`test_resource_with_policies`],
//!   [`resource_with_sequencer`]
//! - Assertion helpers: [`assert_status`], [`begin_request_id`]
//! - Cache/mutation option factories: [`no_cache_options`], [`ttl_zero_options`],
//!   [`no_retry_mutation_options`]
//! - Async test helpers: [`Gate`], [`run_until_parked_and_read`],
//!   [`observe_with_dummy_view`], [`DummyView`]
//!
//! # Usage
//!
//! ```ignore
//! use crate::tests::test_support::*;
//!
//! #[gpui::test]
//! fn my_test(cx: &mut TestAppContext) {
//!     setup_test(cx);
//!     cx.update(|cx| {
//!         // ... test code using cx.global::<QueryClient>() ...
//!     });
//! }
//! ```

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{App, AppContext as _, BackgroundExecutor, Entity, TestAppContext};

use crate::client::QueryClient;
#[cfg(feature = "hook")]
use crate::hook::{MutationOptions, QueryOptions};
use crate::core::{
    CachePolicy, QueryBeginResult, QueryFetchMode, QueryKey, QueryResource, QueryStatus,
    RequestId, RequestPolicy, RequestSequencer,
};

// ── TestAppContext setup ───────────────────────────────────────────────

/// Install a default [`QueryClient`] as a [`Global`] on the given context.
///
/// Call this at the start of any integration test that needs the client
/// layer. After this, `cx.global::<QueryClient>()` and
/// `cx.update_global::<QueryClient, _>(…)` are available.
///
/// [`setup_test`] is the preferred shorter alias for this helper.
pub fn setup_query_client(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(QueryClient::new());
    });
}

/// Preferred entry point for test setup. Installs a default [`QueryClient`]
/// as a [`Global`] on the context. Equivalent to [`setup_query_client`].
///
/// Tests that need custom policies should use [`setup_query_client_with_policies`]
/// or [`setup_query_client_with_gc`] instead.
pub fn setup_test(cx: &mut TestAppContext) {
    setup_query_client(cx);
}

/// Install a [`QueryClient`] with custom policies as a [`Global`].
pub fn setup_query_client_with_policies(
    cx: &mut TestAppContext,
    cache_policy: CachePolicy,
    request_policy: RequestPolicy,
) {
    cx.update(|cx| {
        cx.set_global(QueryClient::with_policies(cache_policy, request_policy));
    });
}

/// Install a [`QueryClient`] with a custom GC time.
pub fn setup_query_client_with_gc(cx: &mut TestAppContext, gc_time_ms: u64) {
    cx.update(|cx| {
        cx.set_global(QueryClient::new().with_gc_time(gc_time_ms));
    });
}

// ── Core resource constructors ────────────────────────────────────────

/// Create a test resource with default policies (TTL 1s, LatestWins).
pub fn test_resource() -> QueryResource<&'static str> {
    QueryResource::new(
        "test",
        CachePolicy::Ttl { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    )
}

/// Create a test resource with custom policies.
pub fn test_resource_with_policies(
    key: impl Into<QueryKey>,
    cache_policy: CachePolicy,
    request_policy: RequestPolicy,
) -> QueryResource<&'static str> {
    QueryResource::new(key, cache_policy, request_policy)
}

/// Create a [`RequestSequencer`] for use in lifecycle tests.
pub fn test_sequencer() -> RequestSequencer {
    RequestSequencer::new()
}

/// Create a fresh resource paired with a new [`RequestSequencer`].
///
/// Returns `(QueryResource, RequestSequencer)` so tests can immediately call
/// `begin_request(&mut r, &mut seq, now, mode)` without boilerplate. The
/// sequencer is a fresh `RequestSequencer::new()`.
///
/// Uses `CachePolicy::NoCache` so every `begin_request` returns `Started`
/// (never `CacheHit`), giving deterministic control over each fetch lifecycle
/// step without worrying about TTL freshness windows.
pub fn resource_with_sequencer(
    key: impl Into<QueryKey>,
) -> (
    QueryResource<&'static str>,
    RequestSequencer,
) {
    (
        QueryResource::new(key, CachePolicy::NoCache, RequestPolicy::LatestWins),
        RequestSequencer::new(),
    )
}

// ── Assertion helpers ──────────────────────────────────────────────────

/// Assert that a resource has the expected status.
// Audit fix #123: removed `#[allow(dead_code)]` — used by request_policy/lifecycle tests.
pub fn assert_status(resource: &QueryResource<impl Clone, impl Clone>, expected: QueryStatus) {
    let actual = resource.status();
    assert_eq!(
        actual, expected,
        "expected status {:?} but got {:?}",
        expected, actual
    );
}

// ── Resource factories for state-transition tests ──────────────────────

/// Create a resource with `NoCache` + `LatestWins`.
///
/// Every `begin_request` on this resource will return `Started` (never `CacheHit`),
/// making it ideal for state-transition tests that want deterministic control
/// over every fetch lifecycle step without worrying about TTL freshness windows.
pub fn nocache_resource(key: impl Into<QueryKey>) -> QueryResource<&'static str> {
    QueryResource::new(key, CachePolicy::NoCache, RequestPolicy::LatestWins)
}

/// Create a fresh resource with a fixed key for state-transition invariant tests.
///
/// Convenience alias for [`nocache_resource`] with key `"invariant-test"`.
/// Every `begin_request` on this resource will return `Started` (never `CacheHit`).
// Audit fix #123: removed `#[allow(dead_code)]` — used by coverage_gaps tests.
pub fn fresh_resource() -> QueryResource<&'static str> {
    nocache_resource("invariant-test")
}

/// Begin a request on the resource and extract the `RequestId`.
///
/// Panics with a descriptive message if the result is anything other than `Started`.
/// Use this in tests that need the `request_id` for subsequent `complete_*` calls
/// but don't care about the full `QueryBeginResult`.
pub fn begin_request_id(
    r: &mut QueryResource<impl Clone, impl Clone>,
    seq: &mut RequestSequencer,
    now_ms: u128,
    mode: QueryFetchMode,
) -> RequestId {
    match r.begin_request(seq, now_ms, mode) {
        QueryBeginResult::Started { request_id, .. } => request_id,
        other => panic!(
            "begin_request_id() expected Started, got {:?} \
             (status={:?}, active_request_id={:?})",
            other,
            r.status(),
            r.active_request_id(),
        ),
    }
}

// ── Cache/mutation option factories ────────────────────────────────────

/// Build [`QueryOptions`] for a key with [`CachePolicy::NoCache`].
///
/// Equivalent to `QueryOptions::new(key).cache_policy(CachePolicy::NoCache)`.
/// Every `begin_request` on the resulting resource will return `Started`
/// (never `CacheHit`), so each `use_query`/`fetch_query` triggers a new fetch.
#[cfg(feature = "hook")]
pub fn no_cache_options(key: impl Into<QueryKey>) -> QueryOptions {
    QueryOptions::new(key).cache_policy(CachePolicy::NoCache)
}

/// Build [`QueryOptions`] for a key with `CachePolicy::Ttl { ttl_ms: 0 }`.
///
/// Equivalent to `QueryOptions::new(key).cache_policy(CachePolicy::Ttl { ttl_ms: 0 })`.
/// With TTL=0, data is fresh only at age=0, so any subsequent `begin_request`
/// at a later timestamp triggers a new fetch — useful for tests that want
/// `can_short_circuit()` to be true (unlike `NoCache`) but still want every
/// fetch to run.
#[cfg(feature = "hook")]
pub fn ttl_zero_options(key: impl Into<QueryKey>) -> QueryOptions {
    QueryOptions::new(key).cache_policy(CachePolicy::Ttl { ttl_ms: 0 })
}

/// Build [`MutationOptions`] with `RetryPolicy::no_retries()` and the
/// standard test GC time (`gc_time_ms: 300_000`).
///
/// Equivalent to:
/// ```ignore
/// MutationOptions {
///     retry_policy: RetryPolicy::no_retries(),
///     gc_time_ms: 300_000,
/// }
/// ```
#[cfg(feature = "hook")]
pub fn no_retry_mutation_options() -> MutationOptions {
    MutationOptions {
        retry_policy: crate::core::RetryPolicy::no_retries(),
        gc_time_ms: 300_000,
    }
}

// ── Async test helpers ─────────────────────────────────────────────────

/// A minimal `DummyView` unit struct for tests that need a view entity to
/// drive `observer.observe(cx)` on a hook-managed entity.
///
/// Many hook-layer tests create a local `struct DummyView;` inside the test
/// body just to host an observer subscription. This shared struct lets those
/// tests call `cx.new(|_| DummyView)` without redefining the unit type each
/// time, and pairs with [`observe_with_dummy_view`] for the
/// create-view-then-observe dance.
#[derive(Default)]
pub struct DummyView;

/// Create a `DummyView` entity, then run `observer.observe(cx)` inside a
/// view-scoped update. Returns the [`gpui::Subscription`] (or `None` if the
/// observer's weak reference is no longer live).
///
/// This collapses the common pattern:
/// ```ignore
/// struct DummyView;
/// let view = cx.new(|_| DummyView);
/// let sub = view.update(cx, |_view, cx| observer.observe(cx));
/// ```
pub fn observe_with_dummy_view<T, E>(
    cx: &mut App,
    observer: &mut crate::client::QueryObserver<T, E>,
) -> Option<gpui::Subscription>
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    let view: Entity<DummyView> = cx.new(|_| DummyView);
    view.update(cx, |_view, cx| observer.observe(cx))
}

/// Run the executor until parked, then read from `entity` inside a fresh
/// `cx.update` closure.
///
/// Equivalent to:
/// ```ignore
/// cx.run_until_parked();
/// cx.update(|cx| {
///     let value = entity.read_with(cx, |state, _| /* projection */);
///     // ... assert on value ...
/// });
/// ```
/// The closure receives the borrowed state and inner `App`, mirroring
/// `entity.read_with` so callers don't need to re-wrap each read.
pub fn run_until_parked_and_read<T, R>(
    cx: &mut TestAppContext,
    entity: &Entity<T>,
    f: impl FnOnce(&T, &App) -> R,
) -> R
where
    T: 'static,
{
    cx.run_until_parked();
    cx.update(|cx| entity.read_with(cx, f))
}

/// A one-shot async gate for tests that need to hold a fetcher/mutation in
/// flight while a second call is issued.
///
/// Replaces the busy-wait `while !gate.load(Ordering::Acquire) {
/// executor.timer(Duration::from_millis(1)).await; }` pattern duplicated
/// across several hook tests. The gate is `Arc`-friendly and clonable so it
/// can be moved into a `move || async move { ... }` fetcher closure.
///
/// # Usage
///
/// ```ignore
/// use crate::tests::test_support::*;
///
/// let gate = Gate::new();
/// let gate_clone = gate.clone();
/// let executor = cx.background_executor.clone();
/// let harness = cx.new(|cx| {
///     fetch_query(
///         &entity,
///         move || {
///             let gate = gate.clone();
///             async move {
///                 gate.wait(&executor).await;
///                 Ok::<_, QueryError>("first")
///             }
///         },
///         cx,
///     );
///     // issue a second call while the first is gated...
/// });
///
/// gate.release();
/// cx.run_until_parked();
/// ```
pub struct Gate {
    inner: Arc<Mutex<bool>>,
}

impl Clone for Gate {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

impl Gate {
    /// Create a closed gate — [`Gate::wait`] will block until [`Gate::release`]
    /// is called.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(false)),
        }
    }

    /// Release the gate, unblocking all current and future [`Gate::wait`]
    /// callers. Releasing twice is a no-op.
    pub fn release(&self) {
        *self.inner.lock().unwrap() = true;
    }

    /// Returns `true` once [`Gate::release`] has been called.
    pub fn is_released(&self) -> bool {
        *self.inner.lock().unwrap()
    }

    /// Wait until the gate is released, polling the `executor` with 1ms timers
    /// (matching the prior busy-wait pattern). When released, returns
    /// immediately. Uses `executor.timer()` so the wait is async-friendly
    /// and does not block the executor thread.
    pub async fn wait(&self, executor: &BackgroundExecutor) {
        while !self.is_released() {
            executor.timer(Duration::from_millis(1)).await;
        }
    }
}

// ── Test fixture types ─────────────────────────────────────────────────

/// A simple user struct for integration tests.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct User {
    pub id: u32,
    pub name: String,
}

impl User {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
        }
    }
}

/// A simple post struct for integration tests.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Post {
    pub id: u32,
    pub title: String,
}

// ── Time helpers ───────────────────────────────────────────────────────

/// A fixed "now" timestamp for deterministic cache tests (ms since UNIX epoch).
// Audit fix #123: removed `#[allow(dead_code)]` — used by request_policy/lifecycle tests.
pub const TEST_NOW_MS: u128 = 1_000_000;
