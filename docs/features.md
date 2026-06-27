# gpui-query Extensions — Design Plan

**HTTP cache helpers and persistence enrichment for the `gpui-query` crate.**

This is the `gpui-query` crate's own design doc for two *proposed* companion
crates — `gpui-query-http` (HTTP cache semantics) and `gpui-query-persist`
(persistence) — grounded in the **current** core API. The two are at very
different maturity:

- **HTTP cache helpers** are wholly proposed. Nothing like them ships today.
- **Persistence** is **not** greenfield. The crate already ships a synchronous,
  metadata-only persistence skeleton in its `client` layer (the `QueryPersister`
  trait, the `DehydratedEntry` / `DehydratedState` snapshot types, and the
  `QueryClient::dehydrate` / `hydrate` / `persist` / `restore` methods). The
  `gpui-query-persist` crate proposed below is an **enrichment** of that skeleton
  toward a richer, async, value-carrying API — and, optionally, an extraction of
  reference adapters into a companion crate. Nothing on the persistence side is
  "to be built from zero."

Throughout, `gpui-app` is **one example external consumer** of this crate — it is
not part of this repository, and this crate makes no claim of co-owning it. Every
`gpui-app` path in this document is illustrative of where any generic consuming
app would wire things in, never a path inside this crate.

> **Status:** Verified against the `gpui-query` crate source under
> `crates/gpui-query/src`. The crate exposes features `core`, `client`, and `hook`
> (see `crates/gpui-query/src/lib.rs` and `crates/gpui-query/Cargo.toml`).
> Persistence is **optional** — it exists only for offline data caching — so this
> plan puts the entire persistence surface behind a new opt-in `persist` feature;
> **today** it still ships unconditionally in the `client` layer, and
> feature-gating that shipped skeleton is itself part of the proposal below.
> The hooks use a signal-always fetcher closure: `use_query` in
> `hook/query_hooks.rs` takes `F: Fn(QuerySignal) -> Fut + Send + 'static` and
> returns a `(Entity<QueryResource<T, E>>, Subscription)` tuple. `QueryClient` is
> a `Global` (`impl Global for QueryClient` in `client/mod.rs`) with
> `set_query_data`, `resource`, `resource_with_policies`, and `diagnostics`. The
> `QueryObserver` / `Observer` machinery in `client/observer.rs` observes
> individual resource entities via `Observer::observe`; there is **no** `Resource`
> trait (the relevant bound is `ObservableResource`). Everything marked *Proposed*
> below — the `gpui-query-http` and `gpui-query-persist` crates, a **disk-based
> cross-platform `FilePersister`**, an async `Persister`, the `persist` feature
> gate, `Fetched<T, E>`, and `use_query_with_policy` — does not exist yet.

---

## Table of Contents

1. [Guiding Principles](#guiding-principles)
2. [Extension Points](#extension-points)
3. [Crate Layout](#crate-layout)
4. [Shared Types](#shared-types)
5. [Required Core Changes](#required-core-changes)
6. [1. HTTP Cache Helpers — `gpui-query-http` *(proposed)*](#1-http-cache-helpers--gpui-query-http)
7. [2. Persistence — `gpui-query-persist` *(proposed enrichment of the shipped skeleton)*](#2-persistence--gpui-query-persist)
8. [Integration Plan for a Consuming App](#integration-plan-for-a-consuming-app)
9. [Migration Order](#migration-order)
10. [Test Strategy](#test-strategy)
11. [Open Questions](#open-questions)

---

## Guiding Principles

1. **Core stays agnostic.** `gpui-query` must not import `reqwest`, `http`, or any
   storage backend. Users with gRPC, IPC, or offline-first transports must not pay
   for HTTP semantics they don't use.
2. **Optional concerns are feature-gated.** Persistence is only for offline data
   caching — the uncommon case — so the whole persistence surface (the shipped
   `QueryPersister` skeleton and any disk adapter) sits behind an opt-in `persist`
   feature. Apps that don't persist pay nothing: no persistence code compiled in,
   and no disk-IO dependencies. HTTP cache helpers get the same treatment by
   living in their own crate.
3. **One extension point per concern.** HTTP cache and persistence have different
   scopes, triggers, and shapes. Each gets its own minimal surface rather than a
   unified abstraction that fits neither well.
4. **The crate owns its extension points; consumers own their adapters.**
   `gpui-query` is the authority on its cache and query contract, so the extension
   points (the `QueryPersister` trait, the `dehydrate` / `hydrate` / `persist` /
   `restore` methods, and the proposed HTTP helper boundary) belong in — or
   directly adjacent to — this crate. There is no second co-owned crate in this
   repo: the root `Cargo.toml` workspace lists only `crates/gpui-query`. A
   consuming app (for example, a hypothetical `gpui-app`) is an *external* user and
   ships only its own backend-specific adapters. The reason to centralize the
   trait and reference adapters here is single-sided design authority, not
   coordination with another owned crate.
5. **Reference adapters ship with the crate; app-specific adapters ship with the
   app.** A reference `FilePersister` already appears as the canonical example in
   the `QueryPersister` trait docs in `client/erased.rs`. The proposed
   `gpui-query-persist` crate ships a **real, disk-based, cross-platform**
   `FilePersister` (see [2. Persistence](#2-persistence--gpui-query-persist)). A
   `SqlitePersister` (which depends on a consumer's own storage layout and
   migrations) always stays in that consumer.
6. **Disk persistence is cross-platform and robust.** The reference `FilePersister`
   resolves an OS-appropriate data directory (Linux / macOS / Windows), writes
   atomically (temp file + replace — with the per-OS atomicity/durability nuances
   in [Disk-based persistence](#disk-based-persistence)), tolerates corruption
   without panicking, and serializes portably (JSON or bincode). It must be robust
   on Linux, macOS, and Windows — surviving crash-mid-write, corrupt files,
   permission errors, and schema drift.
7. **No speculative abstractions.** Don't build a generic plugin/middleware trait
   to wrap these concerns. Retry already lives in core; HTTP is a wrapper, not a
   layer; persistence is an existing synchronous trait plus proposed enrichment. If
   a new need appears later, add a new small extension point then.
8. **Follow `rust-best-practices`.** Typed errors via `thiserror` in any proposed
   companion crates; `Arc<T>` for shared resources; `impl Future<...> + Send` on
   trait methods to guarantee spawnable futures (preferred over `async fn` in
   traits, which does not guarantee `Send`); `#![deny(missing_docs)]` on every
   companion crate; no `unwrap` / `expect` outside tests. These standards govern
   the proposed companion crates and any enrichment of the shipped in-tree
   skeleton alike.

---

## Extension Points

Two concerns, two shapes. Nothing in common, so no shared abstraction.

| Concern     | Status today                                                                                                                                                                | Scope        | When               | Proposed shape                              | Proposed home crate     |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------ | ------------------------------------------- | ----------------------- |
| HTTP cache  | Not yet implemented                                                                                                                                                         | per-fetch    | load time          | function + wrapper                          | `gpui-query-http`       |
| Persistence | **Shipped skeleton**: sync `QueryPersister` trait (`client/erased.rs`) + `dehydrate` / `hydrate` / `persist` / `restore` (`client/lifecycle.rs`); metadata-only `DehydratedEntry` (no value field) | whole bucket | mutation + startup | async trait + subscribe (proposed enrichment) | `gpui-query-persist` (proposed) |

- **HTTP cache** is a wrapper around `reqwest` — there is nothing to swap. You
  either talk HTTP or you don't. A struct with a `fetch` method is the correct
  shape, not a trait or middleware. *(Proposed; nothing ships today.)*
- **Persistence** already has a synchronous, metadata-only skeleton:
  `QueryPersister::load() -> Vec<DehydratedEntry>` and
  `save(Vec<DehydratedEntry>)` in `client/erased.rs`, with
  `QueryClient::persist(&self, persister: &dyn QueryPersister, cx)`, the associated
  `QueryClient::restore(persister) -> Vec<DehydratedEntry>`,
  `QueryClient::dehydrate(cx) -> DehydratedState`, and a documented no-op
  `QueryClient::hydrate`. The shipped `DehydratedEntry` carries only `key`,
  `type_id`, and `kind` (`"query"` / `"infinite"` / `"mutation"`) — **no data
  field** (it was deliberately removed: the former `data_json: Option<String>`
  placeholder, always `None`, was dropped as dead weight in audit fix #L14). The
  proposed enrichment adds: an async persister, a value/meta-carrying entry, a
  `persist_with` debounce/filter subscription, schema versioning, and (optionally)
  extraction into a `gpui-query-persist` crate. Note on the subscription path: a
  `persist_with` built on `cx.observe_global::<QueryClient>()` **does** fire for
  free on any mutation routed through `cx.update_global::<QueryClient, _>` — and
  the crate already does exactly that at every fetch / retry / mutation / infinite-
  query completion site (`hook/query_hooks.rs`, `hook/fetch_retry.rs`,
  `hook/mutation_hooks/hooks.rs`, `hook/use_infinite_query/*`), because GPUI's
  `update_global` pushes a `NotifyGlobalObservers` effect. The one gap is
  `QueryClient::set_query_data` (`client/mod.rs`), which mutates a resource
  **entity** via `entity.update(...)` with no global notify — so seed/prime writes
  would need an explicit global notify (or routing through `update_global`) to wake
  a `persist_with` observer. So `persist_with` is mostly reuse, plus one small
  notify hook for the `set_query_data` path.

Each proposed addition is ~50 lines of core surface, all behind the proposed
opt-in `persist` feature (persistence is offline-data only). No plugin system, no
middleware chain, no lifecycle hook bag.

---

## Crate Layout

This is the **`gpui-query` crate's own** design doc. `gpui-app` (referenced
elsewhere in this document) is one example *external* consumer, never a member of
this repository.

### Current workspace (as it exists today)

```
crates/
  gpui-query/        # the crate this doc belongs to: core + client + hook layers
  gpui-query-legacy/ # legacy v1 crate (excluded from the workspace, kept for reference)
  crates-og.zip      # stray archive, NOT a crate (junk to be removed)
```

Per the root `Cargo.toml`, the only workspace member is `crates/gpui-query`
(`members = ["crates/gpui-query"]`); `crates/gpui-query-legacy` is present in the
tree but listed under `exclude`. There is **no** `gpui-query-http` crate, **no**
`gpui-query-persist` crate, and **no** `gpui-app` crate in this repo.

### Feature flags (real today, plus the proposed `persist` gate)

```toml
# crates/gpui-query/Cargo.toml
[features]
default = ["client"]
core   = []
client = ["core", "dep:gpui"]
hook   = ["client"]
# PROPOSED — gates the optional persistence surface (offline data only).
persist = ["client"]
```

The crate exposes three layers — `core`, `client`, `hook` — gated by features of
the same names (see `crates/gpui-query/src/lib.rs`). `client` is on by default.

Persistence is **optional** (offline-data only), so the design gates it behind a
new opt-in `persist` feature. **Today** the persistence skeleton — the
`QueryPersister` trait (`client/erased.rs`), `DehydratedEntry` / `DehydratedState`
(`client/devtools.rs`), and the `QueryClient::dehydrate` / `hydrate` / `persist` /
`restore` methods (`client/lifecycle.rs`) — ships **unconditionally** as part of
`client`; feature-gating that shipped skeleton is part of the proposal.

The gate is feasible but **not** zero-coupling — three prerequisites must land
first: (1) **extract `current_time_ms`** out of `client/erased.rs` (it is defined
in the same module as `QueryPersister` and re-exported together at
`client/mod.rs`, but it is consumed by non-persistence code — gc, diagnostics,
prepared-fetch, the whole hook layer — so the module cannot be gated as-is; only
the `QueryPersister` symbol can move once the helper is relocated); (2) decide
what to do with `collect_key_status_into` — a required method on the always-
compiled `ErasedBucket` / `ErasedInfiniteBucket` / `ErasedMutationBucket` traits
whose only caller is `dehydrate` (gate the method + its three impls together, or
leave the dead method when `persist` is off); (3) cfg-gate the itemized
`pub use devtools::{…}` and `pub use erased::{…}` lists in `client/mod.rs` in
lockstep with the gated symbols. Once those are done, `QueryClient::diagnostics()`
is unaffected (it returns `ClientDiagnostic` and never touches the dehydrate
types). Dependency note: `serde` is a non-optional core dep (`CachePolicy` derives
`Serialize` / `Deserialize` from it), but **`serde_json` is currently
`[dev-dependencies]`-only** — every `serde_json::` use in the crate is behind
`#[cfg(test)]`. Because the proposed `PersistedEntry.value` / `Fetched.meta` are
typed `serde_json::Value`, the value-carrying surface promotes `serde_json` to a
real (gated) dep: add it under `persist`, or keep all JSON serialization inside
the `gpui-query-persist` companion (current plan: `PersistFormat::Json` lives
there). The disk-IO crates (`dirs` / `tempfile` / `bincode`) likewise live in the
companion, pulled only by apps that actually persist.

### Proposed (not yet implemented)

The richer companion crates this document goes on to describe do not exist yet.
They are sketched for discussion only:

```
crates/
  gpui-query/          # EXISTS today (see above)
  gpui-query-http/     # PROPOSED: CacheMeta, cache_policy_from_headers, HttpCache
  gpui-query-persist/  # PROPOSED: FilePersister, async adapter glue on top of the shipped skeleton
```

A `SqlitePersister`, `SecureStoragePersister`, or any other app-specific adapter
would live in the **consuming app** (e.g. `gpui-app`), not in this repo — it
depends on the app's own storage layout and migrations.

### What persistence looks like today vs. what this doc proposes

The proposed `gpui-query-persist` crate builds on a skeleton that **already
ships** in the `client` layer. Today's reality, and the real gaps:

- **`QueryPersister` trait** — already shipped (`client/erased.rs`).
  `pub trait QueryPersister: Send + Sync`, object-safe, used as
  `&dyn QueryPersister`. Methods are **synchronous**:
  - `fn load(&self) -> Vec<DehydratedEntry>`
  - `fn save(&self, entries: Vec<DehydratedEntry>)`
- **`DehydratedEntry`** — already shipped (`client/devtools.rs`). Metadata-only:
  `{ key: String, type_id: std::any::TypeId, kind: &'static str }`. `kind` is
  `"query"`, `"infinite"`, or `"mutation"`. It is `#[derive(Clone, Debug)]` —
  **not** `Serialize` / `Deserialize`. There is deliberately **no data/value/json
  field**: it was removed (audit fix #L14) as dead weight that was always `None`.
  The trait doc-comment's claim that "entries are serialized as JSON strings" is
  misleading — `TypeId` is not directly serializable, so any serialization is
  entirely the persister adapter's problem.
- **`DehydratedState`** — already shipped (`client/devtools.rs`):
  `{ entries: Vec<DehydratedEntry> }`, `#[derive(Clone, Debug, Default)]`.
- **`QueryClient::dehydrate(&self, cx: &App) -> DehydratedState`**
  (`client/lifecycle.rs`) — emits only `Success`-status entries; metadata only
  (key + `type_id` + kind). No data payload.
- **`QueryClient::hydrate(&mut self, _state, _cx)`** (`client/lifecycle.rs`) — body
  is a **no-op hook point**; documented as a place for typed hydration. Callers
  restore typed data themselves via `QueryClient::set_query_data::<T, E>(...)`
  (`client/mod.rs`).
- **`QueryClient::persist(&self, persister: &dyn QueryPersister, cx: &App)`**
  (`client/lifecycle.rs`) — calls `dehydrate()` then
  `persister.save(state.entries)`.
- **`QueryClient::restore(persister: &dyn QueryPersister) -> Vec<DehydratedEntry>`**
  (`client/lifecycle.rs`) — associated fn (no `&self`); returns `persister.load()`.

The **real gaps** this doc is actually proposing to close, on top of that skeleton,
are therefore: (1) an **async** `Persister` surface (today it is synchronous and
blocks the foreground thread), (2) a `PersistedEntry { value, cached_at,
cache_policy, meta }` shape that actually carries the cached value (today the
entry is metadata-only), (3) `persist_with` (a debounced, filtered subscription) —
today there is only the one-shot `persist()`, (4) a real (non-no-op) `hydrate`
that primes via `set_query_data`, (5) `debounce` / `max_age` / `filter` policy,
and (6) snapshot `version` migration. All of these are future work.

---

## Shared Types

### Shipped today (in the `client` layer of `gpui-query`)

| Type                | Lives in                            | Notes                                                                                                                       |
| ------------------- | ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `CachePolicy`       | `gpui-query` (`core/policy.rs`)     | `Clone + Copy + Debug + PartialEq + Eq + Serialize + Deserialize`; shared by core, and by any future http/persist crates.   |
| `QueryKey`          | `gpui-query` (`core/key.rs`)        | Cache identity (`pub struct QueryKey(Arc<[Arc<str>]>`)); shared by core and any persist crate.                              |
| `QueryKeyFilter<'a>`| `gpui-query` (`core/key_filter.rs`) | `enum { Exact(&'a QueryKey), Prefix(&'a QueryKey), All }`; borrows a `&'a QueryKey` and is **not** `Serialize` — see note in [2. Persistence](#2-persistence--gpui-query-persist) before reusing it in `PersistOptions`. |
| `QueryPersister`    | `gpui-query` (`client/erased.rs`)   | `Send + Sync`, object-safe; synchronous `load()` / `save()` over `Vec<DehydratedEntry>`. The real extension point today.   |
| `DehydratedEntry`   | `gpui-query` (`client/devtools.rs`) | Metadata-only: `key` + `TypeId` + `kind`. **Not serde.** No value field. The real cross-boundary persistence value.         |
| `DehydratedState`   | `gpui-query` (`client/devtools.rs`) | `{ entries: Vec<DehydratedEntry> }`; produced by `QueryClient::dehydrate`, consumed by `hydrate` (a no-op hook today).                           |

### Proposed (not yet implemented — see [2. Persistence](#2-persistence--gpui-query-persist))

| Type                | Would live in                       | Notes                                                                                                              |
| ------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Persister` (async) | `gpui-query` (client/persist, future) | Async successor to the synchronous `QueryPersister`; `load` / `save` returning `impl Future + Send` (**not** object-safe). |
| `PersistedEntry`    | `gpui-query` (future)               | Carries `value: serde_json::Value`, `cached_at`, `cache_policy`, `meta` — the typed payload `DehydratedEntry` lacks. |
| `PersistSnapshot`   | `gpui-query` (future)               | `{ entries: HashMap<QueryKey, PersistedEntry>, version: u32 }`.                                                    |
| `PersistOptions`    | `gpui-query` (future)               | `{ filter, max_age, debounce }` (+ optional `dehydrate_filter`, type TBD). Note: `filter` cannot reuse `QueryKeyFilter<'a>` directly — see [2. Persistence](#2-persistence--gpui-query-persist). |
| `CacheMeta`         | `gpui-query-http` (future)          | `serde::{Serialize, Deserialize}`; crosses into a future persist crate only as opaque `serde_json::Value`.         |
| `HttpCache`         | `gpui-query-http` (future)          | `reqwest` wrapper; nothing swaps, hence a struct not a trait.                                                       |
| `FilePersister`     | `gpui-query-persist` (future)       | Reference adapter.                                                                                                  |
| `SqlitePersister`   | consuming app (e.g. `gpui-app`)     | App-specific; depends on the app's storage layout. Lives outside this crate.                                       |

`CacheMeta` never crosses into core or a persist crate as a typed value — only as
`serde_json::Value`. Today the only typed values that cross a crate boundary are
`CachePolicy` and `DehydratedEntry`; `QueryPersister` is the trait that defines
the boundary. A future async `Persister`, `PersistedEntry { value }`,
`persist_with`, debounce/filter, and versioning are all Proposed — they would
extend the shipped skeleton, not replace it.

---

## Required Core Changes

These are **proposed additions and enhancements to `gpui-query` core** that the
two companion crates depend on. Where a capability already ships in a different
shape, the proposal is framed as an enhancement of that skeleton rather than a
fresh build. Persistence ships unconditionally in the `client` layer today; this
plan gates it behind the proposed `persist` feature (offline-data only — see
[Crate Layout](#crate-layout)).

### 1. Fetcher-returned `CachePolicy` (for HTTP "server wins")

**Problem.** The shipped primary hook is `use_query` in `hook/query_hooks.rs`. Its
fetcher is signal-always — `F: Fn(QuerySignal) -> Fut` — and returns only
`Result<T, E>`. (`use_query_unsignalled` is the backward-compatible `Fn() -> Fut`
variant.) `CachePolicy` is fixed at `use_query` / `fetch_query` call time (via
`QueryOptions` / the `cache_policy` argument). There is no way for a fetcher that
just read `Cache-Control: max-age=30` from a response to push that policy back
into the bucket — so HTTP semantics cannot override the caller's per-query policy.

**Status:** Neither `Fetched<T, E>` nor a `use_query_with_policy` hook exists
today. Both are proposed here.

**Fix — pick one (Open Question 1):**

- **Option A (typed wrapper):** introduce a proposed `Fetched<T, E>` result type:

  ```rust
  pub struct Fetched<T, E> {
      pub data: T,
      pub cache_policy: Option<CachePolicy>,  // None → keep caller's policy
      pub meta: Option<serde_json::Value>,    // proposed: CacheMeta round-trip via persist
      _error: PhantomData<E>,
  }
  ```

  A proposed `use_query_with_policy` would mirror the shipped `use_query` signature
  — same `(Entity<QueryResource<T, E>>, Subscription)` tuple return, same
  `QuerySignal`-accepting fetcher shape — except the fetcher returns
  `Result<Fetched<T, E>, E>` and the bucket applies `cache_policy` on success. The
  existing `Result<T, E>` `use_query` keeps working unchanged (additive,
  non-breaking).

- **Option B (callback):** keep the fetcher as `Result<T, E>`, but give the
  fetcher closure an injected handle to the resource whose
  `QueryResource::set_cache_policy` (`core/resource/accessors.rs:134` — **already
  shipped**; same for `InfiniteQueryResource`) it can call (plus a `set_meta`
  equivalent). Only the handle plumbing is net-new; the setter itself exists
  today. More flexible, more state-passing boilerplate.

**Recommendation:** Option A — explicit, serializable, composes cleanly with
persistence. **Caveat:** the `meta` field and its persistence coupling depend on
Core Change 3 landing a value-carrying persisted entry first. Today the shipped
snapshot type (`DehydratedEntry`) is metadata-only (key + `type_id` + kind) and
carries no `value` / `meta` field, so a `Fetched.meta` would have nowhere to
persist to until that gap is closed.

### 2. Whole-client mutation observation (for persistence subscription)

**Problem.** The shipped `Observer` in `client/observer.rs` (with
`QueryObserver<T, E>` / `InfiniteQueryObserver<T, E>` /
`MutationObserver<V, T, E>` aliases) observes a **single resource entity** and
calls `cx.notify()` only on status change. It does **not** observe whole-client
mutations. A persistence subscription needs to know when _any_ tracked entry
changes.

**Good news — most of the reuse works today.** `QueryClient` is a `Global`
(`impl Global for QueryClient {}` in `client/mod.rs`), and in GPUI mutating a
Global via `cx.update_global::<QueryClient, _>` **does** auto-fire
`cx.observe_global::<QueryClient>()` subscribers (the update pushes a
`NotifyGlobalObservers` effect). The crate already routes real mutations through
`update_global::<QueryClient, _>` at every fetch / retry / mutation / infinite-
query completion site (`hook/query_hooks.rs`, `hook/fetch_retry.rs`,
`hook/mutation_hooks/hooks.rs`, `hook/use_infinite_query/*`). So a `persist_with`
subscribing via `observe_global::<QueryClient>()` **already fires** on the common
mutation paths — no core change needed for those.

**The one gap:** `QueryClient::set_query_data` (`client/mod.rs`) mutates a
resource **entity** via `entity.update(...)` — that does *not* touch the
`QueryClient` global, so it would not wake a global observer. Seed/prime writes
(hydration, optimistic updates) therefore need an explicit global notify (or
re-routing through `update_global`) to be observed.

**Fix — pick one (Open Question 2):**

- **Option A (coarse global observer):** `persist_with` subscribes via
  `cx.observe_global::<QueryClient>()` — free for every mutation already going
  through `update_global` (the common case) — and add one small notify hook so
  `set_query_data` also wakes the observer. The `PersistOptions::filter` (an owned
  filter, see below) and `debounce` do the real selection work. This is almost
  pure reuse plus a one-line notify in `set_query_data`.
- **Option B (typed dirty signal):** add a `QueryClient` dirty signal (analogous
  to the proposed `notify_buckets_changed`) emitted from `set_query_data` / fetch
  completion, observed by `persist_with`. More precise, more core surface.

**Recommendation:** Option A for v1 — it is mostly free reuse (the bulk of
mutations already notify via `update_global`), with a trivial notify added to the
`set_query_data` path. Move to Option B only if profiling shows the coarse path
firing too often.

### 3. Async `Persister` + `persist_with` + real `hydrate` (enhancement of the shipped skeleton)

**What already ships (in the `client` layer, no feature flag):**

- `pub trait QueryPersister: Send + Sync` in `client/erased.rs` — **synchronous**:
  `fn load(&self) -> Vec<DehydratedEntry>` and
  `fn save(&self, entries: Vec<DehydratedEntry>)`. Object-safe; used as
  `&dyn QueryPersister`. A `FilePersister` doc-example ships in the same module.
- `DehydratedEntry { key: String, type_id: TypeId, kind: &'static str }` in
  `client/devtools.rs` — `#[derive(Clone, Debug)]`, **not**
  `Serialize` / `Deserialize`, and **no `value` / `data` / `json` field** (it was
  deliberately removed as dead weight, audit fix #L14). `kind ∈ {"query",
  "infinite", "mutation"}`. `DehydratedState` wraps `Vec<DehydratedEntry>`.
- `QueryClient::dehydrate(&self, cx) -> DehydratedState` in `client/lifecycle.rs`
  — Success-status entries only, metadata only (key + `type_id` + kind), no data
  payload.
- `QueryClient::hydrate(&mut self, _state, _cx)` in `client/lifecycle.rs` — a
  **no-op** body, documented as a hook point for typed hydration; callers restore
  typed data themselves.
- `QueryClient::persist(&self, persister: &dyn QueryPersister, cx)` — calls
  `dehydrate()` then `persister.save(state.entries)`.
- `QueryClient::restore(persister) -> Vec<DehydratedEntry>` — associated fn
  (no `&self`) returning `persister.load()`.

The crate's `QueryPersister` doc-comment claims "Entries are serialized as JSON
strings to avoid generic bounds," but `DehydratedEntry` holds a `std::any::TypeId`
which is not directly serializable — serialization is (and remains) the
persister-adapter's problem.

**The real gaps (proposed future work on top of this skeleton):**

1. **Async trait.** The shipped trait is synchronous. A proposed async `Persister`
   with `impl Future<...> + Send` return types (to be safely spawned on a tokio
   runtime) is **not** object-safe by default; it favors generics
   (`persist_with<P>`) over `Box<dyn>`. (Open Question 5 covers a `Pin<Box<...>>`
   fallback for runtime dispatch.) This is a breaking rework of the shipped trait,
   not an additive change.
2. **Data carriage.** There is no `value` / `meta` field anywhere in the shipped
   snapshot — only metadata. A proposed `PersistedEntry { value, cached_at,
   cache_policy, meta }` and a `value`-carrying dehydrate path would let
   `gpui-query-persist` store actual cached data (and `CacheMeta` as an opaque
   `serde_json::Value`, see [Shared Types](#shared-types)). This is the unblocker
   for the `Fetched.meta` field in Core Change 1.
3. **`persist_with`.** A subscription-style method that observes whole-client
   mutations (per Core Change 2, once the notify hook exists), debounces, and calls
   `save`. No such method ships today; the shipped `persist` is a one-shot call.
4. **Real `hydrate`.** A typed-hydration method that loads entries and primes each
   one via the existing `QueryClient::set_query_data::<T, E>` in `client/mod.rs`
   (creating the resource first with `QueryClient::resource::<T, E>` if needed).
   The shipped `hydrate` is a no-op, so this is net-new behavior built on the
   existing priming primitive — there is no separate `prime` method and none is
   needed.
5. **`PersistOptions` (filter / `max_age` / `debounce` / `dehydrate_filter`) and
   `PersistSnapshot` versioning.** None ship today; both are proposed. Note that
   `PersistOptions::filter` **cannot** reuse `QueryKeyFilter<'a>` directly — that
   type borrows a `&'a QueryKey` and is not `Serialize`, so an owned filter variant
   (or a separate owned filter type) is required; this is an open design detail,
   not a resolved one.

Detailed shape lives in [2. Persistence](#2-persistence--gpui-query-persist).
The richer API — and the gating of the currently-unconditional synchronous
skeleton — lands behind the proposed `persist` feature (see [Crate
Layout](#crate-layout)).

---

## 1. HTTP Cache Helpers — `gpui-query-http`

> **Status:** This crate is **proposed**, not yet present in the workspace. The
> API below is a design sketch for a new `crates/gpui-query-http` crate. (The
> workspace today has only `crates/gpui-query`; `crates/gpui-query-legacy` is
> excluded, and `gpui-app` is an external consumer, not a repo member.) The usage
> example targets the **real** `use_query` hook (signal-always, options-first,
> tuple return); it does **not** rely on any unimplemented core API except where
> explicitly flagged.

### Scope

Per-fetch concern. Parse HTTP cache headers, attach conditional request headers
on refetch, and on `304 Not Modified` return the cached body. This is a wrapper
around `reqwest`, not a core abstraction. The `http`, `reqwest`, and `bytes`
dependencies would belong to `gpui-query-http` only — `gpui-query` core ships with
**none** of them today (none appears in `crates/gpui-query/Cargo.toml` nor anywhere
under `crates/gpui-query/src`), consistent with Guiding Principle 1.

### Public API

```rust
// crates/gpui-query-http/src/lib.rs   (PROPOSED — file does not exist yet)

use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};

/// HTTP cache metadata extracted from a response. Serializable so a future
/// persistence layer (see Section 2) can store it alongside the body and
/// rehydrate a cold start with valid ETags, enabling cheap 304 refetches on the
/// first request after launch.
///
/// Timestamps use `SystemTime` (serde-supported, epoch-relative) — **never**
/// `std::time::Instant`, which has no serde impl and is meaningless across
/// process restarts. This matches the crate's `current_time_ms()` convention.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheMeta {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub stored_at: SystemTime,
    pub fresh_for: Duration,
    pub stale_for: Duration,
}

/// Parse Cache-Control / ETag / Last-Modified / Age.
/// Returns `Ok(policy)` derived from the headers, or a typed `ParseError`.
///
/// `policy` is a gpui-query `CachePolicy` (core/policy.rs), i.e. one of
/// `NoCache`, `Ttl { ttl_ms }`, or `StaleWhileRevalidate { ttl_ms, stale_ms }`
/// — these are the exact variants of the shipped `CachePolicy` enum, which is
/// `#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]`.
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

gpui-query uses a **fetcher closure**, not a `Resource` trait. The primary hook
is `use_query` (`hook/query_hooks.rs`): options-first (first param is
`impl Into<QueryOptions>`), **signal-always** (the fetcher bound is
`F: Fn(QuerySignal) -> Fut`, not `Fn() -> Fut`), and it returns a
`(Entity<QueryResource<T, E>>, Subscription)` tuple. `HttpCache` is called
_inside_ the fetcher:

```rust
// In any consuming app (e.g. gpui-app — an EXTERNAL consumer, not a repo member)
use gpui_query::{use_query, QueryKey, QueryOptions};
use gpui_query::core::{CachePolicy, RequestPolicy, QuerySignal};

#[derive(Clone, serde::Deserialize)]
struct User { /* ... */ }

let url: reqwest::Url = format!("https://api.example.com/users/{id}").parse()?;
let http = http_cache.clone(); // Arc<HttpCache>

// The per-query policy is set on the QueryOptions builder. Note that today
// this policy is FIXED at fetch start — see the note below on "server wins".
let options = QueryOptions::new(QueryKey::from(["users", &id.to_string()]))
    .cache_policy(CachePolicy::Ttl { ttl_ms: 60_000 })
    .request_policy(RequestPolicy::LatestWins);

let (entity, _subscription) = use_query(
    options,
    move |_signal: QuerySignal| async move {
        let (bytes, _policy, _meta) = http.fetch(&url).await?;
        let user: User = serde_json::from_slice(&bytes)?;
        Ok(user) // real fetcher returns Result<T, E>
    },
    cx,
);
```

> **Server-wins is not yet possible against shipped code.** The fetcher above
> discards the `_policy` and `_meta` returned by `http.fetch` because there is
> currently no path to push a server-derived `CachePolicy` back into the
> resource: the resource's policy is established at `begin_request` time
> (`begin_request` / `begin_request_inner` in `core/resource/lifecycle.rs` read
> the stored `cache_policy` / `request_policy`; the result is summarized by
> `QueryBeginResult` in `core/policy.rs`) and is not updated by fetcher output,
> and the fetcher returns only `Result<T, E>`.
>
> Feeding the server-returned `CachePolicy` (and `CacheMeta`) back into
> `QueryClient` requires **Required Core Change 1** — a `Fetched<T, E>` result
> type (Option A) or an injected `QueryContext` callback (Option B), plus a
> `*_with_policy` fetcher variant. Neither `Fetched` nor any such variant exists
> in the crate today. Until then, the caller-supplied `CachePolicy` on
> `QueryOptions` is authoritative, and `HttpCache`'s 304/ETag handling still works
> (it returns the cached body on 304) but the resource's TTL window is the
> caller's, not the server's.

### Design notes

- No middleware, no layer trait. There is nothing to swap — you either talk HTTP
  or you don't. A wrapper struct is the correct shape.
- `HttpCache` owns its own `HashMap` of metadata. It does **not** touch
  `QueryClient`'s bucket. The `CachePolicy` + `CacheMeta` returned by `fetch` are
  meant to flow into `QueryClient` via a fetcher-returned policy channel, so HTTP
  semantics can override the caller's per-query policy mid-flight (server wins) —
  **once Required Core Change 1 is implemented**. Today no such mid-flight
  override path exists; the resource policy is fixed at `begin_request`.
- `CacheMeta` is `serde::{Serialize, Deserialize}` so that a **future**
  persistence layer can persist ETags for cold-start 304 refetches. This is
  forward-looking: the persistence layer that actually ships today
  (`QueryPersister` — a synchronous, object-safe trait with `load()` / `save()` in
  `client/erased.rs`; `DehydratedEntry` in `client/devtools.rs`) is synchronous and
  stores only metadata — its three fields are exactly
  `{ key: String, type_id: TypeId, kind: &'static str }`, with no `meta` / `value`
  / `data` field (the old `data_json` field was deliberately removed as dead
  weight, audit fix #L14). `DehydratedEntry` is `#[derive(Clone, Debug)]` only —
  **not** serde. Storing `CacheMeta` is part of the richer `PersistedEntry` design
  proposed in Section 2, not something the current skeleton supports. Note also
  that the `QueryPersister` trait doc-comment claims entries are "serialized as
  JSON strings to avoid generic bounds," but `DehydratedEntry` is not itself serde
  (it holds a `TypeId`), so serialization is the persister's problem to solve.
- `HttpCache<C>` is generic over the client so tests can inject a stub client
  (static dispatch, per `rust-best-practices` ch.6). The default
  `C = reqwest::Client` keeps the common path ergonomic.

### Out of scope

- Streaming responses, range requests, `Vary` handling. Add only when a real
  consumer needs them.
- Response body storage strategy beyond an in-memory `HashMap`. If a consumer
  needs disk-backed body storage, they implement their own `HttpCache`-shaped
  wrapper.

---

## 2. Persistence — `gpui-query-persist`

### Scope

Whole-bucket concern. Today the crate ships a **synchronous, metadata-only**
persistence skeleton: a tiny object-safe `QueryPersister` trait, a
`DehydratedState` / `DehydratedEntry` snapshot (keys + type identity + kind, **no
data payload**), and `QueryClient` methods that dehydrate, save, load, and provide
a (currently no-op) hydration hook. The richer design below — typed data in
entries, an async persister, debounced auto-save, filters, max-age, and schema
versioning — is the **proposed evolution** of that skeleton, not a description of
what ships.

This is the single biggest reconciliation in the doc: the earlier draft described
the proposed API as if it already existed. It does not. The two real gaps are (1)
`DehydratedEntry` carries **no data** (the field was deliberately removed), and
(2) `QueryClient::hydrate` is a **no-op**. Everything else in the richer design is
future work on top of the shipped skeleton.

### What ships today (`client` layer)

Today the persistence skeleton is compiled whenever the `client` feature is
enabled — it is **not yet** behind the proposed `persist` gate (see [Crate
Layout](#crate-layout)). The surface lives in `client/erased.rs` and
`client/devtools.rs`, and the `QueryClient` methods live in `client/lifecycle.rs`.

```rust
// client/erased.rs — object-safe persistence adapter trait
pub trait QueryPersister: Send + Sync {
    fn load(&self) -> Vec<DehydratedEntry>;
    fn save(&self, entries: Vec<DehydratedEntry>);
}

// client/devtools.rs — the dehydrated snapshot (metadata only)
#[derive(Clone, Debug)]
pub struct DehydratedEntry {
    pub key: String,
    pub type_id: TypeId,    // std::any::TypeId — NOT serializable as-is
    pub kind: &'static str, // "query" | "infinite" | "mutation"
}

#[derive(Clone, Debug, Default)]
pub struct DehydratedState {
    pub entries: Vec<DehydratedEntry>,
}

// client/lifecycle.rs — QueryClient methods
impl QueryClient {
    /// Success-status entries only; metadata only (no data payload).
    pub fn dehydrate(&self, cx: &App) -> DehydratedState;

    /// NO-OP today. Documented hook point for typed hydration. Callers that
    /// know the concrete (T, E) restore typed data themselves via
    /// QueryClient::set_query_data (client/mod.rs).
    pub fn hydrate(&mut self, _state: DehydratedState, _cx: &mut App);

    /// Dehydrate, then `persister.save(state.entries)`.
    pub fn persist(&self, persister: &dyn QueryPersister, cx: &App);

    /// Associated fn (no &self): returns `persister.load()`.
    pub fn restore(persister: &dyn QueryPersister) -> Vec<DehydratedEntry>;
}
```

A `FilePersister` is already shown as the trait's doc example in
`client/erased.rs` — a trivial `impl QueryPersister` whose `load` / `save` move
`Vec<DehydratedEntry>` in and out. It is documentation, not a shipped adapter
module.

**Two honesty notes about the shipped skeleton:**

- `DehydratedEntry` is **not** `#[derive(Serialize, Deserialize)]`. It holds a
  `std::any::TypeId`, which is not directly serializable, and derives only
  `Clone, Debug`. The trait doc-comment's claim that "entries are serialized as
  JSON strings" is the *persister's* problem to solve (e.g. persist `key` + `kind`
  and re-derive type identity at restore time), not something the entry type
  provides for you.
- There is **no data field**, by design (audit fix #L14): `dehydrate` always
  populated it with `None`, so it was removed as dead weight. Typed data
  serialization, when it lands, will be added as a real field rather than a
  permanently-`None` placeholder.

### Proposed evolution (not yet implemented)

The richer API below is the goal of a future `gpui-query-persist` companion crate
plus small core additions. Every item marked **Proposed** below is future work.

```rust
// PROPOSED — core additions (could sit behind a future `persist` feature flag)
//
// An async, debounced, filtered auto-save path built on top of the existing
// skeleton. The shipped QueryPersister is synchronous; this richer shape is
// what the companion crate would target once it exists.

/// PROPOSED. Persistence backend with async load/save. Methods return
/// `impl Future<...> + Send` (not `async fn`) so the returned futures are
/// guaranteed `Send` and can be spawned on the consuming app's tokio runtime.
/// This makes the trait NOT object-safe; use generics (`persist_with<P>`) for
/// static dispatch. (The shipped QueryPersister stays synchronous and
/// object-safe; this is an alternative shape, not a replacement shipped today.)
pub trait Persister: Send + Sync + 'static {
    fn load(&self)
        -> impl Future<Output = Result<HashMap<QueryKey, PersistedEntry>, PersistError>> + Send;
    fn save(&self, snapshot: &PersistSnapshot)
        -> impl Future<Output = Result<(), PersistError>> + Send;
}

/// PROPOSED. A dehydrated entry WITH a data payload — the field that
/// DehydratedEntry deliberately omits today. The `value` is an opaque
/// serde_json::Value so core never needs to know the concrete (T, E); typed
/// de/serialization happens at the persister-adapter boundary.
pub struct PersistedEntry {
    pub value: serde_json::Value,
    pub cached_at: u64,                   // epoch-millis (SystemTime-based, like current_time_ms)
    pub cache_policy: CachePolicy,        // core's CachePolicy is Copy + Serialize
    pub meta: Option<serde_json::Value>,  // opaque, e.g. CacheMeta round-trip
}

/// PROPOSED.
pub struct PersistSnapshot {
    pub entries: HashMap<QueryKey, PersistedEntry>,
    pub version: u32,
}

/// PROPOSED.
pub struct PersistOptions {
    // NOTE: core's QueryKeyFilter<'a> borrows a &'a QueryKey and is not
    // Serialize, so it cannot be stored here as-is. This field needs an
    // owned filter variant (or its own owned filter type) before this struct
    // can compile. Flagged as an open design detail, not a resolved one.
    pub filter: /* owned filter, design TBD — core's QueryKeyFilter cannot be reused directly */,
    pub max_age: Duration,
    pub debounce: Duration,
    // dehydrate_filter: …,  // PROPOSED, type TBD — e.g. skip entries whose
    //                        // serialized value exceeds a size budget (Open Question 4).
}

/// PROPOSED — drop guard that unsubscribes the debounced observer.
pub struct PersistHandle(/* drops to unsubscribe */);

impl QueryClient {
    /// PROPOSED. Subscribe to bucket mutations via `cx.observe_global::<QueryClient>()`
    /// (see Required Core Change 2, Option A), debounce, and call `persister.save`.
    pub fn persist_with<P: Persister>(&self, p: P, opts: PersistOptions) -> PersistHandle;

    /// PROPOSED. Async load on the tokio runtime, filter by max_age/opts.filter,
    /// then prime each entry on the gpui foreground thread via
    /// QueryClient::set_query_data (client/mod.rs). Returns the count rehydrated.
    pub async fn hydrate<P: Persister>(&self, p: &P, opts: &PersistOptions)
        -> Result<usize, PersistError>;
}
```

The proposed `Persister` trait belongs in core because it is tiny (two methods),
central, and defines the contract every adapter must satisfy; the adapters do
not. Note the contrast with the shipped skeleton: today's `QueryPersister` is the
synchronous, object-safe version; the richer `Persister` above is the async,
non-object-safe version the companion crate is aimed at. Deciding whether the two
coexist or the async one supersedes the shipped one is itself an open design
question.

### Disk-based persistence

The headline adapter for `gpui-query-persist` is a **proper disk-based
`FilePersister`** — the normal case for offline data caching. It must work
identically on Linux, macOS, and Windows and survive the usual failure modes
(crash mid-write, corrupt file, permission errors, schema drift):

- **OS-appropriate location.** When the caller does not pass an explicit path,
  resolve a per-app directory with the `dirs` crate and join `<app>` — never
  hardcode a path or separator, always go through `PathBuf`, and never `unwrap`
  the dir (it is `None` in sandboxed/headless envs — return `PersistError::BadPath`
  instead). The default dir is an open choice (see Open Question 8):
  `dirs::data_dir()` (`$XDG_DATA_HOME` / `~/.local/share` on Linux,
  `~/Library/Application Support` on macOS, `%APPDATA%` / Roaming on Windows) vs
  `dirs::cache_dir()` (`~/.cache`, `~/Library/Caches`, `%LOCALAPPDATA%`) — a
  regenerable offline cache conventionally uses `cache_dir()`, which also avoids a
  Windows Roaming profile syncing a regenerable store across machines. Under the
  macOS App Sandbox, both resolve inside the per-app container; an absolute-path
  override outside the container needs entitlements. The caller can always pass an
  explicit absolute path.
- **Atomic writes (atomicity ≠ durability — both matter).** Serialize to a sibling
  temp file, then replace the target **atomically** via `tempfile`'s
  `NamedTempFile::persist`, which dispatches `rename(2)` on POSIX and
  `MoveFileExW`/`ReplaceFile` on Windows. Do **not** call raw `std::fs::rename`
  yourself: on Windows/NTFS it can fail with `ERROR_ACCESS_DENIED` when another
  process (e.g. an AV scanner) or a concurrent reader holds the destination
  (rust-lang/rust#123985) — treat that as a retryable `PersistError::Permission`.
  This guarantees *atomicity* (no torn file is ever visible; the previous good file
  survives a crash). For *durability* across power loss, `fsync` the temp file
  before the replace (use `fcntl(F_FULLFSYNC)` on macOS, where plain `fsync` is
  advisory), and `fsync` the **parent directory** fd after the replace on POSIX.
  Never overwrite the target path in place.
- **Tolerant load.** A missing file is the empty store (first run), not an error.
  A corrupt or partially-written file is logged and treated as empty — **never
  panic, never abort startup**. A version mismatch surfaces as a typed
  `PersistError::VersionMismatch` so the app can wipe-and-continue or bail.
- **Portable serialization.** `PersistFormat::Json` (human-readable, diffable, the
  default) or `PersistFormat::Bincode` (compact, faster, for large caches). Both
  round-trip the same `PersistSnapshot`. JSON is text; bincode is little-endian —
  fine for a local cache, not a cross-architecture wire format.
- **Concurrency.** Writes are serialized — one save at a time, enforced by the
  persister (e.g. a serialization `Mutex` on the tokio save path or a chained
  in-flight future; the debounced `persist_with` would own this once it exists).
  `load` is independent. No lock is held across the gpui↔tokio boundary — the
  persister owns its IO entirely on the tokio side.
- **Typed errors.** Every IO failure maps to a `PersistError` variant (`Io`,
  `Serialize`, `Deserialize`, `VersionMismatch`, `Permission`, `BadPath`) — no
  `unwrap` / `expect`, per `rust-best-practices` ch.4. The app decides whether a
  failed save is fatal.

Encryption, compression, and non-disk backends (SQLite, keychain) are adapter
concerns, not core's — see Open Question 6.

### Proposed companion crate

```rust
// PROPOSED — crates/gpui-query-persist/src/lib.rs (does not exist today)
//
// A real disk-based FilePersister (see "Disk-based persistence" above) plus the
// types it needs. This is the reference adapter; the doc-snippet FilePersister in
// client/erased.rs is only a trait-example.
pub enum PersistFormat { Json, Bincode }

pub struct FilePersister {
    path: PathBuf,       // explicit, or resolved from dirs::data_dir() + app_name
    format: PersistFormat,
    app_name: String,    // names the subdir under the OS data dir
}

impl Persister for FilePersister { /* atomic write + tolerant load, as above */ }

pub struct NoopPersister;     // for tests / in-memory
impl Persister for NoopPersister { /* ... */ }
```

`FilePersister` is what most consumers will use; `NoopPersister` is for tests. A
`SqlitePersister` or `SecureStoragePersister` stays in the consuming app.

### App-specific adapters (in the consuming app)

```rust
// In a consuming app (e.g. one modeled on gpui-app) — NOT in this crate.
// SqlitePersister depends on that app's own storage layout and migrations,
// so it lives with the app, never here.
pub struct SqlitePersister<'a> { pool: &'a sqlx::SqlitePool, table: &'static str }
impl Persister for SqlitePersister<'_> { /* ... */ }
```

This stays in the app because it depends on the app's existing storage layout and
migrations. `FilePersister` is the reference adapter that would ship in the
proposed crate; `SqlitePersister` is the app-specific adapter that ships with the
app.

### Hydration sequence at startup

Today the shipped `hydrate` is a no-op, so the only working restore path is
`QueryClient::restore(&persister)` (loads `Vec<DehydratedEntry>`) followed by
caller-driven typed restoration via `QueryClient::set_query_data`. The async,
foreground-priming sequence below describes how the **proposed** `hydrate<P>`
would behave once implemented.

A consuming app already bridges tokio and gpui via its own runtime; treat any such
bridge as an example of a generic external consumer, not as part of this crate.

```
app startup (App context)
  │
  ├── cx.set_global(QueryClient::new(...))
  ├── cx.spawn(async move |cx| {
  │     // load() runs on the consuming app's tokio runtime (IO);
  │     // priming runs on the gpui foreground thread.
  │     let entries = persister.load().await?;            // PROPOSED async load
  │     cx.update_global::<QueryClient, _>(|client, cx| {
  │         for (key, entry) in entries {
  │             client.set_query_data::<T, E>(&key, /* deserialized */, cx);
  │         }
  │     }).ok();
  │     // begin observing mutations
  │     let _handle = cx.update_global::<QueryClient, _>(|client, _| {
  │         client.persist_with(persister, opts)           // PROPOSED
  │     });
  │     // store _handle for app lifetime so the subscription is not dropped
  │ }).detach();
  ├── gpui window opens
  └── use_query hooks observe already-primed entries → render cached data
                                                    → refetch in background
```

`persist_with`'s `save()` hops the other way: the debounced observer callback runs
on the gpui foreground thread, collects the snapshot, then the app spawns
`persister.save(&snapshot)` on its tokio runtime to write without blocking the UI.

### Interaction with `gpui-query-http`

The one place these two crates cooperate: persisting `CacheMeta` so cold-start
refetches send `If-None-Match` and get cheap `304`s. The proposed
`PersistedEntry::meta` is typed `Option<serde_json::Value>` (opaque to core) so
`gpui-query-persist` does **not** need to depend on `gpui-query-http`. The HTTP
crate serializes `CacheMeta` into that `Value` on save and deserializes it back on
load. (`CacheMeta` is itself proposed — it does not exist in core today; it would
be defined by the proposed `gpui-query-http` crate.) Zero coupling between the two
companion crates; they share only the `serde_json::Value` contract and the
`CachePolicy` type from core.

This coupling boundary is sound — but it only becomes meaningful once the proposed
`PersistedEntry` and `CacheMeta` exist. Today's `DehydratedEntry` carries no data
and no meta at all, so cross-crate data sharing is not yet possible.

### Design notes

- **No generic mutation hook.** `persist_with` is specific: it subscribes via
  `cx.observe_global::<QueryClient>()` and calls one trait method. Adding a
  general `on_mutate` hook would invite misuse and bloat core. Note that
  `observe_global::<QueryClient>()` **fires automatically** on any
  `cx.update_global::<QueryClient, _>` (GPUI pushes a `NotifyGlobalObservers`
  effect), and the crate already routes fetch / retry / mutation / infinite-query
  completion through `update_global`, so the subscription works for free on those
  paths. The only gap is `QueryClient::set_query_data`, which mutates a resource
  entity (no global notify) — that one path needs an explicit notify (or routing
  through `update_global`) to be observed.
- **Server wins.** `CachePolicy` stored in `PersistedEntry` (proposed) is whatever
  the server last returned (via `Fetched.cache_policy`, itself proposed — see
  Required Core Change 1). On rehydrate, that policy applies until the next fetch
  refreshes it.
- **Debounce is mandatory.** `PersistOptions::debounce` (proposed; concrete default
  to be chosen when the type lands — not yet fixed at any value) gates saves;
  passing `Duration::ZERO` is allowed but discouraged for non-toy persisters.
- **Versioning.** `PersistSnapshot::version` (proposed) is bumped on every breaking
  change to `PersistedEntry`'s schema. `hydrate` rejects mismatched versions with
  a typed error; apps decide whether to wipe and continue or surface the error.
- **`Send + Sync + 'static`** on `Persister` is required because `save` / `load`
  are spawned on the tokio runtime (`rust-best-practices` ch.9).
- **The real gaps, restated.** Two things make the shipped skeleton unsuitable for
  real data persistence today: `DehydratedEntry` has no data field (metadata
  only), and `QueryClient::hydrate` is a no-op. The proposed evolution adds the
  data payload, the typed/async persister, the debounced subscription, and the
  versioned snapshot on top of the existing synchronous `QueryPersister` /
  `dehydrate` / `persist` / `restore` plumbing.

---

## Integration Plan for a Consuming App

Concrete steps for wiring the proposed companion crates into a consuming app.
`gpui-app` is one example **external consumer** of this crate — it is not part of
this repo. Every path below (`Cargo.toml`, a tokio-runtime helper, a query
playground, a storage service) is illustrative of where *any* consumer would hook
things in, not a path in this crate.

Two things must be clear up front:

1. **A persistence skeleton already ships** with `gpui-query` today (under the
   `client` feature, no separate feature flag): the synchronous, object-safe
   `QueryPersister` trait (`client/erased.rs`), `DehydratedEntry` /
   `DehydratedState` (`client/devtools.rs`), and
   `QueryClient::dehydrate` / `hydrate` / `persist` / `restore`
   (`client/lifecycle.rs`). The steps below distinguish *use the shipped skeleton*
   from *adopt the proposed richer API*.
2. **`gpui-query-http` and `gpui-query-persist` do not exist yet** — they are
   proposed companion crates (see [Crate Layout](#crate-layout)). Steps that
   reference them are contingent on those crates being created.

### Step 0 — Use the shipped persistence skeleton (no companion crate required)

A consumer can persist query metadata across restarts *today*:

1. **`Cargo.toml`** — depend on `gpui-query` with the `client` feature (and
   `hook` if using hooks). Persistence currently ships with `client` (no flag
   needed yet); once the proposed `persist` gate lands, enable it explicitly.
2. **Implement `QueryPersister`** in the consumer (e.g. a `query_persister` module
   next to the app's other storage code). The trait is two synchronous methods:
   `fn load(&self) -> Vec<DehydratedEntry>` and
   `fn save(&self, entries: Vec<DehydratedEntry>)`. Because it is object-safe, hand
   the consumer's instance to `QueryClient::persist(&self, &dyn QueryPersister, cx)`.
3. **On shutdown / periodically** — call `QueryClient::persist(&persister, cx)`,
   which `dehydrate()`s Success entries and `save()`s their metadata (key +
   `TypeId` + kind).
4. **On startup** — call the associated function
   `QueryClient::restore(&persister) -> Vec<DehydratedEntry>` to load entries,
   then iterate and restore typed data yourself via
   `cx.update_global::<QueryClient, _>(|client, cx| client.set_query_data::<T, E>(
   ...))` for each entry whose `(T, E)` you know (`set_query_data` is `&mut self`
   on a `Global`, so it must go through `update_global`).
   `QueryClient::hydrate(...)` is currently a **documented no-op** hook point — it
   does not prime anything; typed restoration is the caller's job.

> **Real gap.** The shipped skeleton is metadata-only. `DehydratedEntry` has
> **no value/data field** (it was deliberately removed as dead weight; see its
> doc comment in `client/devtools.rs`), and `dehydrate()` emits no payload. So a
> cold restart today restores *that keys existed* but not their data. Closing
> that gap is the point of the proposed persistence work below.

### Step 1 — Adopt the proposed richer persistence API (future work)

Once the proposed `gpui-query-persist` companion crate and its core changes land
(see [Required Core Changes](#required-core-changes) and
[Open Questions](#open-questions)), the consumer upgrades:

1. **`Cargo.toml`** — add the proposed `gpui-query-persist` dep and enable the
   proposed `persist` feature on `gpui-query` (it gates the persistence surface —
   offline data only). Today neither exists; this is contingent on the proposal.
2. **Replace the manual `persist` / `restore` calls** with the proposed
   `QueryClient::persist_with(persister, opts)` (debounced global observer) and
   the proposed typed `hydrate`. These do not exist yet.
3. **`SqlitePersister`** (or equivalent) stays in the consumer, implementing the
   proposed async `Persister` against the consumer's existing storage layout and
   migrations — it is not crate code.

### Step 2 — Adopt the proposed HTTP cache helper (future work)

Contingent on the proposed `gpui-query-http` crate and Required Core Change 1
(`Fetched<T, E>` + a `*_with_policy` fetcher variant), neither of which exists
today. The real current fetcher shape is
`use_query(options, fetcher: Fn(QuerySignal) -> Fut, cx) -> (Entity<QueryResource<T,E>>, Subscription)`
(see `hook/query_hooks.rs`).

1. **The consumer's HTTP client wrapper** — wherever it owns a shared
   `reqwest::Client` (a tokio-runtime helper module), wrap it once in the proposed
   `HttpCache` and expose `Arc<HttpCache>` alongside the raw client.
2. **Fetch sites** — replace raw `reqwest` bodies inside fetcher closures with
   `HttpCache::fetch`, and switch those fetchers to the proposed `*_with_policy`
   variant returning `Result<Fetched<T, E>, E>` so the server-returned
   `CachePolicy` flows back (server wins). This requires Core Change 1 to be
   implemented first.
3. **DevTools wiring** — wherever `QueryClient` is constructed today (per the
   consumer's devtools setup), the existing `QueryClient::diagnostics()` (method
   in `client/lifecycle.rs`; diagnostic *types* in `client/devtools.rs`) can
   surface a persistence section. A "Persistence" card in the consumer's
   playground (hydrate count on startup, last save timestamp, persisted key count)
   doubles as the integration-test surface.

---

## Migration Order

The order matters because the richer persistence API touches core, and the HTTP
helper depends on the proposed `Fetched<T, E>` core change. Note that a
metadata-only persistence skeleton already ships, so "persistence" below means
*upgrading* it, not building it from zero. All persistence work — gating the
shipped skeleton, the async upgrade, and the disk adapter — lands behind the
proposed `persist` feature (offline data only).

1. **Use the shipped skeleton** (no core change). Consumers can wire
   `QueryPersister` + `QueryClient::persist` / `restore` + caller-side typed
   `set_query_data` priming today. This is metadata-only; verify with
   `#[gpui::test]` + `TestAppContext` (the crate's own tests already use this
   pattern — see `tests/`).
2. **Proposed Core Change 1 — `Fetched<T, E>` + `*_with_policy` fetcher variant**
   in `gpui-query`. Non-breaking: the existing `use_query` signature (signal
   fetcher, tuple return) is unchanged; the new variant is additive. Add unit
   tests that a `Fetched` with `Some(policy)` overrides the bucket's policy and
   `None` keeps it.
3. **Feature-gate the shipped skeleton, then upgrade it.** First land the three
   gating prerequisites (extract `current_time_ms` from `client/erased.rs`; decide
   `collect_key_status_into`'s fate; cfg-gate the itemized `pub use` lists in
   `client/mod.rs`) and move the persistence surface behind `persist`. Then add
   the async `Persister`, `PersistedEntry { value, ... }`, `persist_with`, typed
   `hydrate`, `PersistOptions { filter, max_age, debounce }`, and snapshot
   versioning on top of the shipped `QueryPersister` skeleton. `persist_with`
   uses Core Change 2 Option A (`observe_global::<QueryClient>()`) — which works
   for free on the mutations already routed through `update_global`, needing only
   a small notify hook for the `set_query_data` path (see Open Question 2). Ship a
   reference `FilePersister` in the proposed `gpui-query-persist` crate. Add unit
   tests for debounce, `max_age` filtering, and version mismatch — these need
   `#[gpui::test]` + `TestAppContext` since they touch `QueryClient`.
4. **Proposed `gpui-query-http`** with `CacheMeta`, `cache_policy_from_headers`,
   and `HttpCache`. Header-parsing tests are plain `#[test]` (no gpui context);
   the `304` / SWR / `no-store` integration tests use a `mockito` server on tokio.
5. **Wire a consuming app** to both per the integration plan. Replace raw
   `reqwest` calls and add an app-specific persister.

---

## Test Strategy

Per `gpui-test`: tests that touch `QueryClient` (a `Global`) need
`#[gpui::test]` + `TestAppContext`; pure-logic tests (header parsing, debounce
math) are plain `#[test]`. This matches the crate's own test suite, which uses
`#[gpui::test]` + `TestAppContext` throughout `tests/` (with `setup_query_client`
/ `setup_test` helpers in `tests/test_support.rs`).

| Target                                | Unit tests                                                | Integration tests                                  | GPUI ctx? |
| ------------------------------------- | --------------------------------------------------------- | -------------------------------------------------- | --------- |
| Shipped persistence skeleton          | `QueryPersister` load/save round-trip; `restore` returns entries | dehydrate → persist → restore → caller `set_query_data` priming | Yes       |
| Proposed Core 1 (`Fetched` / `*_with_policy`) | `Fetched` policy override, `None` keeps policy            | `*_with_policy` end-to-end                         | Yes       |
| Proposed Core 3 (upgraded persistence) | debounce coalescing, `max_age` filtering, version reject  | hydrate → persist → hydrate round-trip             | Yes       |
| Proposed `gpui-query-http`            | header parsing edge cases                                 | `mockito` server: 200 / 304 / SWR / `no-store`     | No        |
| Proposed `gpui-query-persist`         | `FilePersister` round-trip, concurrent saves              | large snapshot, schema migration                   | No        |
| Consuming app                         | —                                                         | playground section card asserts real flows         | Yes       |

Cross-crate integration test (lives in the consuming app's `tests/`, since only
the app wires both proposed crates together): HTTP fetch → `CacheMeta` stored in
the proposed `PersistedEntry::meta` → app shutdown → app restart → `hydrate` →
next fetch sends `If-None-Match` → server returns `304` → cached body served, no
full refetch. This one test exercises the entire proposed design end-to-end.

---

## Open Questions

1. **Fetcher-returns-policy shape** — Option A (`Fetched<T, E>` wrapper) vs
   Option B (injected `QueryContext` callback)? Default: A, for explicitness and
   serializability. See Required Core Change 1.
2. **Bucket-mutation observation** — Option A (`observe_global::<QueryClient>()`,
   coarse) vs Option B (typed dirty signal, precise)? Default: A for v1.
   Clarification: in GPUI, `cx.update_global::<QueryClient, _>` **does**
   auto-notify `observe_global::<QueryClient>()` subscribers (it pushes a
   `NotifyGlobalObservers` effect), and the crate already routes fetch / retry /
   mutation / infinite-query completion through `update_global` at six sites
   (`hook/query_hooks.rs`, `hook/fetch_retry.rs`, `hook/mutation_hooks/hooks.rs`,
   `hook/use_infinite_query/*`). So Option A works for free on those paths. The
   only mutation that does **not** wake a global observer today is
   `QueryClient::set_query_data`, which mutates a resource entity directly — that
   path needs an explicit global notify (or re-routing through `update_global`).
   Option B (a typed dirty signal) is more precise but more core surface; defer
   unless profiling shows the coarse path firing too often.
3. **Adding a typed value field to the shipped metadata-only entry.** The shipped
   `DehydratedEntry` is `{ key, type_id, kind }` with **no value field** (it was
   removed as dead weight — see its doc comment in `client/devtools.rs`), and
   `dehydrate()` emits no payload. The real open question is how to **add** a
   typed value: a new `serde_json::Value` field (flexible, opaque to core, slower)
   vs a generic `T: Serialize + DeserializeOwned` path (typed, faster, more
   bounds). Default: a `serde_json::Value` payload on the entry (or on a new
   `PersistedEntry`), with typed de/serialization at the `Persister` adapter
   boundary — but this is net new, not a choice between existing shapes.
4. **Per-key persistence policy** — a proposed `PersistOptions::filter` would be
   global. If apps need per-key TTLs or per-key serializers, extend
   `QueryKeyFilter` (in `core/key_filter.rs`) — noting its `<'a>` borrow and lack
   of `Serialize` — or add a `PersistRule` enum. Defer until a concrete need
   exists.
5. **Object safety of the PROPOSED async `Persister`.** Note: the shipped
   `QueryPersister` is **already synchronous and object-safe** — it is used as
   `&dyn QueryPersister` by `QueryClient::persist` and `QueryClient::restore` (see
   `client/lifecycle.rs`). The object-safety tension is a consequence of the
   proposed upgrade to `impl Future + Send` return types, which would make the
   trait not object-safe (`Box<dyn Persister>` would not compile). That is fine
   for generics (`persist_with<P>`); if runtime dispatch is ever needed (dev vs
   prod persister swap), switch to `Pin<Box<dyn Future<...> + Send>>`.
6. **Encryption** — should the `Persister` support encrypted backends, or is that
   the adapter's job? Recommendation: adapter's job. Core hands the adapter a
   snapshot; the adapter encrypts however it wants.
7. **DevTools surface** — `QueryClient::diagnostics()` already exists (method in
   `client/lifecycle.rs`; diagnostic types in `client/devtools.rs`). Add a
   persistence section showing last save, last hydrate, persisted key count, and
   per-key age. Wire into the consumer's existing Query DevTools page.
8. **Default directory — `data_dir()` vs `cache_dir()`.** The offline cache is
   regenerable, for which `dirs::cache_dir()` (`%LOCALAPPDATA%` / `~/.cache` /
   `~/Library/Caches`) is the conventional choice and avoids a Windows Roaming
   profile silently syncing it across machines. `dirs::data_dir()` is the safer
   default if the store is treated as non-regenerable user data. Pick one as the
   default and document the roaming/sandbox consequences.
