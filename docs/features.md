# gpui-query Extensions — Design Plan

**Companion crates for HTTP cache semantics and persistence.**

The core `gpui-query` crate stays cache- and transport-agnostic, mirroring TanStack
Query's philosophy. Concrete helpers live in small companion crates, each behind an
optional feature flag, so apps that don't need them pay nothing.

> **Status:** Verified against `gpui-query` v2 source at
> `/Users/hmziq/fo/gpui-query/crates/gpui-query/src`. API sketches below match the
> real `use_query` fetcher-closure model, `QueryClient::set_query_data`,
> `CachePolicy` enum, and `QueryObserver` — not an invented `Resource` trait.

---

## Table of Contents

1. [Guiding Principles](#guiding-principles)
2. [Extension Points](#extension-points)
3. [Crate Layout](#crate-layout)
4. [Required Core Changes](#required-core-changes)
5. [1. HTTP Cache Helpers — `gpui-query-http`](#1-http-cache-helpers--gpui-query-http)
6. [2. Persistence — `gpui-query-persist`](#2-persistence--gpui-query-persist)
7. [Shared Types](#shared-types)
8. [Integration Plan for gpui-app](#integration-plan-for-gpui-app)
9. [Migration Order](#migration-order)
10. [Test Strategy](#test-strategy)
11. [Open Questions](#open-questions)

---

## Guiding Principles

1. **Core stays agnostic.** `gpui-query` must not import `reqwest`, `http`, or any
   storage backend. Users with gRPC, IPC, or offline-first transports must not pay
   for HTTP semantics they don't use.
2. **One extension point per concern.** HTTP cache and persistence have different
   scopes, triggers, and shapes. Each gets its own minimal surface rather than a
   unified abstraction that fits neither well.
3. **Own both crates, so hoist.** We own `gpui-query` and `gpui-app`. There is zero
   coordination cost to placing reusable helpers in a companion crate rather than
   re-deriving them per app.
4. **Reference adapters ship with the crate; app-specific adapters ship with the app.**
   `FilePersister` is in `gpui-query-persist`. `SqlitePersister` (which depends on
   the app's storage layout) is in `gpui-app`.
5. **No speculative abstractions.** Don't build a generic plugin/middleware trait
   to wrap these concerns. Retry already lives in core; HTTP is a wrapper, not a
   layer; persistence is a trait + a subscription. If a new need appears later,
   add a new small extension point then.
6. **Follow `rust-best-practices`.** Typed errors via `thiserror` in the companion
   crates; `Arc<T>` for shared resources; `impl Future<...> + Send` on trait
   methods to guarantee spawnable futures (preferred over `async fn` in traits,
   which does not guarantee `Send`); `#![deny(missing_docs)]` on every companion
   crate; no `unwrap`/`expect` outside tests.

---

## Extension Points

Two concerns, two shapes. Nothing in common, so no shared abstraction.

| Concern     | Scope        | When               | Shape              | Home crate           |
| ----------- | ------------ | ------------------ | ------------------ | -------------------- |
| HTTP cache  | per-fetch    | load time          | function + wrapper | `gpui-query-http`    |
| Persistence | whole bucket | mutation + startup | trait + subscribe  | `gpui-query-persist` |

- **HTTP cache** is a wrapper around `reqwest` — there is nothing to swap. You
  either talk HTTP or you don't. A struct with a `fetch` method is the correct
  shape, not a trait or middleware.
- **Persistence** is a bucket-level observer. It subscribes to `QueryClient`
  mutations, debounces, and hands a snapshot to a `Persister` trait impl. On
  startup, `hydrate` primes the bucket from the persister.

Each adds ~50 lines of core surface. No plugin system, no middleware chain, no
lifecycle hook bag.

---

## Crate Layout

```
crates/
  gpui-query/              # core: QueryClient (Global), CachePolicy, QueryObserver, Persister trait
  gpui-query-http/         # CacheMeta, cache_policy_from_headers, HttpCache
  gpui-query-persist/      # PersistOptions, FilePersister, PersistHandle
  gpui-app/                # SqlitePersister, endpoint-specific fetchers
```

Feature flags on `gpui-query`:

```toml
[features]
persist = []      # enables Persister trait + persist_with + hydrate in core
```

`gpui-query-http` lives in its own crate and depends on `gpui-query` core only.

---

## Required Core Changes

These are **additions to `gpui-query` core** that the two companion crates depend
on. They are small, feature-gated, and non-breaking for existing consumers.

### 1. Fetcher-returned `CachePolicy` (for HTTP "server wins")

**Problem.** Today the fetcher is `Fn() -> Future<Output = Result<T, E>>` and
returns _only data_ (`hook/mod.rs:17`). `CachePolicy` is fixed at `use_query` /
`fetch_query` call time. There is no way for a fetcher that just read
`Cache-Control: max-age=30` from a response to push that policy back into the
bucket — so HTTP semantics cannot override the caller's per-query policy.

**Fix — pick one (Open Question 1):**

- **Option A (typed wrapper):** introduce a `Fetched<T, E>` result type:

  ```rust
  pub struct Fetched<T, E> {
      pub data: T,
      pub cache_policy: Option<CachePolicy>,  // None → keep caller's policy
      pub meta: Option<serde_json::Value>,    // for CacheMeta round-trip via persist
      _error: PhantomData<E>,
  }
  ```

  `use_query`/`fetch_query` gain a `*_with_policy` variant whose fetcher returns
  `Result<Fetched<T, E>, E>`. The bucket applies `cache_policy` on success. The
  plain `Result<T, E>` fetcher keeps working unchanged.

- **Option B (callback):** keep the fetcher as `Result<T, E>`, but give the
  fetcher closure an injected `&QueryContext` handle it can call
  `ctx.set_cache_policy(...)` / `ctx.set_meta(...)` on. More flexible, more
  state-passing boilerplate.

**Recommendation:** Option A. Explicit, serializable, composes cleanly with
persistence (the `meta` field is exactly what `gpui-query-persist` stores).

### 2. Bucket-mutation observation (for persistence subscription)

**Problem.** The existing `QueryObserver::observe` (`client/observer.rs:153`)
watches a _single resource entity's status transitions_ — it does **not** observe
whole-bucket mutations. `persist_with` needs to know when _any_ tracked entry
changes.

**Fix — pick one (Open Question 2):**

- **Option A (coarse global observer):** `persist_with` subscribes via
  `cx.observe_global::<QueryClient>()`. Because `QueryClient` is a `Global`
  (`client/mod.rs:64`) and every mutation goes through `update_global` + notify,
  this fires on any change. The `PersistOptions::filter` (a `QueryKeyFilter`) and
  `debounce` do the real selection work. Zero core change beyond what exists.
- **Option B (typed dirty signal):** add `QueryClient::notify_buckets_changed`
  called from `set_query_data` / fetch completion, observed by `persist_with`.
  More precise, more core surface.

**Recommendation:** Option A for v1 — it reuses GPUI's existing
`observe_global_in` and the filter+debounce already hides the coarseness. Move to
Option B only if profiling shows the coarse path firing too often.

### 3. `Persister` trait + `persist_with` + `hydrate` (behind `persist` feature)

The persistence trait and the two `QueryClient` methods. Detailed in
[Persistence](#2-persistence--gpui-query-persist). `hydrate` primes via the
**existing** `QueryClient::set_query_data` (`client/mod.rs:274`) — there is no
separate `prime` method and none is needed.

---

## 1. HTTP Cache Helpers — `gpui-query-http`

### Scope

Per-fetch concern. Parse HTTP cache headers, attach conditional request headers on
refetch, and on `304 Not Modified` return the cached body. This is a wrapper around
`reqwest`, not a core abstraction.

### Public API

```rust
// crates/gpui-query-http/src/lib.rs

/// HTTP cache metadata extracted from a response. Serializable so
/// `gpui-query-persist` can store it alongside the body and rehydrate a cold
/// start with valid ETags, enabling cheap 304 refetches on the first request
/// after launch.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CacheMeta {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub stored_at: std::time::Instant,
    pub fresh_for: Duration,
    pub stale_for: Duration,
}

/// Parse Cache-Control / ETag / Last-Modified / Age.
/// Returns `Ok(policy)` derived from the headers, or a typed `ParseError`.
pub fn cache_policy_from_headers(h: &http::HeaderMap) -> Result<CachePolicy, ParseError>;

/// Wraps a `reqwest::Client` (or any client with the same surface) and keeps an
/// in-memory map of `CacheMeta` + cached bodies keyed by URL.
pub struct HttpCache<C = reqwest::Client> {
    client: C,
    meta: std::sync::Mutex<HashMap<reqwest::Url, CacheMeta>>,
    bodies: std::sync::Mutex<HashMap<reqwest::Url, bytes::Bytes>>,
}

impl<C> HttpCache<C> {
    pub fn new(client: C) -> Self;

    /// Fetch a URL. On 200, stores body + meta and returns `(body, policy, meta)`.
    /// On 304, returns the cached body, a refreshed policy, and the stored meta.
    /// On a non-cacheable response, returns `(body, CachePolicy::NoCache, None)`.
    ///
    /// The std `Mutex`es are **never held across an `.await`** — each critical
    /// section is lock/read/drop-lock, then the async request runs, then a second
    /// lock/write/drop-lock. This keeps `HttpCache: Send + Sync` without a tokio
    /// mutex.
    pub async fn fetch(
        &self,
        url: &reqwest::Url,
    ) -> Result<(bytes::Bytes, CachePolicy, Option<CacheMeta>), HttpError>;
}
```

### Usage from a fetcher closure

gpui-query uses a **fetcher closure**, not a `Resource` trait. `HttpCache` is
called _inside_ the closure. The server-returned policy is fed back via the
`Fetched<T, E>` wrapper from [Required Core Change 1](#1-fetcher-returned-cachepolicy-for-http-server-wins):

```rust
// In gpui-app or any consumer
#[derive(Clone, serde::Deserialize)]
struct User { /* ... */ }

let url: reqwest::Url = format!("https://api.example.com/users/{id}").parse()?;
let http = http_cache.clone(); // Arc<HttpCache>

let (entity, _sub) = use_query_with_policy(
    QueryKey::from(["users", &id.to_string()]),
    CachePolicy::Ttl { ttl_ms: 60_000 },   // caller default; server may override
    RequestPolicy::LatestWins,
    move || async move {
        let (bytes, policy, meta) = http.fetch(&url).await?;
        let user: User = serde_json::from_slice(&bytes)?;
        Ok(Fetched {
            data: user,
            cache_policy: Some(policy),     // server wins
            meta: meta.map(|m| serde_json::to_value(&m).unwrap_or_default()),
            _error: PhantomData,
        })
    },
    cx,
);
```

### Design notes

- No middleware, no layer trait. There is nothing to swap — you either talk HTTP
  or you don't. A wrapper struct is the correct shape.
- `HttpCache` owns its own `HashMap` of metadata. It does **not** touch
  `QueryClient`'s bucket. The `CachePolicy` + `CacheMeta` returned by `fetch`
  flow into `QueryClient` via the `Fetched<T, E>` wrapper and the normal fetcher
  completion path, so HTTP semantics _override_ the caller's per-query policy
  mid-flight (server wins) — **once [Required Core Change 1] is implemented**.
- `CacheMeta` is `serde::{Serialize, Deserialize}` so `gpui-query-persist` can
  store it in `PersistedEntry::meta` and rehydrate a cold start with valid ETags,
  enabling cheap `304` refetches on the first request after launch.
- `HttpCache<C>` is generic over the client so tests can inject a stub client
  (static dispatch, per `rust-best-practices` ch.6). The default `C = reqwest::Client`
  keeps the common path ergonomic.

### Out of scope

- Streaming responses, range requests, `Vary` handling. Add only when a real
  consumer needs them.
- Response body storage strategy beyond an in-memory `HashMap`. If a consumer
  needs disk-backed body storage, they implement their own `HttpCache`-shaped
  wrapper.

---

## 2. Persistence — `gpui-query-persist`

### Scope

Whole-bucket concern. Observe `QueryClient` mutations, debounce, serialize, and
hand to a `Persister`. On startup, rehydrate entries via the existing
`QueryClient::set_query_data` (`client/mod.rs:274`).

### Core surface (added to `gpui-query`, behind `persist` feature)

```rust
// crates/gpui-query/src/persist.rs (new module)

/// Persistence backend. Implemented by `FilePersister` (companion crate),
/// `SqlitePersister` (app), `SecureStoragePersister` (app), etc.
///
/// Methods return `impl Future<...> + Send` (not `async fn`) so the returned
/// futures are guaranteed `Send` and can be spawned on the tokio runtime. This
/// makes the trait **not object-safe**; use generics (`persist_with<P>`) for
/// static dispatch. If runtime dispatch (`Box<dyn Persister>`) is ever needed,
/// switch the return type to `Pin<Box<dyn Future<...> + Send>>` (Open Question 5).
pub trait Persister: Send + Sync + 'static {
    /// Load every persisted entry. Called once at startup by `hydrate`.
    fn load(&self) -> impl Future<Output = Result<HashMap<QueryKey, PersistedEntry>, PersistError>> + Send;

    /// Save a snapshot. Called (debounced) on every qualifying bucket mutation.
    fn save(&self, snapshot: &PersistSnapshot) -> impl Future<Output = Result<(), PersistError>> + Send;
}

pub struct PersistedEntry {
    pub value: serde_json::Value,
    pub cached_at: Instant,
    pub cache_policy: CachePolicy,        // CachePolicy is Copy + Serialize (core/policy.rs:5)
    pub meta: Option<serde_json::Value>,  // CacheMeta from gpui-query-http, if present
}

pub struct PersistSnapshot {
    pub entries: HashMap<QueryKey, PersistedEntry>,
    pub version: u32,
}

pub struct PersistOptions {
    pub filter: QueryKeyFilter,           // reuse core's existing filter (core/key_filter.rs)
    pub max_age: Duration,                // refuse to rehydrate entries older than this
    pub debounce: Duration,               // coalesce rapid mutations
    pub dehydrate_filter: DehydrateFilter, // skip entries whose value is too large, etc.
}

pub struct PersistHandle(/* drops to unsubscribe */);

impl QueryClient {
    /// Subscribe to bucket mutations, debounce, and call `persister.save`.
    /// Subscription uses `cx.observe_global::<QueryClient>()`
    /// (Required Core Change 2, Option A). The `filter` + `debounce` hide the
    /// coarseness of the global observer. Dropping the handle unsubscribes.
    pub fn persist_with<P: Persister>(&self, p: P, opts: PersistOptions) -> PersistHandle;

    /// Run once at startup. Loads entries from `persister` (on the tokio
    /// runtime), filters by `max_age` and `opts.filter`, then primes each entry
    /// on the gpui foreground thread via `self.set_query_data(...)` (creating
    /// the resource first with `self.resource::<T, E>(key, cx)` if needed).
    /// Returns the number of entries rehydrated.
    pub async fn hydrate<P: Persister>(
        &self,
        p: &P,
        opts: &PersistOptions,
    ) -> Result<usize, PersistError>;
}
```

The `Persister` trait lives in core because it's tiny (two methods), central, and
defines a contract every adapter must satisfy. The trait is the extension point;
the adapters are not.

### Companion crate

```rust
// crates/gpui-query-persist/src/lib.rs

pub struct FilePersister {
    path: PathBuf,
    format: PersistFormat,   // Json | Bincode
}

impl Persister for FilePersister { /* ... */ }

pub struct NoopPersister;     // for tests
impl Persister for NoopPersister { /* ... */ }
```

### App-specific adapters (in `gpui-app`)

```rust
// src/services/storage/query_persister.rs (new)
pub struct SqlitePersister<'a> { pool: &'a sqlx::SqlitePool, table: &'static str }

impl Persister for SqlitePersister<'_> { /* ... */ }
```

This stays in the app because it depends on the app's existing storage layout
(`src/services/storage`) and migrations.

### Hydration sequence at startup

`Persister::load()` does file/SQLite IO and **must run on the tokio runtime**,
not gpui's foreground executor. The priming step runs on the gpui foreground
thread via `cx.update_global`. The app already bridges tokio↔gpui this way (see
`query_playground/queries/actions.rs:546` `run_http`).

```
app startup (App context)
  │
  ├── cx.set_global(QueryClient::new(...))
  ├── cx.spawn(async move |cx| {
  │     // load() runs on tokio (IO); the existing shared runtime is used
  │     let entries = tokio_runtime.spawn(async move { persister.load().await }).await??;
  │     // prime on the gpui foreground thread
  │     cx.update_global::<QueryClient, _>(|client, cx| {
  │         for (key, entry) in entries {
  │             client.set_query_data::<T, E>(&key, /* deserialized */, cx);
  │         }
  │     }).ok();
  │     // begin observing mutations
  │     let _handle = cx.update_global::<QueryClient, _>(|client, _| {
  │         client.persist_with(persister, opts)
  │     });
  │     // store _handle for app lifetime
  │ }).detach();
  ├── gpui window opens
  └── use_query hooks observe already-primed entries → render cached data
                                                    → refetch in background
```

`persist_with`'s `save()` hops the other way: the debounced observer callback
runs on the gpui foreground thread, collects the snapshot, then
`tokio_runtime.spawn(async move { persister.save(&snapshot).await })` writes it
out without blocking the UI.

### Interaction with `gpui-query-http`

The one place these two crates cooperate: persisting `CacheMeta` so cold-start
refetches send `If-None-Match` and get cheap `304`s. `PersistedEntry::meta` is
typed `Option<serde_json::Value>` (opaque to core) so `gpui-query-persist` does
**not** need to depend on `gpui-query-http`. The HTTP crate serializes `CacheMeta`
into that `Value` on save and deserializes it back on load. Zero coupling between
the two companion crates; they share only the `serde_json::Value` contract and
the `CachePolicy` type from core.

### Design notes

- **No generic mutation hook.** `persist_with` is specific: it subscribes via
  `cx.observe_global::<QueryClient>()` and calls one trait method. Adding a
  general `on_mutate` hook would invite misuse and bloat core.
- **Server wins.** `CachePolicy` stored in `PersistedEntry` is whatever the
  server last returned (via `Fetched.cache_policy`). On rehydrate, that policy
  applies until the next fetch refreshes it.
- **Debounce is mandatory.** `PersistOptions::debounce` defaults to 1s; passing
  `Duration::ZERO` is allowed but discouraged for non-toy persisters.
- **Versioning.** `PersistSnapshot::version` is bumped on every breaking change
  to `PersistedEntry`'s schema. `hydrate` rejects mismatched versions with a
  typed error; apps decide whether to wipe and continue or surface the error.
- **`Send + Sync + 'static`** on `Persister` is required because `save`/`load`
  are spawned on the tokio runtime (`rust-best-practices` ch.9).

---

## Shared Types

| Type              | Lives in             | Used by                                          |
| ----------------- | -------------------- | ------------------------------------------------ |
| `CachePolicy`     | `gpui-query` (core)  | core, http, persist                              |
| `QueryKey`        | `gpui-query` (core)  | core, persist                                    |
| `QueryKeyFilter`  | `gpui-query` (core)  | core, persist                                    |
| `Fetched<T, E>`   | `gpui-query` (core)  | core, http, app                                  |
| `Persister` trait | `gpui-query` (core)  | core, persist, app                               |
| `PersistedEntry`  | `gpui-query` (core)  | core, persist, app                               |
| `CacheMeta`       | `gpui-query-http`    | http, app; persist sees only `serde_json::Value` |
| `HttpCache`       | `gpui-query-http`    | http, app                                        |
| `FilePersister`   | `gpui-query-persist` | persist, app (optional)                          |
| `SqlitePersister` | `gpui-app`           | app only                                         |

`CacheMeta` never crosses into core or persist as a typed value — only as
`serde_json::Value`. `CachePolicy`, `Fetched`, and `Persister` are the only types
that cross crate boundaries as typed values.

---

## Integration Plan for gpui-app

Concrete steps once the crates exist:

1. **`Cargo.toml`** — add `gpui-query-http` and `gpui-query-persist` deps. Enable
   the `persist` feature on `gpui-query`.
2. **`src/services/tokio_runtime.rs`** — keep the shared `reqwest::Client`
   (`tokio_runtime.rs:12`); wrap it once in `HttpCache` and expose
   `http_cache: Arc<HttpCache>` alongside `http_client`.
3. **`src/features/pages/query_playground/queries/actions.rs:546`** — replace the
   raw `reqwest::Client` body of `run_http` with `HttpCache::fetch`. Switch the
   fetcher from `Result<HttpFetchResult, QueryError>` to
   `Result<Fetched<HttpFetchResult, QueryError>, QueryError>` via
   `use_query_with_policy` so the server-returned `CachePolicy` flows back.
4. **`src/services/storage/query_persister.rs`** (new) — `SqlitePersister`
   implementing `Persister`, writing to a `query_cache` table in the app's
   existing SQLite database. Migrations live with the rest of the app's
   migrations.
5. **App init** (wherever `QueryClient` is constructed today, per the devtools
   wiring in `docs/completed/query-devtools.md:147`) — after
   `cx.set_global(QueryClient::new(...))`, run the `cx.spawn` hydration sequence
   from [Hydration sequence](#hydration-sequence-at-startup), then hold the
   `PersistHandle` for the app lifetime (store it on a long-lived entity so it
   isn't dropped).
6. **`query_playground`** — add a "Persistence" section card showing: hydrate
   count on startup, last save timestamp, current persisted key count. This
   doubles as the integration test surface.

---

## Migration Order

The order matters because persistence touches core and HTTP depends on the
`Fetched<T, E>` core change.

1. **Core change 1 — `Fetched<T, E>` + `use_query_with_policy`** in
   `gpui-query`. Non-breaking: the existing `use_query` signature is unchanged;
   the new variant is additive. Add unit tests that a `Fetched` with
   `Some(policy)` overrides the bucket's policy and `None` keeps it.
2. **Core change 3 — `Persister` trait + `persist_with` + `hydrate`** in
   `gpui-query` behind the `persist` feature. `persist_with` uses Core change 2
   Option A (`observe_global`) so it ships together. Ship `FilePersister` in
   `gpui-query-persist`. Add unit tests for debounce, `max_age` filtering, and
   version mismatch — these need `#[gpui::test]` + `TestAppContext` since they
   touch `QueryClient`.
3. **`gpui-query-http`** with `CacheMeta`, `cache_policy_from_headers`, and
   `HttpCache`. Header-parsing tests are plain `#[test]` (no gpui context);
   the `304`/`swr`/`no-store` integration tests use a `mockito` server on tokio.
4. **Wire gpui-app** to both per the integration plan. Replace raw `reqwest`
   calls in `query_playground`. Add `SqlitePersister`.

---

## Test Strategy

Per `gpui-test`: tests that touch `QueryClient` (a `Global`) need
`#[gpui::test]` + `TestAppContext`; pure logic tests (header parsing, debounce
math) are plain `#[test]`.

| Crate                 | Unit tests                                        | Integration tests                                  | GPUI ctx? |
| --------------------- | ------------------------------------------------- | -------------------------------------------------- | --------- |
| `gpui-query` (core 1) | `Fetched` policy override, `None` keeps policy    | `use_query_with_policy` end-to-end                 | Yes       |
| `gpui-query` (core 3) | debounce coalescing, `max_age` filtering, version | hydrate → persist → hydrate round-trip             | Yes       |
| `gpui-query-http`     | header parsing edge cases                         | `mockito` server: 200/304/swr/`no-store`           | No        |
| `gpui-query-persist`  | `FilePersister` round-trip, concurrent saves      | large snapshot, schema migration                   | No        |
| `gpui-app`            | —                                                 | `query_playground` section card asserts real flows | Yes       |

Cross-crate integration test (lives in `gpui-app/tests/` since only the app wires
both crates together): HTTP fetch → `CacheMeta` stored in `PersistedEntry::meta`
→ app shutdown → app restart → `hydrate` → next fetch sends `If-None-Match` →
server returns `304` → cached body served, no full refetch. This one test
exercises the entire design end-to-end.

---

## Open Questions

1. **Fetcher-returns-policy shape** — Option A (`Fetched<T, E>` wrapper) vs
   Option B (injected `QueryContext` callback)? Default: A, for explicitness and
   serializability. See [Required Core Change 1](#1-fetcher-returned-cachepolicy-for-http-server-wins).
2. **Bucket-mutation observation** — Option A (`observe_global`, coarse) vs
   Option B (typed dirty signal, precise)? Default: A for v1, revisit if
   profiling shows excessive firing. See [Required Core Change 2](#2-bucket-mutation-observation-for-persistence-subscription).
3. **`PersistedEntry::value` type** — `serde_json::Value` (flexible, slower) vs
   generic `T: Serialize + DeserializeOwned` (typed, faster, more bounds).
   Default: `serde_json::Value` for the wire format; typed de/serialization
   happens at the `Persister` adapter boundary, not in core.
4. **Per-key persistence policy** — `PersistOptions::filter` is global. If apps
   need per-key TTLs or per-key serializers, extend `QueryKeyFilter` or add a
   `PersistRule` enum. Defer until a concrete need exists.
5. **`Persister` object safety** — `impl Future + Send` return makes the trait
   not object-safe (`Box<dyn Persister>` won't compile). Fine for generics; if
   runtime dispatch is ever needed (e.g. dev vs prod persister swap), switch to
   `Pin<Box<dyn Future<...> + Send>>`. Defer.
6. **Encryption** — should `Persister` support encrypted backends, or is that the
   adapter's job? Recommendation: adapter's job. Core hands the adapter a
   `PersistSnapshot`; the adapter encrypts however it wants. Keeps a
   `SecureStoragePersister` in `gpui-app` trivial.
7. **DevTools surface** — `QueryClient::diagnostics()` already exists
   (`client/devtools.rs`). Add a `persist` section showing last save, last
   hydrate, persisted key count, and per-key age. Wire into the existing Query
   DevTools page.

