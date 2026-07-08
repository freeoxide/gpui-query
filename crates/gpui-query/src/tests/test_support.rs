//! Shared test infrastructure for gpui-query.
//!
//! Provides:
//! - [`TestAppContext`] setup helpers via [`setup_test`] / [`setup_query_client`]
//! - [`QueryClient`] as a [`Global`] for tests via [`setup_query_client`]
//! - Core resource constructors: [`test_resource`], [`test_resource_with_policies`],
//!   [`resource_with_sequencer`]
//! - Assertion helpers: [`assert_status`], [`begin_request_id`]
//! - Cache/mutation option factories: [`no_retry_mutation_options`]
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

#[cfg(feature = "client")]
use crate::client::QueryClient;
use crate::core::{
    CachePolicy, QueryBeginResult, QueryFetchMode, QueryKey, QueryResource, QueryStatus, RequestId,
    RequestPolicy, RequestSequencer,
};
#[cfg(feature = "hook")]
use crate::hook::MutationOptions;

// ── TestAppContext setup ───────────────────────────────────────────────

/// Install a default [`QueryClient`] as a [`Global`] on the given context.
///
/// Call this at the start of any integration test that needs the client
/// layer. After this, `cx.global::<QueryClient>()` and
/// `cx.update_global::<QueryClient, _>(…)` are available.
///
/// [`setup_test`] is the preferred shorter alias for this helper.
#[cfg(feature = "client")]
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
#[cfg(feature = "client")]
pub fn setup_test(cx: &mut TestAppContext) {
    setup_query_client(cx);
}

/// Install a [`QueryClient`] with custom policies as a [`Global`].
#[cfg(feature = "client")]
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
#[cfg(feature = "client")]
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
#[expect(dead_code, reason = "kept as a shared lifecycle-test helper")]
pub fn resource_with_sequencer(
    key: impl Into<QueryKey>,
) -> (QueryResource<&'static str>, RequestSequencer) {
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
    now_ms: u64,
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

/// Accept the current request by `request_id` and complete it with success.
///
/// Convenience wrapper around [`QueryResource::complete_current_success`]
/// (audit fix #85). Mirrors [`begin_request_id`] so tests that just need to
/// drive a request through to `Success` can do so in one call without
/// repeating the `(request_id, data, now_ms)` triple inline.
///
/// Returns `true` if the request was the active one and was completed.
pub fn complete_success_id<T, E>(
    r: &mut QueryResource<T, E>,
    request_id: RequestId,
    data: T,
    now_ms: u64,
) -> bool {
    r.complete_current_success(request_id, data, now_ms)
}

// ── Cache/mutation option factories ────────────────────────────────────

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
pub(crate) struct DummyView;

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
#[cfg(feature = "client")]
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

/// A minimal generic test harness that owns a single entity handle.
///
/// Audit fix #47: many hook-layer tests define a one-off `struct H { entity:
/// Entity<...> }` purely to host hook calls via `cx.new(|cx| ...)` and later
/// inspect the entity. [`HookHarness`] replaces that boilerplate for the common
/// single-entity case: tests construct `cx.new(|cx| HookHarness::new(entity))`
/// and read back via `harness.read(cx).entity.read(cx)`.
///
/// This is intentionally narrow (one entity). Tests that need to hold several
/// handles, observer subscriptions, or counters should keep their bespoke
/// harness struct — full migration of every harness is explicitly optional.
pub struct HookHarness<T> {
    /// The hook-managed entity under test.
    pub entity: Entity<T>,
}

impl<T> HookHarness<T> {
    /// Wrap a single entity handle in a harness.
    pub fn new(entity: Entity<T>) -> Self {
        Self { entity }
    }
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

/// A simple post type tag for integration tests. Only used as a type
/// parameter (`client.resource::<Post, _>`), never constructed with data.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Post;

// ── Time helpers ───────────────────────────────────────────────────────

/// A fixed "now" timestamp for deterministic cache tests (ms since UNIX epoch).
// Audit fix #123: removed `#[allow(dead_code)]` — used by request_policy/lifecycle tests.
pub const TEST_NOW_MS: u64 = 1_000_000;

/// Assert that every value in `cases` survives a JSON serialize -> deserialize
/// roundtrip unchanged. Shared helper for the serde-roundtrip tests (extends
/// audit #129).
pub fn assert_serde_roundtrip<T>(cases: &[T])
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    for value in cases {
        let json = serde_json::to_string(value).expect("serialize");
        let back: T = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            &back, value,
            "serde roundtrip changed value (json={})",
            json
        );
    }
}
