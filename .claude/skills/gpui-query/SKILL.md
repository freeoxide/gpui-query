---
name: gpui-query
description: Use when working IN the gpui-query codebase — adding or fixing query, mutation, caching, retry, persistence, HTTP-cache, or observer behavior; touching the core/client/hook layers or the http/persist satellite crates; or building, testing, documenting, or releasing the crates. Do not use for general GPUI app development; use it when the change is to this library itself.
---

## What it is

gpui-query is async state management for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui), inspired by TanStack Query v5. You write a fetcher; the library caches, retries, deduplicates, invalidates, garbage-collects, and cooperatively cancels. Main crate `gpui-query` (v0.2.0); two satellites add HTTP cache-header handling (`gpui-query-http`) and a disk persistence adapter (`gpui-query-persist`).

## Architecture

Three layers in the main crate, each a Cargo feature, strictly additive; the public API is glob re-exported at the crate root so users write `gpui_query::use_query`.

- **`core`** (`feature = "core"`) — serde-only state machine. `QueryResource<T,E>`, `MutationResource<V,T,E>`, `InfiniteQueryResource<T,E>`, `CachePolicy`, `RetryPolicy`, `QueryKey`, `QuerySignal`, two-phase completion types. No GPUI dep — usable anywhere.
- **`client`** (`feature = "client"`, the default) — `QueryClient`, a GPUI `Global` holding type-erased, type-partitioned buckets (`AHashMap<TypeId, Box<dyn Erased*>>`). GC, bulk invalidate/cancel/reset/remove, observers, `PreparedFetch`, devtools diagnostics.
- **`hook`** (`feature = "hook"`) — `use_query` / `use_mutation` / `use_infinite_query` / `use_query_select`, returning `(Entity<Resource>, Subscription)`.
- **`persist`** (`feature = "persist"`) — async `Persister` trait, `QueryClient::persist_with` debounced driver, free `hydrate()` fn, typed serializer registries.

`lib.rs` exposes each layer as a `pub mod` plus a gated glob re-export: `pub use core::*;` / `pub use client::*;` / `pub use hook::*;` (the `client` glob carries `#[allow(ambiguous_glob_reexports)]` because `current_time_ms` is defined identically in both `client` and `hook`).

Satellites: `gpui-query-http` depends on core-only (no GPUI); `gpui-query-persist` hard-depends on `persist+client+hook`.

## Core state machine

`QueryStatus` (derives serde; `Idle` default): `Idle`, `LoadingEmpty`, `LoadingWithData`, `Success`, `Failure`, `Cancelled`. Methods: `label()`, `is_loading()`, `is_pending()`.

`CachePolicy` (derives serde; default `Ttl { ttl_ms: 60_000 }`):

| Variant | Behavior | Freshness |
|---|---|---|
| `NoCache` | Always fetches. | `should_short_circuit_cache` = false |
| `Ttl { ttl_ms }` | Serves fresh within TTL. | `is_cache_fresh` when `age <= ttl_ms` (INCLUSIVE boundary, unlike HTTP max-age) |
| `StaleWhileRevalidate { ttl_ms, stale_ms }` | Stale window `[ttl, ttl+stale]` serves stale + background revalidate. | `should_serve_stale_and_revalidate` in that window |

`QueryKey`: hierarchical key backed by `Arc<[Arc<str>]>` (cheap clone). Serde-flexible — deserializes from a JSON array of strings OR a bare string. `starts_with` is segment-wise prefix (used by `QueryKeyFilter::Prefix`). `QueryKey::new` PANICS on an empty iterator — guard emptiness at the call site.

`RetryPolicy`: all fields public (`max_retries`, `retry_delay_ms`, `exponential_backoff`, `max_retry_delay_ms`). Default = 3 retries, exponential backoff, 1s base, 30s configured cap, hard 1-hour ceiling. `delay_for_attempt` = `retry_delay_ms * 2^attempt` (shift clamped to 62, saturating mul). `QueryResource::new` starts with `RetryPolicy::no_retries()`; the hook layer installs the real policy.

## Two-phase completion

`begin_request[_with_id]` → `QueryBeginResult` (`Started` / `CacheHit` / `StaleCacheHit` / `IgnoredWhileLoading`) + `RequestId`. Caller fetches async, then `accept_current_request(id)` returns `Option<RequestGuard>` — `Some` only if `id` is still active, else `None` (stale result silently discarded). `complete_*(guard, ...)` consume the single-use `RequestGuard` by value. This is the authoritative stale-write guard: cancelled async work can never overwrite newer state.

`RequestId` = `(NonZero scope, sequence)`; a per-resource `RequestSequencer` mints monotonic ids (scope advances on u64::MAX sequence). The `QueryClient` bucket keeps a co-located sequencer so ids stay unique across a resource's lifetime.

## Hooks

```rust
pub fn use_query<T, E, C, F, Fut>(
    options: impl Into<QueryOptions>,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    F: Fn(QuerySignal) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static;
```

```rust
pub fn use_mutation<V, T, E, C>(
    options: impl Into<MutationOptions>,
    cx: &mut Context<C>,
) -> (Entity<MutationResource<V, T, E>>, Subscription);   // use_mutation((), cx) works

pub fn use_infinite_query<T, E, C, FNext, Fut>(
    options: InfiniteQueryOptions,
    fetch_next: FNext,
    cx: &mut Context<C>,
) -> (Entity<InfiniteQueryResource<T, E>>, Subscription)
where
    FNext: Fn(Option<&T>) -> Fut + 'static,
    Fut: Future<Output = Result<(T, bool), E>>;   // (page, has_more)
```

Every hook returns `(Entity<Resource>, Subscription)` — both must be stored; dropping the `Subscription` kills the observation. (`use_query_select` returns a 3-tuple: mapped entity, source entity, two subscriptions.)

Fetcher + signal contract: the fetcher receives a `QuerySignal` (shared `Arc<AtomicBool>`; cancelling any clone cancels all). Poll `signal.is_cancelled()` to abort early, but write rejection is owned by `accept_current_request` — do NOT rely on an `is_cancelled()` check after the fetch returns (TOCTOU). `use_query_with_policy`'s fetcher returns `Fetched<T>` so a server-derived `CachePolicy` overrides the resource's on success (`Fetched::with_policy`, "server wins").

Minimal example:

```rust
use gpui_query::{use_query, QueryClient};

cx.set_global(QueryClient::new());

let (entity, _sub) = use_query(
    "users",
    |signal| async move {
        if signal.is_cancelled() { return Err(MyError::Cancelled); }
        Ok::<Vec<User>, MyError>(fetch_users().await?)
    },
    cx,
);
```

Plain-query fetch tasks are `.detach()`ed (cooperative signal + `accept_current_request` prevent stale writes; the task self-terminates on entity drop). Mutations and infinite queries store the task via `set_current_task` so replacement/unmount HARD-aborts the prior task.

## Observers

A single generic `Observer<R: ObservableResource>` with aliases `QueryObserver` / `InfiniteQueryObserver` / `MutationObserver`. It holds a `Cell<Option<Status>>` and calls `cx.notify()` ONLY when `observable_status()` changes — `increment_retry()` / `prepare_retry()` (stay `Loading`) and `set_current_task` do NOT re-render. Attach with `Observer::new(&entity).with_config(cfg).observe(cx) -> Option<Subscription>`.

## Persistence

Two tiers, both `persist`-gated, both defined in the MAIN crate (`gpui-query-persist` only ships the `FilePersister` adapter):

- Legacy metadata-only: `QueryPersister` trait (sync, object-safe, `Vec<DehydratedEntry>`), `QueryClient::dehydrate` / `hydrate` (method — a NO-OP stub) / `persist` / `restore`. `DehydratedEntry` carries key + type_id + kind only — NO data field.
- Value-carrying (use this): `Persister` trait (async, NON-object-safe — returns `impl Future`; monomorphized via `persist_with<P: Persister>`), `PersistSnapshot` / `PersistedEntry` / `PersistError`. `QueryClient::persist_with(persister, opts, cx) -> PersistHandle` debounces saves off a `CacheMutation` marker Global; the free `hydrate()` fn re-primes the cache via registered deserializers.

`FilePersister` (in `gpui-query-persist`) is an atomic, durable disk adapter: sibling `NamedTempFile` → fsync → macOS `F_FULLFSYNC` → rename over target → POSIX parent-dir fsync. Tolerant load: missing → empty snapshot, corrupt → logged + empty, version mismatch → typed `PersistError::VersionMismatch`. `PersistFormat::Json` (default) or `::Bincode`. It implements `Persister`, NOT `QueryPersister`.

Register typed (de)serializers via `register_serializer::<T,E>(fn(&T)->JsonValue)` / `register_deserializer::<T,E>(fn(&JsonValue)->Option<T>)`. `hydrate()` offers EVERY entry to EVERY deserializer (O(n×m)) — keep deserializers STRICT (return `None` for foreign shapes).

## HTTP cache

`gpui-query-http` derives a `CachePolicy` from RFC 9111 `Cache-Control` ("server wins") and layers an in-memory URL-keyed `HttpCache<B: HttpBackend>` for cheap 304 revalidations.

- `cache_policy_from_headers(headers: &HeaderMap) -> Result<CachePolicy, ParseError>` — `no-store`/`no-cache` short-circuits to `NoCache`; `s-maxage` overrides `max-age`; `stale-while-revalidate` → SWR. Absent/empty header → `Ok(NoCache)` (no Expires heuristic).
- `HttpCache::fetch(url)` short-circuits on fresh entries; otherwise a conditional GET carries `If-None-Match` (ETag) / `If-Modified-Since`; a 304 re-serves the cached body, a 200 parses headers and stores fresh. Only 200s are stored.
- Hand the result to `Fetched::with_policy(data, policy)` inside a `use_query_with_policy` fetcher.
- `ReqwestBackend` is behind the OFF-by-default `reqwest` feature (rustls-tls only; no cookies/compression). `HttpBackend` is non-object-safe (static dispatch via `HttpCache<B>`); guards are never held across an `.await`.

## Common tasks

- Add a feature flag: add to `crates/gpui-query/Cargo.toml` `[features]`, gate the module in `lib.rs` with `#[cfg(feature = "...")]` + `#[cfg_attr(docsrs, doc(cfg(feature=...)))]`, glob re-export at the crate root.
- Add a test: tests are inline under `crates/gpui-query/src/tests/` (declared in `src/tests/mod.rs`), feature-gated per module. Match the gate to the layer under test (`core_*` = ungated, `integration_*` = `#[cfg(feature = "client")]` / `hook`). Run: `cargo test --features "<layer>"` or `just test` for everything.
- Update docs / regenerate llms.txt: edit `web/src/content/docs/docs/**` (Astro + Starlight, MDX), then `just web-build` — it regenerates `llms.txt` / `llms-full.txt` via `web/scripts/generate-llms-txt.ts`. Never hand-edit those or `web/dist/**`.
- Cut a release: add a `## [x.y.z] - YYYY-MM-DD` section to `CHANGELOG.md`, then `just release x.y.z` (commits + pushes; CI tags, publishes, deploys).

## Gotchas

- WeakEntity retention: async tasks capture `entity.downgrade()`. If the owning component unmounts mid-fetch, the result is silently discarded (`weak.upgrade()` = `None`). Mutation `on_settled(None, None)` still fires on drop as a safety net.
- Notify only on status changes: `Observer` calls `cx.notify()` only when `observable_status()` changes. `increment_retry` / `prepare_retry` / `set_current_task` do NOT re-render.
- Signal cancellation on LatestWins replacement: `begin_loading` cancels the OLD signal before installing a fresh one, so a superseded fetcher observes `is_cancelled()`. The authoritative stale-write guard is `accept_current_request` returning `None`.
- Inclusive TTL boundary: `age <= ttl_ms` is fresh (opposite of HTTP `max-age`). Watch for off-by-one at the boundary.
- Bounded `max_pages` default: `InfiniteQueryResource` caps at `Some(50)` pages. `FetchDirection::ForwardOnly` (default) starts `has_next_page=true` as an assumption — use `new_bidirectional()` if the fetcher's `has_more` should drive fetching.
- Mutation GC actually runs: `gc_time_ms` default is 300_000 (explicit `Default` impl; the derived one produced 0, which DISABLED GC). `with_gc_time(0)` still disables GC; values < 1000 are clamped.
- AHashMap cache keys: `QueryClient` buckets use `AHashMap<TypeId, ...>` (~2x faster, trusted keys). `SerializerRegistry` is keyed by `TypeId::of::<T>()` ALONE, not `(T,E)` — registering the same `T` under two `E`s overwrites.
- macOS Metal toolchain: building `client`/`hook`/`persist` (anything pulling `gpui`) fails without the Metal Toolchain. Run `xcodebuild -downloadComponent MetalToolchain` once. Core-only builds need nothing.
- Cache-mutation dirty signal: `cx.default_global::<CacheMutation>()` is bumped at every cache-mutation site (`set_query_data`, `PreparedFetch` completions, hook completions); `persist_with`'s `observe_global` reacts to it.
- `QueryError` messages are stored verbatim and surface in Display/Debug/serde. Use the typed constructors (`response` / `transport` / `cancelled`) to preserve category, and `sanitized()` (redacts tokens/paths/emails, truncates to 512 bytes) before constructing from untrusted server data.

## Pointers

- Docs site: https://gpui-query.freeoxide.com/docs/ — guides under `/docs/guides/` (`caching`, `retry`, `query-keys`, `persistence`, `http-caching`, `select-pattern`, `error-handling`); API under `/docs/api/`.
- `llms.txt` / `llms-full.txt` are generated into the site root at build time (not committed source) — read them from a built `web/dist/client/` if you need the full rendered reference.
- Key source paths:
  - Main crate: `crates/gpui-query/src/{core,client,hook}/` (re-exported by `lib.rs`).
  - `QueryResource` lifecycle: `src/core/resource/{lifecycle,completion,cache}.rs`.
  - `QueryClient`: `src/client/{mod,lifecycle,bucket/}.rs`; persistence: `src/client/persist.rs`.
  - Hooks: `src/hook/{query_hooks,mutation_hooks/,use_infinite_query/}.rs`.
  - HTTP: `crates/gpui-query-http/src/{cache,backend,reqwest_backend}.rs`.
  - Disk adapter: `crates/gpui-query-persist/src/lib.rs`.
- Tests: `crates/gpui-query/src/tests/` (feature-gated per module).
