# gpui-query Extensions — Design Plan

**HTTP cache helpers and persistence enrichment for the `gpui-query` crate.**

This is the `gpui-query` crate's own design doc for two companion crates —
`gpui-query-http` (HTTP cache semantics) and `gpui-query-persist` (persistence) —
grounded in the core API. **Both companions have shipped** (see [Implementation
Status](#implementation-status)); the historical "proposed" framing below is
preserved for rationale. The two were at very different maturity when this doc
was written:

- **HTTP cache helpers** were wholly proposed at the time; they now ship as
  `crates/gpui-query-http` (library-agnostic over `HttpBackend`).
- **Persistence** was **not** greenfield. The crate already shipped a synchronous,
  metadata-only persistence skeleton in its `client` layer (the `QueryPersister`
  trait, the `DehydratedEntry` / `DehydratedState` snapshot types, and the
  `QueryClient::dehydrate` / `hydrate` / `persist` / `restore` methods). The
  `gpui-query-persist` crate is an **enrichment** of that skeleton toward a
  richer, async, value-carrying API — and an extraction of the reference disk
  adapter into a companion crate. Nothing on the persistence side was "to be
  built from zero."

Throughout, `gpui-app` is **one example external consumer** of this crate — it is
not part of this repository, and this crate makes no claim of co-owning it. Every
`gpui-app` path in this document is illustrative of where any generic consuming
app would wire things in, never a path inside this crate.

> **Status:** Verified against the `gpui-query` crate source under
> `crates/gpui-query/src`. The crate exposes features `core`, `client`, `hook`,
> and `persist` (see `crates/gpui-query/src/lib.rs` and
> `crates/gpui-query/Cargo.toml`).
> Persistence is **optional** — it exists only for offline data caching — and
> the `persist` feature gate has **shipped**: it gates the richer value-carrying
> persistence surface (the async `Persister`, `PersistedEntry`, `persist_with`,
> typed `hydrate`, `PersistOptions`, and snapshot versioning). The original
> synchronous, metadata-only skeleton (`QueryPersister` / `dehydrate` /
> `hydrate` no-op / `persist` / `restore`) still ships unconditionally in the
> `client` layer; the `persist` feature layers the enrichment on top of it.
> The hooks use a signal-always fetcher closure: `use_query` in
> `hook/query_hooks.rs` takes `F: Fn(QuerySignal) -> Fut + Send + 'static` and
> returns a `(Entity<QueryResource<T, E>>, Subscription)` tuple. `QueryClient` is
> a `Global` (`impl Global for QueryClient` in `client/mod.rs`) with
> `set_query_data`, `resource`, `resource_with_policies`, and `diagnostics`. The
> `QueryObserver` / `Observer` machinery in `client/observer.rs` observes
> individual resource entities via `Observer::observe`; there is **no** `Resource`
> trait (the relevant bound is `ObservableResource`).
>
> **All items previously marked *Proposed* below — the `gpui-query-http` and
> `gpui-query-persist` crates, a disk-based cross-platform `FilePersister`, an
> async `Persister`, the `persist` feature gate, `Fetched<T>`, and
> `use_query_with_policy` — are now SHIPPED on master (820 tests pass across the
> three workspace members).** See [Implementation Status](#implementation-status)
> for the precise file/crate mapping. The prose below retains its original design
> rationale; where it now contradicts the shipped code it is corrected inline or
> flagged as superseded.

## Implementation Status

Everything this document proposes has landed and is green on `master`. The
workspace now has **three** members
(`crates/gpui-query`, `crates/gpui-query-http`, `crates/gpui-query-persist`;
see root `Cargo.toml`), and the full suite passes — `cargo test --workspace
--all-features` reports **820 passing tests** across the three crates.

| Design item (as named below)                          | Shipped location                                                                                       |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `Fetched<T>` result wrapper (Option A, **single** type param, no `PhantomData`) | `crates/gpui-query/src/core/fetched.rs`                                                              |
| `use_query_with_policy` / `fetch_query_with_policy` hooks | `crates/gpui-query/src/hook/query_hooks.rs`                                                        |
| `persist` cargo feature (gates `serde_json` + `thiserror`) | `crates/gpui-query/Cargo.toml`                                                                     |
| Async `Persister` trait (`impl Future + Send`, non-object-safe) | `crates/gpui-query/src/client/persist.rs`                                                       |
| `PersistedEntry { value, cached_at, cache_policy, meta }` | `crates/gpui-query/src/client/persist.rs`                                                         |
| `PersistSnapshot { entries: HashMap<String, PersistedEntry>, version }` (keyed by `QueryKey::to_path()`) | `crates/gpui-query/src/client/persist.rs`                                            |
| `PersistFilter { Exact, Prefix, All }` (owned)        | `crates/gpui-query/src/client/persist.rs`                                                              |
| `PersistOptions { filter, max_age, debounce }`        | `crates/gpui-query/src/client/persist.rs`                                                              |
| `PersistError` (Io/Serialize/Deserialize/VersionMismatch/BadPath/Permission) | `crates/gpui-query/src/client/persist.rs`                                                  |
| `QueryClient::persist_with<P: Persister>` + `NoopPersister` | `crates/gpui-query/src/client/persist.rs`                                                       |
| Free fn `hydrate<P: Persister>` (value-carrying, strict-deserializer contract) | `crates/gpui-query/src/client/persist.rs`                                              |
| Serializer / deserializer registries on `QueryClient` | `crates/gpui-query/src/client/persist.rs`                                                              |
| `CacheMutation` marker `gpui::Global` (Option B dirty signal) | `crates/gpui-query/src/client/mutation_signal.rs`; bumped via `cx.default_global::<CacheMutation>()` at the three completion-site families + `set_query_data`, observed via `cx.observe_global::<CacheMutation>()` in `persist_with` |
| `PERSIST_VERSION = 1` (version **rejection** only — no migration) | `crates/gpui-query/src/client/persist.rs`                                                   |
| `CacheMeta { etag, last_modified, stored_at, fresh_for, stale_for }` | `crates/gpui-query-http/src/lib.rs`                                                       |
| `cache_policy_from_headers(&HeaderMap) -> Result<CachePolicy, ParseError>` | `crates/gpui-query-http/src/lib.rs`                                                  |
| `HttpBackend` trait + `HttpCache<B: HttpBackend>` (library-agnostic; keys are `String`) | `crates/gpui-query-http/src/backend.rs`, `crates/gpui-query-http/src/cache.rs`             |
| `ReqwestBackend` (ONE optional backend behind the `reqwest` cargo feature; never a hard dep) | `crates/gpui-query-http/src/reqwest_backend.rs`                                        |
| `FilePersister { path, format, write_lock }` (atomic write, `F_FULLFSYNC`, tolerant load) | `crates/gpui-query-persist/src/lib.rs`                                                   |
| `FilePersister::in_cache_dir(app_name)` (resolves via `dirs::cache_dir()`, no `app_name` field) | `crates/gpui-query-persist/src/lib.rs`                                            |
| `PersistFormat::Json \| Bincode`                       | `crates/gpui-query-persist/src/lib.rs`                                                                 |

**Supersedes the "Current workspace" snapshot below** (which describes only
`crates/gpui-query` and treats the two companions as not-yet-existing). The
design rationale in the rest of the document is preserved; factual code
shapes have been corrected inline, and a handful of sections that described the
shipped API as "proposed / does not exist yet" carry a brief note pointing back
here.

---

## Table of Contents

1. [Guiding Principles](#guiding-principles)
2. [Extension Points](#extension-points)
3. [Crate Layout](#crate-layout)
4. [Shared Types](#shared-types)
5. [Required Core Changes](#required-core-changes)
6. [1. HTTP Cache Helpers — `gpui-query-http` *(shipped)*](#1-http-cache-helpers--gpui-query-http)
7. [2. Persistence — `gpui-query-persist` *(shipped enrichment of the legacy skeleton)*](#2-persistence--gpui-query-persist)
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
   `restore` methods, the async `Persister`, and the `HttpBackend` boundary)
   belong in — or directly adjacent to — this crate. The repo owns the two
   companion crates (`gpui-query-http`, `gpui-query-persist`) as workspace
   members; it does not co-own a consuming app. A consuming app (for example, a
   hypothetical `gpui-app`) is an *external* user and ships only its own
   backend-specific adapters. The reason to centralize the trait and reference
   adapters here is single-sided design authority, not coordination with another
   owned crate.
5. **Reference adapters ship with the crate; app-specific adapters ship with the
   app.** A reference `FilePersister` appears as the canonical example in the
   `QueryPersister` trait docs in `client/erased.rs` (for the legacy skeleton), and
   a **real, disk-based, cross-platform `FilePersister`** ships in
   `gpui-query-persist` (see [2. Persistence](#2-persistence--gpui-query-persist)).
   A `SqlitePersister` (which depends on a consumer's own storage layout and
   migrations) always stays in that consumer.
6. **Disk persistence is cross-platform and robust.** The reference `FilePersister`
   resolves an OS-appropriate cache directory (Linux / macOS / Windows —
   `dirs::cache_dir()` by default), writes
   atomically (temp file + replace — with the per-OS atomicity/durability nuances
   in [Disk-based persistence](#disk-based-persistence)), tolerates corruption
   without panicking, and serializes portably (JSON or bincode). It is robust
   on Linux, macOS, and Windows — surviving crash-mid-write, corrupt files,
   permission errors, and version mismatch (no schema migration is implemented).
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

| Concern     | Status today                                                                                                                                                                | Scope        | When               | Shipped shape                               | Home crate              |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------ | ------------------------------------------- | ----------------------- |
| HTTP cache  | **Shipped** (`crates/gpui-query-http`): `cache_policy_from_headers`, `CacheMeta`, `HttpBackend` trait, `HttpCache<B: HttpBackend>` (reqwest is one optional backend)        | per-fetch    | load time          | function + generic-over-backend wrapper     | `gpui-query-http`       |
| Persistence | **Shipped skeleton** (sync, metadata-only): `QueryPersister` trait (`client/erased.rs`) + `dehydrate` / `hydrate` / `persist` / `restore` (`client/lifecycle.rs`); metadata-only `DehydratedEntry` (no value field). **Shipped enrichment** (behind `persist`): async `Persister`, value-carrying `PersistedEntry`, `persist_with` on `CacheMutation`, typed `hydrate`, `PersistOptions`, version rejection | whole bucket | mutation + startup | async trait + subscribe (enrichment of the skeleton) | `gpui-query` (`persist`) + `gpui-query-persist` |

- **HTTP cache** is a wrapper over a backend trait — `HttpBackend` abstracts the
  single conditional `GET`, so any request library can plug in. The shipped
  `reqwest` backend is one optional implementation behind the `reqwest` feature.
  A struct with a `fetch` method is the correct shape, not a middleware layer.
  *(Shipped.)*
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
  shipped enrichment adds: an async `Persister`, a value/meta-carrying
  `PersistedEntry`, a `persist_with` debounce/filter subscription, version
  rejection, and extraction of the reference disk adapter into the
  `gpui-query-persist` crate. Note on the subscription path (the historical
  analysis that motivated the shipped `CacheMutation` signal): a `persist_with`
  built on `cx.observe_global::<QueryClient>()` fires only when the
  `QueryClient` **global** is mutated via `cx.update_global::<QueryClient, _>` /
  `global_mut` / `set_global` (these push a `NotifyGlobalObservers` effect). The
  crate's `update_global::<QueryClient, _>` sites all run at **fetch-start /
  hook-setup** — resource creation (`resource_with_policies`), request-id
  allocation (`next_request_id_for_key`), and mutation registration
  (`register_mutation`) — **not** at completion. The actual completion writes
  (`complete_success` / `complete_failure`) were bare `entity.update(...)` +
  `cx.notify()` on the resource **entity** (`hook/fetch_retry.rs`,
  `hook/mutation_hooks/internals.rs`, `hook/use_infinite_query/fetch_runners.rs`),
  which wake `cx.observe(&entity)` subscribers but **never** a global observer. So
  a `persist_with` on `observe_global::<QueryClient>` would have fired before
  fetched data landed (and `dehydrate()` emits only `Success` entries, so nothing
  useful was persistable yet) and would **miss** every completion. Closing this
  therefore needed every completion write to bump a dedicated signal — which is
  exactly what the shipped `CacheMutation` marker Global does
  (`cx.default_global::<CacheMutation>()` at those three completion-site families
  + `set_query_data`, observed via `cx.observe_global::<CacheMutation>()` in
  `persist_with`). `set_query_data` itself is **not** the gap: being `&mut self`
  on a `Global`, every caller already invokes it through `update_global` /
  `global_mut`, both of which notify (and it also bumps `CacheMutation`
  directly). See [Required Core Change 2](#2-whole-client-mutation-observation-for-persistence-subscription).

Each addition is ~50 lines of core surface, all behind the opt-in `persist`
feature (persistence is offline-data only). No plugin system, no middleware
chain, no lifecycle hook bag.

---

## Crate Layout

This is the **`gpui-query` crate's own** design doc. `gpui-app` (referenced
elsewhere in this document) is one example *external* consumer, never a member of
this repository.

### Current workspace (as it exists today)

> **Superseded by [Implementation Status](#implementation-status).** The
> workspace now has **three** members. The snapshot below is the pre-enrichment
> state, retained for the historical "what already shipped before this work"
> framing that the rest of this section builds on.

```
crates/
  gpui-query/        # the crate this doc belongs to: core + client + hook + persist layers
  gpui-query-http/   # SHIPPED: CacheMeta, cache_policy_from_headers, HttpBackend, HttpCache<B>
  gpui-query-persist/# SHIPPED: FilePersister, NoopPersister, PersistFormat
  gpui-query-legacy/ # legacy v1 crate (excluded from the workspace, kept for reference)
  crates-og.zip      # stray archive, NOT a crate (junk to be removed)
```

Per the root `Cargo.toml`, the workspace members are `crates/gpui-query`,
`crates/gpui-query-http`, and `crates/gpui-query-persist`
(`members = ["crates/gpui-query", "crates/gpui-query-http",
"crates/gpui-query-persist"]`); `crates/gpui-query-legacy` is present in the
tree but listed under `exclude`. There is **no** `gpui-app` crate in this repo
(it is an example external consumer).

### Feature flags (as shipped)

```toml
# crates/gpui-query/Cargo.toml
[features]
default = ["client"]
core   = []
client = ["core", "dep:gpui"]
hook   = ["client"]
# SHIPPED — gates the value-carrying persistence surface (offline data only);
# pulls serde_json + thiserror as optional deps.
persist = ["client", "hook", "dep:serde_json", "dep:thiserror"]
```

The crate exposes four layers — `core`, `client`, `hook`, and (optionally)
`persist` — gated by features of the same names (see
`crates/gpui-query/src/lib.rs`). `client` is on by default; `persist` is opt-in.

Persistence is **optional** (offline-data only), so the richer value-carrying
surface is gated behind the `persist` feature. The legacy metadata-only
skeleton — the `QueryPersister` trait (`client/erased.rs`), `DehydratedEntry` /
`DehydratedState` (`client/devtools.rs`), and the `QueryClient::dehydrate` /
`hydrate` / `persist` / `restore` methods (`client/lifecycle.rs`) — still ships
**unconditionally** as part of `client`; the `persist` feature layers the async,
value-carrying enrichment (`client/persist.rs`, `client/mutation_signal.rs`) on
top of it rather than replacing it.

(Historical gating analysis, retained for rationale: gating the *legacy*
skeleton itself was originally proposed but was not done — only the enrichment
was gated. The analysis below describes what full gating of the synchronous
skeleton would have entailed.) The gate is feasible but **not** zero-coupling —
three prerequisites would have to land first: (1) **extract `current_time_ms`**
out of `client/erased.rs` (it is defined
in the same module as `QueryPersister` and re-exported together at
`client/mod.rs`, but it is consumed by non-persistence **client-layer** code — gc
(`client/mod.rs`), the lifecycle methods (`client/lifecycle.rs`), and
`client/infinite_mutation_ops.rs` — so the module cannot be gated as-is; only the
`QueryPersister` symbol can move once the helper is relocated. The hook layer does
**not** consume this one: it has its own identical `current_time_ms` in
`hook/mod.rs`, and `lib.rs` deliberately re-exports both, so gating `erased.rs`
leaves the hook layer untouched; `prepared_fetch` only mentions it in doc-comments);
(2) decide
what to do with `collect_key_status_into` — a required method on the always-
compiled `ErasedBucket` / `ErasedInfiniteBucket` / `ErasedMutationBucket` traits
whose only caller is `dehydrate` (gate the method + its three impls together, or
leave the dead method when `persist` is off); (3) cfg-gate the itemized
`pub use devtools::{…}` and `pub use erased::{…}` lists in `client/mod.rs` in
lockstep with the gated symbols. Once those are done, `QueryClient::diagnostics()`
is unaffected (it returns `ClientDiagnostic` and never touches the dehydrate
types). Dependency note: `serde` is a non-optional core dep (`CachePolicy`
derives `Serialize` / `Deserialize` from it). **`serde_json` and `thiserror` are
now real gated deps under `persist`** — the value-carrying `PersistedEntry.value`
/ `Fetched.meta` are typed `serde_json::Value`, so the gate pulls `serde_json` in
for non-test code (and `thiserror` for `PersistError`). The disk-IO crates
(`dirs` / `tempfile` / `bincode`) live in the `gpui-query-persist` companion,
pulled only by apps that actually persist.

### Companion crates (shipped)

```
crates/
  gpui-query/          # core + client + hook + persist-feature layers
  gpui-query-http/     # SHIPPED: CacheMeta, cache_policy_from_headers, HttpBackend, HttpCache<B>
  gpui-query-persist/  # SHIPPED: FilePersister, NoopPersister, PersistFormat
```

A `SqlitePersister`, `SecureStoragePersister`, or any other app-specific adapter
would live in the **consuming app** (e.g. `gpui-app`), not in this repo — it
depends on the app's own storage layout and migrations.

### What persistence looks like today vs. what this doc proposed

The `gpui-query-persist` crate builds on a skeleton that **already shipped** in
the `client` layer. The list below is the pre-enrichment reality (the legacy
synchronous skeleton, which still ships unchanged); the gaps it identifies have
since been closed by the `persist`-feature surface (see [Implementation
Status](#implementation-status)).

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

The **real gaps** this doc identified, on top of that skeleton, were: (1) an
**async** `Persister` surface (the legacy trait is synchronous and blocks the
foreground thread), (2) a `PersistedEntry { value, cached_at, cache_policy, meta }`
shape that actually carries the cached value (the legacy entry is metadata-only),
(3) `persist_with` (a debounced, filtered subscription) — the legacy surface has
only the one-shot `persist()`, (4) a real (non-no-op) `hydrate` that primes via
`set_query_data`, (5) `debounce` / `max_age` / `filter` policy, and (6) snapshot
`version` rejection. **All six have shipped** behind the `persist` feature —
note that (6) is version **rejection** (`PersistError::VersionMismatch`), not
migration; no schema-migration path exists.

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

### Proposed (now **shipped** — see [Implementation Status](#implementation-status) and [2. Persistence](#2-persistence--gpui-query-persist))

The types below were originally listed as future work; they have all landed in
`crates/gpui-query/src/client/persist.rs` (behind the `persist` feature) or in
the `gpui-query-http` / `gpui-query-persist` companion crates.

| Type                | Would live in                       | Notes                                                                                                              |
| ------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Persister` (async) | `gpui-query` (client/persist.rs; **shipped**) | Async successor to the synchronous `QueryPersister`; `load` / `save` returning `impl Future + Send` (**not** object-safe). |
| `PersistedEntry`    | `gpui-query` (client/persist.rs; **shipped**) | Carries `value: serde_json::Value`, `cached_at`, `cache_policy`, `meta` — the typed payload `DehydratedEntry` lacks. |
| `PersistSnapshot`   | `gpui-query` (client/persist.rs; **shipped**) | `{ entries: HashMap<String, PersistedEntry>, version: u32 }`. Keys are the `String` form of `QueryKey::to_path()` so the snapshot is self-contained and serializable (an `Arc<[Arc<str>]>` `QueryKey` is not directly serde, and keying by `String` avoids `Arc` aliasing across processes). |
| `PersistOptions`    | `gpui-query` (client/persist.rs; **shipped**) | `{ filter: PersistFilter, max_age, debounce }`. Shipped without `dehydrate_filter`; `filter` is the owned `PersistFilter { Exact, Prefix, All }` (NOT `QueryKeyFilter<'a>`, which borrows and is not `Serialize`). |
| `CacheMeta`         | `gpui-query-http` (lib.rs; **shipped**) | `serde::{Serialize, Deserialize}`; crosses into the persist crate only as opaque `serde_json::Value`.            |
| `HttpCache`         | `gpui-query-http` (cache.rs; **shipped**) | Generic over the `HttpBackend` trait (`HttpCache<B: HttpBackend>`); keys are `String`. `reqwest` is one optional backend behind the `reqwest` feature. |
| `FilePersister`     | `gpui-query-persist` (lib.rs; **shipped**) | Reference disk adapter. `{ path, format, write_lock }`; no `app_name` field (folded into the resolved path by `in_cache_dir`). |
| `SqlitePersister`   | consuming app (e.g. `gpui-app`)     | App-specific; depends on the app's storage layout. Lives outside this crate.                                       |

`CacheMeta` never crosses into core or a persist crate as a typed value — only as
`serde_json::Value`. Today the only typed values that cross a crate boundary are
`CachePolicy` and `DehydratedEntry`; `QueryPersister` is the trait that defines
the boundary. The async `Persister`, `PersistedEntry { value }`,
`persist_with`, debounce/filter, and versioning have all shipped behind the
`persist` feature — they extend the shipped synchronous skeleton, not replace it.

---

## Required Core Changes

These are **additions and enhancements to `gpui-query` core** that the two
companion crates depend on. All three have shipped (see [Implementation
Status](#implementation-status)). Where a capability already shipped in a
different shape, the work is framed as an enhancement of that skeleton rather
than a fresh build. The legacy synchronous persistence skeleton still ships
unconditionally in the `client` layer; the richer value-carrying surface ships
behind the `persist` feature (offline-data only — see [Crate Layout](#crate-layout)).

### 1. Fetcher-returned `CachePolicy` (for HTTP "server wins")

**Problem.** The shipped primary hook is `use_query` in `hook/query_hooks.rs`. Its
fetcher is signal-always — `F: Fn(QuerySignal) -> Fut` — and returns only
`Result<T, E>`. (`use_query_unsignalled` is the backward-compatible `Fn() -> Fut`
variant.) `CachePolicy` is fixed at `use_query` / `fetch_query` call time (via
`QueryOptions` / the `cache_policy` argument). There is no way for a fetcher that
just read `Cache-Control: max-age=30` from a response to push that policy back
into the bucket — so HTTP semantics cannot override the caller's per-query policy.

**Status:** **Shipped.** Both `Fetched<T>` and the `use_query_with_policy` /
`fetch_query_with_policy` hooks exist today (see [Implementation
Status](#implementation-status)). The shipped shape realizes **Option A**, with
one simplification: `Fetched` takes a **single** type parameter `T` — the error
type `E` is not threaded through the wrapper. `E` still travels via the
fetcher's `Result<Fetched<T>, E>` return, so no `PhantomData<E>` is needed.

**Fix — pick one (Open Question 1):**

- **Option A (typed wrapper):** the shipped `Fetched<T>` result type
  (`crates/gpui-query/src/core/fetched.rs`):

  ```rust
  pub struct Fetched<T> {
      pub data: T,
      pub cache_policy: Option<CachePolicy>,  // None → keep caller's policy
      #[cfg(feature = "persist")]
      pub meta: Option<serde_json::Value>,    // CacheMeta round-trip via persist
  }
  ```

  Note there is **no** `_error: PhantomData<E>` — `E` is carried by the fetcher's
  `Result<Fetched<T>, E>` return, so the wrapper is simpler and `PhantomData` is
  unnecessary. The shipped `use_query_with_policy` mirrors the `use_query`
  signature exactly — same `(Entity<QueryResource<T, E>>, Subscription)` tuple
  return, same `QuerySignal`-accepting fetcher shape — except the fetcher returns
  `Result<Fetched<T>, E>` and the bucket applies `cache_policy` on success. The
  existing `Result<T, E>` `use_query` keeps working unchanged (additive,
  non-breaking).

- **Option B (callback):** keep the fetcher as `Result<T, E>`, but give the
  fetcher closure an injected handle to the resource whose
  `QueryResource::set_cache_policy` (`core/resource/accessors.rs:134` — **already
  shipped**; same for `InfiniteQueryResource`) it can call (plus a `set_meta`
  equivalent). Only the handle plumbing is net-new; the setter itself exists
  today. More flexible, more state-passing boilerplate.

**Recommendation:** Option A — explicit, serializable, composes cleanly with
persistence. **Landed as `Fetched<T>`** (single type param, no `PhantomData`);
the `meta` field is `#[cfg(feature = "persist")]` and now persists through the
shipped value-carrying `PersistedEntry.meta` (Core Change 3 has shipped — see
[Implementation Status](#implementation-status)).

### 2. Whole-client mutation observation (for persistence subscription)

**Problem.** The shipped `Observer` in `client/observer.rs` (with
`QueryObserver<T, E>` / `InfiniteQueryObserver<T, E>` /
`MutationObserver<V, T, E>` aliases) observes a **single resource entity** and
calls `cx.notify()` only on status change. It does **not** observe whole-client
mutations. A persistence subscription needs to know when _any_ tracked entry
changes.

**Reality check — the reuse does *not* cover completions.** `QueryClient` is a
`Global` (`impl Global for QueryClient {}` in `client/mod.rs`), and in GPUI
mutating a Global via `cx.update_global::<QueryClient, _>` (or `global_mut` /
`set_global`) **does** auto-fire `cx.observe_global::<QueryClient>()` subscribers
(the update pushes a `NotifyGlobalObservers` effect — verified in
`gpui-0.2.2/src/app.rs`, `end_global_lease`). **But** the crate's
`update_global::<QueryClient, _>` sites all run at **fetch-start / hook-setup**,
never at completion: they wrap `resource_with_policies` /
`infinite_resource_with_policies` (resource creation), `next_request_id_for_key` /
`next_request_id_for_infinite_key` (request-id allocation), and `register_mutation`
(`hook/query_hooks.rs`, `hook/fetch_retry.rs`, `hook/use_infinite_query/*`,
`hook/mutation_hooks/hooks.rs`). The actual **completion** writes —
`complete_success` / `complete_failure` — are bare `entity.update(...)` +
`cx.notify()` on the resource entity (`hook/fetch_retry.rs:164`,
`hook/mutation_hooks/internals.rs:89`, `hook/use_infinite_query/fetch_runners.rs:97`).
`cx.notify()` inside `entity.update` wakes only `cx.observe(&entity)` subscribers,
**not** `observe_global`. So a `persist_with` on `observe_global::<QueryClient>()`
fires at fetch-start (before the data lands; and `dehydrate()` emits only `Success`
entries, so there is nothing to persist yet) and **does not fire** when the fetch
resolves. This is observable in practice: gpui-app's Query DevTools dashboard is
the only consumer that wires `observe_global::<QueryClient>` for live updates
(`observe_global_in`, no polling), and it refreshes on query creation/start and
manual toolbar mutations but **not** when a query resolves.

**The real gaps:** the three completion paths above. Each writes via bare
`entity.update(...)` and never notifies the global, so a global observer misses
exactly the writes persistence cares about. `QueryClient::set_query_data`
(`client/mod.rs`) is **not** one of them: it is `&mut self` on a `Global`, so every
caller must invoke it through `cx.update_global::<QueryClient, _>` (or `global_mut`)
— both of which push `NotifyGlobalObservers` — so seed/prime writes (hydration,
optimistic updates) already wake a global observer for free. An earlier draft had
this backwards.

**Fix — pick one (Open Question 2):**

- **Option A (coarse global observer):** `persist_with` subscribes via
  `cx.observe_global::<QueryClient>()`. For this to capture fetched data, **every
  completion write must notify the global** — route the `complete_success` /
  `complete_failure` `entity.update` blocks in `hook/fetch_retry.rs`,
  `hook/mutation_hooks/internals.rs`, and `hook/use_infinite_query/fetch_runners.rs`
  through `update_global::<QueryClient, _>`, or add an explicit global notify after
  each. The `PersistOptions::filter` (an owned filter, see below) and `debounce` do
  the selection work. This is **not** free reuse: it touches all three completion
  sites (creation/start and `set_query_data` already notify), and the observer
  still fires spuriously on every fetch-start/creation.
- **Option B (typed dirty signal):** add a dedicated dirty signal emitted from
  the same three completion-site families (plus `set_query_data`), observed by
  `persist_with`. More precise — fires only on real data changes, not on
  fetch-start — for slightly more core surface. **Shipped as the marker
  `gpui::Global` `CacheMutation`** (`crates/gpui-query/src/client/mutation_signal.rs`):
  bump sites call `cx.default_global::<CacheMutation>()` (which, like
  `set_global` / `global_mut`, unconditionally pushes GPUI's
  `NotifyGlobalObservers` effect) from the three completion-site families —
  `hook/fetch_retry.rs`, `hook/use_infinite_query/fetch_runners.rs`,
  `hook/mutation_hooks/internals.rs` — plus `QueryClient::set_query_data`
  (`client/mod.rs`). `persist_with` observes it via
  `cx.observe_global::<CacheMutation>()`.

**Recommendation:** Option B — **landed as `CacheMutation`.** The earlier
"Option A is free reuse" rationale did not hold — the completion sites had to be
touched either way (Option A misses completions entirely until they are), so the
only question was whether the subscriber is coarse-and-noisy (A, also fires on
every fetch-start/creation) or precise (B, fires only on completion +
`set_query_data`). Given equal completion-site work, B won; the shipped
`CacheMutation` marker realizes it.

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

**The real gaps (all shipped on top of this skeleton — see [Implementation
Status](#implementation-status)):**

1. **Async trait.** The legacy trait is synchronous. The shipped async `Persister`
   (`client/persist.rs`) uses `impl Future<...> + Send` return types and runs on
   GPUI's `background_executor`; it is **not** object-safe, so it favors generics
   (`persist_with<P>`) over `Box<dyn>`. (Open Question 5 covers a `Pin<Box<...>>`
   fallback for runtime dispatch.) It is an additive layer alongside the legacy
   trait, not a replacement.
2. **Data carriage.** The legacy snapshot has no `value` / `meta` field. The
   shipped `PersistedEntry { value, cached_at, cache_policy, meta }`
   (`client/persist.rs`) and the `collect_persist_snapshot` path let
   `gpui-query-persist` store actual cached data (and `CacheMeta` as an opaque
   `serde_json::Value`, see [Shared Types](#shared-types)). This unblocked the
   `Fetched.meta` field in Core Change 1.
3. **`persist_with`.** A subscription-style method that observes the
   `CacheMutation` marker Global (per Core Change 2), debounces, and calls
   `save`. The legacy `persist` one-shot still ships.
4. **Real `hydrate`.** A typed-hydration free function
   (`hydrate::<P>(client, persister, filter, max_age, cx)`) that loads entries and
   primes each one via the existing `QueryClient::set_query_data::<T, E>` in
   `client/mod.rs`, driven by the deserializer registry. The legacy `hydrate`
   remains a no-op — typed re-priming is the new function, not a modification of
   the legacy method.
5. **`PersistOptions` (filter / `max_age` / `debounce`) and `PersistSnapshot`
   versioning.** Both shipped. `PersistOptions::filter` is the owned
   `PersistFilter { Exact, Prefix, All }` — it does **not** reuse
   `QueryKeyFilter<'a>` (that type borrows a `&'a QueryKey` and is not
   `Serialize`); `dehydrate_filter` did not ship (Open Question 4 remains future
   work). Versioning is `PERSIST_VERSION = 1` with **rejection** on mismatch
   (`PersistError::VersionMismatch`), not migration.

Detailed shape lives in [2. Persistence](#2-persistence--gpui-query-persist).
The richer API lands behind the `persist` feature (see [Crate
Layout](#crate-layout)).

---

## 1. HTTP Cache Helpers — `gpui-query-http`

> **Status:** This crate has **shipped** as `crates/gpui-query-http`
> (`gpui-query-http` on crates.io). The API below reflects the shipped types in
> `crates/gpui-query-http/src/{lib.rs,cache.rs,backend.rs}`; the original design
> prose is preserved but corrected where it predates the library-agnostic
> `HttpBackend` split. `gpui-app` remains an external consumer, not a repo member.
> The usage example targets the **real** `use_query` hook (signal-always,
> options-first, tuple return) and the shipped `use_query_with_policy` /
> `Fetched::with_policy` server-wins path.

### Scope

Per-fetch concern. Parse HTTP cache headers, attach conditional request headers
on refetch, and on `304 Not Modified` return the cached body. The shipped crate
is **library-agnostic**: it is generic over an `HttpBackend` trait rather than
hardcoded to `reqwest`. The real (non-optional) deps are `http` and `bytes`;
`reqwest` is **one optional backend** behind the crate's `reqwest` cargo feature
(`ReqwestBackend`), never a hard dependency. All three belong to
`gpui-query-http` only — `gpui-query` core ships with **none** of them (none
appears in `crates/gpui-query/Cargo.toml` nor anywhere under
`crates/gpui-query/src`), consistent with Guiding Principle 1.

### Public API

```rust
// crates/gpui-query-http/src/lib.rs   (SHIPPED — see backend.rs / cache.rs too)

use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};

/// HTTP cache metadata extracted from a response. Serializable so the
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

// backend.rs — library-agnostic conditional-GET trait (NOT reqwest-specific).
pub trait HttpBackend: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    fn fetch(
        &self,
        url: &str,
        conditionals: Conditionals,
    ) -> impl Future<Output = Result<BackendResponse, Self::Error>> + Send;
}

// cache.rs — the URL-keyed cache. Generic over the BACKEND TRAIT, not a client.
//   - reqwest is ONE optional backend (ReqwestBackend) behind the `reqwest`
//     cargo feature; it is never a hard dependency.
//   - Map keys are `String` (not `reqwest::Url`), so the cache is usable with
//     any backend that accepts a URL string.
pub struct HttpCache<B: HttpBackend> {
    backend: B,
    meta: std::sync::Mutex<HashMap<String, CacheMeta>>,
    bodies: std::sync::Mutex<HashMap<String, bytes::Bytes>>,
}

impl<B: HttpBackend> HttpCache<B> {
    pub fn new(backend: B) -> Self;

    /// Fetch a URL. On 200, stores body + meta and returns `(body, policy, meta)`.
    /// On 304, returns the cached body, the stored policy, and the stored meta.
    /// On a non-cacheable response, returns `(body, CachePolicy::NoCache, None)`.
    ///
    /// The std `Mutex`es are **never held across an `.await`** — each critical
    /// section is lock/read/drop-lock, then the async request runs, then a second
    /// lock/write/drop-lock. This keeps `HttpCache: Send + Sync` without a tokio
    /// mutex and lets it run on any executor.
    pub async fn fetch(
        &self,
        url: &str,
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
use gpui_query::{use_query_with_policy, QueryKey, QueryOptions};
use gpui_query::core::{CachePolicy, Fetched, RequestPolicy, QuerySignal};

#[derive(Clone, serde::Deserialize)]
struct User { /* ... */ }

let url = format!("https://api.example.com/users/{id}");
let http = http_cache.clone(); // Arc<HttpCache<B>>

// The per-query policy is set on the QueryOptions builder as the *caller's*
// default; the fetcher can override it per-response via Fetched::with_policy
// (server wins, see below).
let options = QueryOptions::new(QueryKey::from(["users", &id.to_string()]))
    .cache_policy(CachePolicy::Ttl { ttl_ms: 60_000 })
    .request_policy(RequestPolicy::LatestWins);

let (entity, _subscription) = use_query_with_policy(
    options,
    move |_signal: QuerySignal| async move {
        let (bytes, policy, _meta) = http.fetch(&url).await?;
        let user: User = serde_json::from_slice(&bytes)?;
        // Server wins: adopt the CachePolicy derived from the response headers.
        Ok(Fetched::with_policy(user, policy))
    },
    cx,
);
```

> **Server-wins is shipped.** The fetcher above feeds `_policy` and `_meta` back
> into the resource via the shipped `use_query_with_policy` /
> `fetch_query_with_policy` hooks (`hook/query_hooks.rs`), which accept a fetcher
> returning `Result<Fetched<T>, E>`. Wrap the data in
> `Fetched::with_policy(data, policy)` (or `Fetched::with_meta(...)` under the
> `persist` feature) and the resource adopts the server's `CachePolicy`
> immediately after `complete_success`. `Fetched::new(data)` keeps the caller's
> policy. See [Required Core Change 1](#1-fetcher-returned-cachepolicy-for-http-server-wins)
> and `crates/gpui-query/src/core/fetched.rs`.
>
> (Historical note: an earlier draft of this section stated server-wins was "not
> yet possible" because `Fetched` and the `*_with_policy` hooks did not exist.
> Both have since shipped.)

### Design notes

- No middleware, no layer trait. There is nothing to swap at the HTTP layer —
  but there *is* a backend to swap (any HTTP client), which is why the shipped
  `HttpCache<B: HttpBackend>` is generic over the `HttpBackend` trait. The
  shipped `MockBackend` in `cache.rs` tests exercises this static-dispatch seam.
- `HttpCache` owns its own `HashMap<String, _>` of metadata and bodies. It does
  **not** touch `QueryClient`'s bucket. The `CachePolicy` + `CacheMeta` returned
  by `fetch` flow into `QueryClient` via the shipped `Fetched::with_policy` /
  `Fetched::with_meta` channel through `use_query_with_policy`, so HTTP semantics
  override the caller's per-query policy mid-flight (server wins). Required Core
  Change 1 has shipped.
- `CacheMeta` is `serde::{Serialize, Deserialize}` so the persistence layer can
  persist ETags for cold-start 304 refetches. This is no longer forward-looking:
  the shipped value-carrying `PersistedEntry.meta: Option<serde_json::Value>`
  (behind the `persist` feature) round-trips a serialized `CacheMeta` through
  `FilePersister` so a cold start after relaunch can send `If-None-Match`. (The
  older synchronous `QueryPersister` / `DehydratedEntry` skeleton in
  `client/erased.rs` / `client/devtools.rs` remains metadata-only —
  `{ key, type_id, kind }`, not serde — and does not participate in this path.)
- `HttpCache<B: HttpBackend>` is generic over the backend trait so tests inject a
  stub backend (static dispatch, per `rust-best-practices` ch.6). The shipped
  `reqwest` feature supplies `ReqwestBackend` for production; the default build
  has no `reqwest` dependency at all.

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
a (legacy no-op) hydration hook. The richer design below — typed data in entries,
an async persister, debounced auto-save, filters, max-age, and version rejection
— is the **evolution** of that skeleton.

> **Update (shipped):** this evolution has landed behind the `persist` feature —
> see [Implementation Status](#implementation-status). The historical framing
> below is preserved for the design rationale. (1) `DehydratedEntry` still
> carries **no data** by design — the value-carrying path uses the new
> `PersistedEntry` instead. (2) The legacy `QueryClient::hydrate` remains a
> no-op; typed re-priming now happens through the new free-function `hydrate<P>`
> and the deserializer registry. The two gaps that motivated this work are
> closed by the richer surface, which coexists with the synchronous skeleton.

### What ships today (`client` layer)

The legacy metadata-only skeleton is compiled whenever the `client` feature is
enabled — it is **not** behind the `persist` gate (the richer value-carrying
surface described below is what `persist` gates). The surface lives in
`client/erased.rs` and `client/devtools.rs`, and the `QueryClient` methods live
in `client/lifecycle.rs`.

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

### Proposed evolution (shipped — see [Implementation Status](#implementation-status))

The richer API below has **shipped** in `crates/gpui-query/src/client/persist.rs`
(behind the `persist` feature). The code block reflects the shipped shapes; it is
kept here as the design contract the companion crate targets.

```rust
// SHIPPED — core additions behind the `persist` feature flag.
//
// An async, debounced, filtered auto-save path built on top of the existing
// skeleton. The legacy QueryPersister stays synchronous; this richer shape is
// what the companion crate targets.

/// SHIPPED. Persistence backend with async load/save. Methods return
/// `impl Future<...> + Send` (not `async fn`) so the returned futures are
/// guaranteed `Send` and spawnable on GPUI's background_executor. This makes the
/// trait NOT object-safe; use generics (`persist_with<P>`) for static dispatch.
/// (The legacy QueryPersister stays synchronous and object-safe; this is an
/// alternative shape, not a replacement of it.)
pub trait Persister: Send + Sync + 'static {
    fn load(&self) -> impl Future<Output = Result<PersistSnapshot, PersistError>> + Send;
    fn save(&self, snapshot: &PersistSnapshot)
        -> impl Future<Output = Result<(), PersistError>> + Send;
}

/// SHIPPED. A dehydrated entry WITH a data payload — the field that
/// DehydratedEntry deliberately omits. The `value` is an opaque
/// serde_json::Value so core never needs to know the concrete (T, E); typed
/// de/serialization happens via the serializer/deserializer registries on
/// QueryClient.
pub struct PersistedEntry {
    pub value: serde_json::Value,
    pub cached_at: u64,                   // epoch-millis (SystemTime-based, like current_time_ms)
    pub cache_policy: CachePolicy,        // core's CachePolicy is Copy + Serialize
    pub meta: Option<serde_json::Value>,  // opaque, e.g. CacheMeta round-trip
}

/// SHIPPED. Keys are `String` (the `to_path()` form of `QueryKey`), NOT
/// `QueryKey` itself — `QueryKey` is an `Arc<[Arc<str>]>` and is not directly
/// serde, so keying by its owned path string keeps the snapshot self-contained
/// and serializable across processes.
pub struct PersistSnapshot {
    pub entries: HashMap<String, PersistedEntry>,
    pub version: u32,                     // PERSIST_VERSION = 1
}

/// SHIPPED.
pub struct PersistOptions {
    // Owned filter (NOT core's borrowing QueryKeyFilter<'a>, which is not
    // Serialize and cannot be stored here).
    pub filter: PersistFilter,            // enum { Exact(QueryKey), Prefix(QueryKey), All }
    pub max_age: Duration,                // default 24h
    pub debounce: Duration,               // default 500ms; ZERO disables the coalescing window
    // No `dehydrate_filter` shipped — Open Question 4 remains future work.
}

/// SHIPPED — drop guard that unsubscribes the debounced observer.
pub struct PersistHandle { /* holds the Subscription; drops to unsubscribe */ }

impl QueryClient {
    /// SHIPPED. Subscribe to the `CacheMutation` marker Global via
    /// `cx.observe_global::<CacheMutation>()` (see Required Core Change 2,
    /// Option B — realized), debounce, and call `persister.save`.
    pub fn persist_with<P: Persister>(
        &self, p: P, opts: PersistOptions, cx: &mut gpui::App,
    ) -> PersistHandle;
}

// SHIPPED — free function (not a QueryClient method). Loads, version-checks,
// filters by max_age/filter, and re-primes each entry on the gpui foreground
// thread via the deserializer registry → set_query_data. Returns the loaded
// snapshot (post-filter) for ad-hoc priming; errors propagate.
pub async fn hydrate<P: Persister>(
    client: &mut QueryClient,
    persister: &P,
    filter: &PersistFilter,
    max_age: Duration,
    cx: &mut gpui::App,
) -> Result<PersistSnapshot, PersistError>;
```

The `Persister` trait belongs in core because it is tiny (two methods), central,
and defines the contract every adapter must satisfy; the adapters do not. Note
the contrast with the legacy skeleton: `QueryPersister` is the synchronous,
object-safe version; the richer `Persister` above is the async, non-object-safe
version the companion crate targets. **Shipped: the two coexist** — the async
`Persister` is layered behind the `persist` feature and does not remove the
synchronous `QueryPersister`.

### Disk-based persistence

The headline adapter for `gpui-query-persist` is a **proper disk-based
`FilePersister`** — the normal case for offline data caching. It must work
identically on Linux, macOS, and Windows and survive the usual failure modes
(crash mid-write, corrupt file, permission errors, schema drift):

- **OS-appropriate location.** When the caller does not pass an explicit path,
  resolve a per-app directory with the `dirs` crate and join `<app>` — never
  hardcode a path or separator, always go through `PathBuf`, and never `unwrap`
  the dir (it is `None` in sandboxed/headless envs — return `PersistError::BadPath`
  instead). **Shipped default: `dirs::cache_dir()`** (Open Question 8 resolved to
  `cache_dir`, not `data_dir`) — `~/.cache`, `~/Library/Caches`, `%LOCALAPPDATA%`
  — because the offline cache is regenerable, and `cache_dir()` avoids a Windows
  Roaming profile silently syncing it across machines. (`dirs::data_dir()` was the
  alternative; see Open Question 8 for the trade-off discussion.) Under the macOS
  App Sandbox, `cache_dir()` resolves inside the per-app container; an
  absolute-path override outside the container needs entitlements. The caller can
  always pass an explicit absolute path via `FilePersister::new` / `::json` /
  `::bincode`.
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

### Companion crate (shipped)

```rust
// SHIPPED — crates/gpui-query-persist/src/lib.rs
//
// A real disk-based FilePersister (see "Disk-based persistence" above) plus the
// types it needs. This is the reference adapter; the doc-snippet FilePersister in
// client/erased.rs is only a trait-example for the synchronous skeleton.
pub enum PersistFormat { Json, Bincode }

pub struct FilePersister {
    path: PathBuf,       // explicit, or resolved from dirs::cache_dir() + app_name
    format: PersistFormat,
    write_lock: std::sync::Mutex<()>,  // serializes concurrent saves
}

impl FilePersister {
    pub fn new(path: impl Into<PathBuf>, format: PersistFormat) -> Self;
    pub fn json(path: impl Into<PathBuf>) -> Self;
    pub fn bincode(path: impl Into<PathBuf>) -> Self;
    /// Resolve `<dirs::cache_dir()>/<app_name>/gpui-query-cache.json`. There is
    /// NO `app_name` field on the struct — `app_name` is folded into the
    /// resolved path here and discarded. Returns `BadPath` when the OS reports
    /// no cache dir. (Open Question 8 resolved to `cache_dir`, not `data_dir`.)
    pub fn in_cache_dir(app_name: impl AsRef<str>) -> Result<Self, PersistError>;
}

impl Persister for FilePersister { /* atomic write + tolerant load, as above */ }

pub use gpui_query::client::NoopPersister;  // re-exported for tests / in-memory
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
migrations. `FilePersister` is the reference adapter that ships in
`gpui-query-persist`; `SqlitePersister` is the app-specific adapter that ships
with the app.

### Hydration sequence at startup

The legacy `QueryClient::hydrate` remains a no-op; the working typed restore path
is the free function `hydrate::<P>(client, persister, filter, max_age, cx)`
(`client/persist.rs`), which loads the snapshot, version-checks it, and re-primes
each entry whose `(T, E)` has a registered deserializer via
`QueryClient::set_query_data`. The sequence below sketches how a consuming app
wires it at startup.

A consuming app already bridges tokio and gpui via its own runtime; treat any such
bridge as an example of a generic external consumer, not as part of this crate.

```
app startup (App context)
  │
  ├── cx.set_global(QueryClient::new(...))
  ├── cx.spawn(async move |cx| {
  │     // load() runs as async IO (on GPUI's background_executor for the
  │     // shipped FilePersister; on tokio for an app-supplied backend);
  │     // priming runs on the gpui foreground thread.
  │     let snapshot = hydrate:: <P>(&mut client, &persister, &filter, max_age, cx).await?;
  │     // begin observing mutations
  │     let _handle = client.persist_with(persister, opts, cx);  // observes CacheMutation
  │     // store _handle for app lifetime so the subscription is not dropped
  │ }).detach();
  ├── gpui window opens
  └── use_query hooks observe already-primed entries → render cached data
                                                    → refetch in background
```

`persist_with`'s `save()` hops the other way: the debounced observer callback runs
on the gpui foreground thread (it observes `CacheMutation`), collects the
snapshot, then spawns `persister.save(&snapshot)` on the background executor to
write without blocking the UI.

### Interaction with `gpui-query-http`

The one place these two crates cooperate: persisting `CacheMeta` so cold-start
refetches send `If-None-Match` and get cheap `304`s. The shipped
`PersistedEntry::meta` is typed `Option<serde_json::Value>` (opaque to core) so
`gpui-query-persist` does **not** need to depend on `gpui-query-http`. The HTTP
crate serializes `CacheMeta` into that `Value` on save and deserializes it back on
load. (`CacheMeta` is defined by `gpui-query-http`, not core.) Zero coupling
between the two companion crates; they share only the `serde_json::Value`
contract and the `CachePolicy` type from core.

This coupling boundary is sound and **now meaningful** — both `PersistedEntry`
and `CacheMeta` have shipped, so cross-crate data sharing works end-to-end.

### Design notes

- **No generic mutation hook.** `persist_with` is specific: it subscribes via
  `cx.observe_global::<CacheMutation>()` (the shipped marker Global, see Required
  Core Change 2) and calls one trait method. Adding a general `on_mutate` hook
  would invite misuse and bloat core. (Historical design context: an earlier
  Option A proposed observing `observe_global::<QueryClient>()` directly. That
  does fire automatically on any `cx.update_global::<QueryClient, _>`, but the
  crate routes only fetch-start / hook-setup through `update_global` — resource
  creation, request-id allocation, mutation registration — while completion writes
  are bare `entity.update` on the resource entity and do not notify that global.
  So Option A would have fired spuriously on fetch-start and missed every
  completion. The shipped `CacheMutation` marker is bumped explicitly at the three
  completion-site families + `set_query_data`, which is why it sees exactly the
  writes persistence cares about.)
- **Server wins.** `CachePolicy` stored in `PersistedEntry` is whatever the
  server last returned (via `Fetched::with_policy`, shipped — see Required Core
  Change 1). On rehydrate, that policy applies until the next fetch refreshes it.
- **Debounce is mandatory.** `PersistOptions::debounce` (**shipped**, default
  `Duration::from_millis(500)`) gates saves; passing `Duration::ZERO` disables
  the coalescing window but is allowed (each bump still races to drain the pending
  slot; saves still serialize through the drain slot).
- **Versioning.** `PersistSnapshot::version` (**shipped**, `PERSIST_VERSION = 1`)
  is bumped on every breaking change to `PersistedEntry`'s schema. `hydrate`
  **rejects** mismatched versions with a typed `PersistError::VersionMismatch`;
  there is **no migration path** (apps decide whether to wipe and continue or
  surface the error). Migration is future work.
- **`Send + Sync + 'static`** on `Persister` is required because `save` / `load`
  are spawned on GPUI's `background_executor` (`rust-best-practices` ch.9; the
  shipped `FilePersister` performs synchronous `std::fs` I/O inside its async
  bodies and is intended for that blocking-friendly pool).
- **The real gaps, restated.** Two things made the legacy skeleton unsuitable for
  real data persistence: `DehydratedEntry` had no data field (metadata only), and
  `QueryClient::hydrate` was a no-op. The shipped enrichment adds the data
  payload, the typed/async persister, the debounced subscription, and the
  version-rejecting snapshot on top of the existing synchronous `QueryPersister`
  / `dehydrate` / `persist` / `restore` plumbing.

---

## Integration Plan for a Consuming App

Concrete steps for wiring the companion crates into a consuming app. Both
companions have shipped (see [Implementation Status](#implementation-status)).
`gpui-app` is one example **external consumer** of this crate — it is not part of
this repo. Every path below (`Cargo.toml`, a tokio-runtime helper, a query
playground, a storage service) is illustrative of where *any* consumer would hook
things in, not a path in this crate.

Two things must be clear up front:

1. **A legacy persistence skeleton ships** with `gpui-query` (under the
   `client` feature, no separate feature flag): the synchronous, object-safe
   `QueryPersister` trait (`client/erased.rs`), `DehydratedEntry` /
   `DehydratedState` (`client/devtools.rs`), and
   `QueryClient::dehydrate` / `hydrate` / `persist` / `restore`
   (`client/lifecycle.rs`). The steps below distinguish *use the legacy skeleton*
   from *adopt the richer shipped API*.
2. **`gpui-query-http` and `gpui-query-persist` ship** as workspace members (see
   [Crate Layout](#crate-layout)). Steps that reference them are the shipped
   integration path.

### Step 0 — Use the shipped persistence skeleton (no companion crate required)

A consumer can persist query metadata across restarts *today*:

1. **`Cargo.toml`** — depend on `gpui-query` with the `client` feature (and
   `hook` if using hooks). The synchronous metadata-only skeleton ships with
   `client` (no flag needed); the richer value-carrying surface requires the
   `persist` feature, which has shipped.
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

> **Legacy-skeleton gap (closed by the shipped enrichment).** The legacy
> skeleton is metadata-only. `DehydratedEntry` has **no value/data field** (it
> was deliberately removed as dead weight; see its doc comment in
> `client/devtools.rs`), and `dehydrate()` emits no payload. So with Step 0
> alone, a cold restart restores *that keys existed* but not their data. Step 1
> (the shipped `persist` feature + `gpui-query-persist`) closes this gap.

### Step 1 — Adopt the richer persistence API (shipped)

The `gpui-query-persist` companion crate and its core changes have shipped (see
[Required Core Changes](#required-core-changes) and [Implementation
Status](#implementation-status)). The consumer upgrades:

1. **`Cargo.toml`** — add the `gpui-query-persist` dep and enable the `persist`
   feature on `gpui-query` (it gates the value-carrying persistence surface and
   pulls `serde_json` + `thiserror`; offline data only). Both ship today.
2. **Replace the manual `persist` / `restore` calls** with
   `QueryClient::persist_with(persister, opts)` (debounced observer on
   `CacheMutation`) and the free-function `hydrate::<P>(client, persister,
   filter, max_age, cx)` (typed re-priming via the deserializer registry). Both
   ship.
3. **`SqlitePersister`** (or equivalent) stays in the consumer, implementing the
   async `Persister` trait against the consumer's existing storage layout and
   migrations — it is not crate code. The shipped `FilePersister` is the
   reference disk adapter.

### Step 2 — Adopt the HTTP cache helper (shipped)

Both dependencies have landed: `gpui-query-http` ships in this repo, and
Required Core Change 1 (`Fetched<T>` + the `use_query_with_policy` /
`fetch_query_with_policy` hooks) is shipped. The real fetcher shape is
`use_query(options, fetcher: Fn(QuerySignal) -> Fut, cx) -> (Entity<QueryResource<T,E>>, Subscription)`
(see `hook/query_hooks.rs`); the server-wins variant is identical except the
fetcher returns `Result<Fetched<T>, E>`.

1. **The consumer's HTTP client wrapper** — wherever it owns a shared
   `reqwest::Client` (a tokio-runtime helper module), wrap it once in
   `HttpCache<ReqwestBackend>` (enable the `reqwest` cargo feature on
   `gpui-query-http`) and expose `Arc<HttpCache<ReqwestBackend>>` alongside the
   raw client. Any non-`reqwest` client can implement `HttpBackend` instead.
2. **Fetch sites** — replace raw `reqwest` bodies inside fetcher closures with
   `HttpCache::fetch`, and switch those fetchers to `use_query_with_policy` /
   `fetch_query_with_policy` returning `Result<Fetched<T>, E>` (wrapping the data
   in `Fetched::with_policy(data, policy)`) so the server-returned `CachePolicy`
   flows back (server wins).
3. **DevTools wiring** — wherever `QueryClient` is constructed today (per the
   consumer's devtools setup), the existing `QueryClient::diagnostics()` (method
   in `client/lifecycle.rs`; diagnostic *types* in `client/devtools.rs`) can
   surface a persistence section. A "Persistence" card in the consumer's
   playground (hydrate count on startup, last save timestamp, persisted key count)
   doubles as the integration-test surface.

---

## Migration Order

The order below is the **historical** migration order; all steps have shipped.
It is retained for the design rationale. The richer persistence API touches
core, and the HTTP helper depends on the `Fetched<T>` core change. A
metadata-only persistence skeleton already shipped before this work, so
"persistence" below means *upgrading* it, not building it from zero. All
persistence work — gating the richer surface, the async upgrade, and the disk
adapter — lands behind the `persist` feature (offline data only).

1. **Use the shipped skeleton** (no core change). Consumers can wire
   `QueryPersister` + `QueryClient::persist` / `restore` + caller-side typed
   `set_query_data` priming. This is metadata-only; verify with
   `#[gpui::test]` + `TestAppContext` (the crate's own tests already use this
   pattern — see `src/tests/`).
2. **Core Change 1 — `Fetched<T>` + `*_with_policy` fetcher variant**
   in `gpui-query` (**shipped**). Non-breaking: the existing `use_query` signature
   (signal fetcher, tuple return) is unchanged; the new variant is additive. Unit
   tests cover that `Fetched::with_policy(Some)` overrides the bucket's policy and
   `Fetched::new` (None) keeps it. (Note: single type param — `E` travels via the
   fetcher's `Result<Fetched<T>, E>` return, no `PhantomData`.)
3. **Feature-gate the richer surface, then ship it.** First land the three
   gating prerequisites (extract `current_time_ms` from `client/erased.rs`; decide
   `collect_key_status_into`'s fate; cfg-gate the itemized `pub use` lists in
   `client/mod.rs`) and move the value-carrying surface behind `persist`. Then add
   the async `Persister`, `PersistedEntry { value, ... }`, `persist_with`, typed
   `hydrate`, `PersistOptions { filter, max_age, debounce }`, and snapshot
   versioning on top of the shipped `QueryPersister` skeleton. `persist_with`
   needs Core Change 2 first: the completion writes (`complete_success` /
   `complete_failure` in `hook/fetch_retry.rs`, `hook/mutation_hooks/internals.rs`,
   `hook/use_infinite_query/fetch_runners.rs`) were bare `entity.update`s that did
   **not** notify the `QueryClient` global, so they now bump the shipped
   `CacheMutation` marker Global (Option B, realized) which `persist_with`
   observes via `observe_global::<CacheMutation>()`; `set_query_data` also bumps
   it (see Open Question 2). Ship a
   reference `FilePersister` in the `gpui-query-persist` crate. Unit tests cover
   debounce, `max_age` filtering, and version mismatch — these use
   `#[gpui::test]` + `TestAppContext` since they touch `QueryClient`.
4. **`gpui-query-http`** — **shipped** — with `CacheMeta`,
   `cache_policy_from_headers`, the `HttpBackend` trait, and `HttpCache<B>`.
   Header-parsing tests are plain `#[test]` (no gpui context); the `304` / SWR /
   `no-store` integration tests use a hand-rolled stub `HttpBackend`
   (`MockBackend`) in `#[tokio::test]`, **not** a `mockito` server. (A wire-level
   `mockito` suite could be added later for end-to-end coverage against a real
   HTTP server; the shipped stub-backend tests cover the cache logic without one.)
5. **Wire a consuming app** to both per the integration plan. Replace raw
   `reqwest` calls and add an app-specific persister.

---

## Test Strategy

Per `gpui-test`: tests that touch `QueryClient` (a `Global`) need
`#[gpui::test]` + `TestAppContext`; pure-logic tests (header parsing, debounce
math) are plain `#[test]`. This matches the crate's own test suite, which uses
`#[gpui::test]` + `TestAppContext` throughout `src/tests/` (with `setup_query_client`
/ `setup_test` helpers in `src/tests/test_support.rs`; the only top-level `tests/`
file is `tests/edge_cases.rs`).

| Target                                | Unit tests                                                | Integration tests                                  | GPUI ctx? |
| ------------------------------------- | --------------------------------------------------------- | -------------------------------------------------- | --------- |
| Shipped persistence skeleton          | `QueryPersister` load/save round-trip; `restore` returns entries | dehydrate → persist → restore → caller `set_query_data` priming | Yes       |
| Core 1 (`Fetched` / `*_with_policy`) — **shipped** | `Fetched` policy override, `None` keeps policy        | `*_with_policy` end-to-end                         | Yes       |
| Core 3 (upgraded persistence) — **shipped** | debounce coalescing, `max_age` filtering, version reject | hydrate → persist → hydrate round-trip           | Yes       |
| `gpui-query-http` — **shipped**       | header parsing edge cases                                 | hand-rolled stub `HttpBackend` (`MockBackend`) in `#[tokio::test]`: 200 / 304 / SWR / `no-store` | No |
| `gpui-query-persist` — **shipped**    | `FilePersister` round-trip, concurrent saves              | large snapshot, version **rejection** (NOT migration — no migration path is implemented; mismatched versions surface as `PersistError::VersionMismatch`) | No |
| Consuming app                         | —                                                         | playground section card asserts real flows         | Yes       |

Cross-crate integration test (lives in the consuming app's `tests/`, since only
the app wires both shipped crates together): HTTP fetch → `CacheMeta` stored in
`PersistedEntry::meta` → app shutdown → app restart → `hydrate` → next fetch
sends `If-None-Match` → server returns `304` → cached body served, no full
refetch. This one test exercises the entire design end-to-end.

---

## Open Questions

1. **Fetcher-returns-policy shape** — Option A (`Fetched<T>` wrapper) vs
   Option B (injected `QueryContext` callback)? Default was A, for explicitness
   and serializability. **Resolved: Option A shipped as `Fetched<T>`**
   (`crates/gpui-query/src/core/fetched.rs`), with the simplification that the
   wrapper takes a **single** type param `T` (no `PhantomData<E>` — `E` travels
   via the fetcher's `Result<Fetched<T>, E>`). See Required Core Change 1.
2. **Bucket-mutation observation** — Option A (`observe_global::<QueryClient>()`,
   coarse) vs Option B (typed dirty signal, precise)? Default was **B**.
   **Resolved: Option B shipped as the marker `gpui::Global` `CacheMutation`**
   (`crates/gpui-query/src/client/mutation_signal.rs`), bumped via
   `cx.default_global::<CacheMutation>()` at the three completion-site families
   (`hook/fetch_retry.rs`, `hook/use_infinite_query/fetch_runners.rs`,
   `hook/mutation_hooks/internals.rs`) plus `set_query_data` (`client/mod.rs`),
   and observed via `cx.observe_global::<CacheMutation>()` in `persist_with`.
   (Historical rationale preserved:) in GPUI, `cx.update_global::<QueryClient, _>`
   (and `global_mut` / `set_global`) **does** auto-notify
   `observe_global::<QueryClient>()` subscribers (pushes a
   `NotifyGlobalObservers` effect), and `default_global::<CacheMutation>()` does
   the same. But the crate's `update_global::<QueryClient>` sites all fire at
   **fetch-start / hook-setup** (resource creation, request-id allocation,
   mutation registration) — **not** at completion. The completion writes
   (`complete_success` / `complete_failure`) were bare `entity.update`s that
   notify only the resource entity, so `observe_global::<QueryClient>` did
   **not** see them — which is why the dedicated `CacheMutation` marker is bumped
   explicitly at those sites. (`set_query_data` is the opposite of a gap: being
   `&mut self` on a `Global`, callers reach it via `update_global`, which
   notifies — and it also bumps `CacheMutation` directly.) Option B's precision
   won because it avoids the spurious fetch-start/creation notifications Option A
   would deliver before data exists.
3. **Adding a typed value field to the shipped metadata-only entry.** The shipped
   `DehydratedEntry` is `{ key, type_id, kind }` with **no value field** (it was
   removed as dead weight — see its doc comment in `client/devtools.rs`), and
   `dehydrate()` emits no payload. The real open question was how to **add** a
   typed value: a new `serde_json::Value` field (flexible, opaque to core, slower)
   vs a generic `T: Serialize + DeserializeOwned` path (typed, faster, more
   bounds). **Resolved: `serde_json::Value` payload** on a new `PersistedEntry`
   (`crates/gpui-query/src/client/persist.rs`), with typed de/serialization driven
   by serializer/deserializer registries on `QueryClient` (so no `T: Serialize`
   bound leaks onto the resource). The metadata-only `DehydratedEntry` is
   unchanged.
4. **Per-key persistence policy** — the shipped `PersistOptions::filter` is
   global (an owned `PersistFilter { Exact, Prefix, All }`). If apps need per-key
   TTLs or per-key serializers, extend `QueryKeyFilter` (in `core/key_filter.rs`)
   — noting its `<'a>` borrow and lack of `Serialize` — or add a `PersistRule`
   enum. Defer until a concrete need exists.
5. **Object safety of the async `Persister`.** Note: the shipped
   `QueryPersister` is **synchronous and object-safe** — it is used as
   `&dyn QueryPersister` by `QueryClient::persist` and `QueryClient::restore` (see
   `client/lifecycle.rs`). **Resolved: the async `Persister` shipped as
   non-object-safe** (`impl Future + Send` return types; `Box<dyn Persister>`
   does not compile), consumed generically via `persist_with<P: Persister>`. If
   runtime dispatch is ever needed (dev vs prod persister swap), switch to
   `Pin<Box<dyn Future<...> + Send>>`.
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
   default if the store is treated as non-regenerable user data. **Resolved:
   `cache_dir()`** — `FilePersister::in_cache_dir(app_name)` resolves via
   `dirs::cache_dir()` (`crates/gpui-query-persist/src/lib.rs`), joining
   `<app_name>/gpui-query-cache.json`.
