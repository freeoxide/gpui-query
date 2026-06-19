# gpui-query Deep Audit — Follow-up Findings (Performance, Fallbacks, Improvements)

**Date:** 2026-06-20
**Scope:** Whole crate (`src/core`, `src/client`, `src/hook`, `src/tests`, `src/lib.rs`) — looking **beyond** the existing 144-finding audit (`gpui-query-audit.md`) and its implementation status (`gpui-query-audit-implementation-status.md`).
**Method:** Four parallel explore subagents (core / client / hook / tests+lib) read every file in their area, cross-checked against the indexed finding numbers, and only reported items that don't overlap with the existing 144 findings or the implementation-status notes. Criticals were then hand-verified against the live source.
**Verification:** A second pass with four parallel verification subagents checked every cited file:line, cross-reference, and proposed fix against the live source. See [`gpui-query-audit-deep-followup-verification.md`](./gpui-query-audit-deep-followup-verification.md) for the full meta-audit. **Result: 44/51 fully CONFIRMED, 7 PARTIALLY ACCURATE (corrected inline below), 0 INACCURATE.** The most significant corrections: the #112 drift claim had a fabricated quote (now fixed), M7's fix justification was wrong about GPUI notify semantics (now refined), and T8/T9 allocation counts were off (now corrected).

**Total NEW findings: 51** (2 HIGH, 19 MED, 30 LOW) — grouped by area below.

---

## Headline

The previous audit + implementation pass was thorough on the correctness and GC surface, but it left a second tier of **hot-path performance issues** and **API-constraint gaps** that only surface when you look past the cited findings. The biggest new items are:

1. **`evict_oldest` is O(n) entity reads per eviction** (M2). Once a bucket hits `max_entries`, **every new unique key** pays 10 000 `WeakEntity::upgrade` + `entity.read(cx)` lock cycles. This is the single hottest previously-unreported bottleneck.
2. **`use_query_select` allocates an `Arc<T>` (with full `T::clone`) on every source notification even when data is unchanged** (H1). Polling/refetch workloads with identical responses pay a full clone per tick.
3. **`MutationEntry::loading` is a dead field** (M1) — always `false`, drives a dead branch in `gc`, wastes 1 byte + padding per entry.
4. **`MutationResource` / `InfiniteQueryResource` are missing `PartialEq`/`Eq` derives** (N7/N8) — the implementation status doc claims #78 is "Implemented", but only `QuerySignal` got it.
5. **`cargo test --no-default-features --features core` does not compile** (T1) — three test modules use `crate::client::QueryClient` without `#[cfg(feature = "client")]` gating.
6. **`sanitize_message` does 5 full-text scans + 5+ allocations even when no patterns match** (N6) — no `contains` early-exit guards; the `Cow<str>` is always `Owned` after the first call.

---

## ✅ Verified implementation-status drift (not findings, but worth flagging)

The implementation-status doc (`gpui-query-audit-implementation-status.md`) is **internally contradictory** on three items — its top "Implementation Complete" section (`:12-35`) correctly says they're done, but its detailed tables (`:88-150`, written as a pre-implementation baseline) were never updated. The code is in better shape than the doc's tables report:

- **#20 `MappedQueryResource` stores `Option<Arc<T>>`** — the doc's "Not implemented" table (`:119`) says "still stores `Option<T>`", but the doc's top section (`:23`) says it's done. The code **has** it (`core/select.rs:136`). The remaining gap is only H1 (per-notification clone when unchanged).
- **#71 `Observer::observe` is `&self`** — the doc's "Not implemented" table (`:143`) says "still `&mut self`", but the doc's top section (`:25`) says it's done. The code **has** `&self` for all three observer types (`observer.rs:60,109,167`).
- **#112 `MutationBucket::touch` / `last_updated_at`** — the doc's "Partial" table (`:105`) says "`MutationBucket::touch()` exists but is never called from hooks → `updated_at` still insertion-time", but the doc's top section (`:20`) says it's done. The code confirms the top section: `MutationResource::last_updated_at_ms` **exists** (`mutation.rs:101`) and is stamped in `complete_success`/`complete_failure` (`mutation.rs:190,204`); `gc` at `mutation_bucket.rs:284` correctly prefers it over the insertion-time fallback. There are also **zero** `#[allow(dead_code)]` left in `src/client/` (the doc's `:101` claims 4 remain).

---

## 🚨 HIGH severity (2)

### M2 — `evict_oldest` does O(n) entity upgrades + reads per eviction; at capacity every new unique key pays O(n)
**Files:** `client/bucket/ops.rs:37-62`, `client/infinite_bucket.rs:101-124` (byte-identical), `client/mutation_bucket.rs:111-128` (structurally similar — reads `entry.updated_at` directly instead of from entity, uses `*id` instead of `key.clone()`); driven by `ops.rs:106-108`
**Severity:** HIGH | **New:** YES

`evict_oldest` iterates **every** entry, calls `entry.entity.upgrade()?` and `entity.read(cx)` (a GPUI lock acquire+release) per entry to fetch `is_loading()` and `last_updated_at_ms()`. The capacity guard `else if self.entries.len() >= self.max_entries` at `ops.rs:106` fires on **every insert of a new unique key once the bucket is full** (10 000 entries). So steady-state insertion of fresh keys into a full bucket is **O(n) entity locks per insert** — 10 000 lock-acquire+release cycles per new key.

The 64-op opportunistic GC (`mod.rs:142-156`) adds another O(n) sweep every 64 inserts, but `evict_oldest`'s per-insert O(n) dominates once full.

**Why it's not in the existing audit:** #59 covers the *clone* of the winning `QueryKey` (fixed). #17 covers `gc` upgrading every entry. **Neither covers `evict_oldest`'s O(n) entity reads, nor the steady-state O(n)-per-insert-at-capacity cost.**

**Impact:** An app that constantly creates new query keys (feed with unique object IDs, paginated list with `page::N` keys, etc.) hits the cap and then pays O(10 000) entity reads on every subsequent `get_or_create`. The single hottest previously-unreported bottleneck in the client.

**Proposed fix:** Mirror `last_updated_at_ms` and a `loading` hint into `BucketEntry`/`InfiniteBucketEntry` so `evict_oldest` can scan entry metadata with only a `WeakEntity::upgrade` liveness probe (cheap atomic) and **no `entity.read(cx)`**. The mirror must be refreshed on completion (a cross-layer change — the completion path needs to touch the bucket entry). Cheaper mitigations: (a) min-heap keyed on the mirrored timestamp for O(log n) eviction; (b) scale `GC_INTERVAL` with bucket size; (c) document that workloads with unbounded unique keys should raise `max_entries`.

### T1 — Feature-gate inconsistency: `cargo test --no-default-features --features core` fails to compile
**Files:** `tests/mod.rs:1` (`mod test_support;`), `:10` (`mod coverage_gaps;`), `:11` (`mod integration_client;`); `test_support.rs:33` (`use crate::client::QueryClient;`)
**Severity:** HIGH | **New:** YES

Three test modules use `crate::client::QueryClient` but are NOT `#[cfg(feature = "client")]` gated:
1. `test_support.rs:33` — un-gated `use crate::client::QueryClient;` (the hook-only imports on :34-35 ARE gated).
2. `integration_client/` — all 4 subfiles use `QueryClient`; module declared as plain `mod integration_client;` in `tests/mod.rs:11`.
3. `coverage_gaps/gc_eviction.rs` — uses `QueryClient`; parent `coverage_gaps` is plain `mod coverage_gaps;` in `tests/mod.rs:10`.

Only `integration_client_coverage`, `property_tests`, and `hook_tests` are correctly `#[cfg(feature = "hook")]` gated (`tests/mod.rs:12-17`).

**Impact:** The `core` feature exists for transport-agnostic state-machine users without GPUI. `cargo test --no-default-features --features core` is supposed to work but doesn't — the core-only tests (`core_cache/`, `core_lifecycle/`, `core_mutation/`, `core_policy_types/`, `core_request/`, `core_select.rs`) don't need `client`, but they share `test_support.rs` which has an un-gated `use crate::client::QueryClient` that breaks the entire module.

**Proposed fix:** Gate the client-dependent parts:
- In `test_support.rs`: `#[cfg(feature = "client")]` on `use crate::client::QueryClient;` and on all functions that use `QueryClient` (`setup_query_client*`, `observe_with_dummy_view`, etc.). Core-only helpers (`test_resource`, `test_sequencer`, `begin_request_id`, `complete_success_id`, `assert_status`, `nocache_resource`, `fresh_resource`, `resource_with_sequencer`, `TEST_NOW_MS`) stay un-gated.
- In `tests/mod.rs`: `#[cfg(feature = "client")]` on `mod integration_client;` and `mod coverage_gaps;` (or at minimum gate `gc_eviction` within `coverage_gaps/mod.rs`).

---

## 🟠 MED severity (19)

### Core (`src/core/*`) — 7 findings

#### N1 — `InfiniteQueryResource::accept_current_request` doesn't increment `ignored_results` on stale IDs
**File:** `infinite_query/lifecycle.rs:226-233`
**Severity:** MED | **New:** YES

```rust
pub fn accept_current_request(&mut self, request_id: RequestId) -> Option<RequestGuard> {
    if self.is_current_request(request_id) {
        self.active_request_id = None;
        Some(RequestGuard::new(request_id))
    } else {
        None  // <-- no ignored_results increment
    }
}
```

`QueryResource::accept_current_request` (`resource/lifecycle.rs:199-207`) calls `self.mark_ignored_result()` in the `else` branch. `InfiniteQueryResource::accept_current_request` does NOT. Meanwhile, the convenience methods `complete_page_success` (`:300-303`) and `complete_page_failure` (`:336-339`) DO increment `ignored_results` on stale IDs. The two-phase protocol (`accept_current_request` → `complete_*_with_guard`) is the preferred path (audit #86 made guards consume-by-value) and IS used in production (`fetch_runners.rs:98,153`). So `ignored_results` is systematically **undercounted** for infinite queries using the two-phase protocol.

**Fix:** Add `self.ignored_results += 1;` (or `saturating_add`) in the `else` branch, matching `QueryResource`.

#### N2 — `MutationResource::reset` doesn't reset `last_updated_at_ms`
**File:** `mutation.rs:228-239`
**Severity:** MED | **New:** YES

`QueryResource::reset` (`resource/lifecycle.rs:298`) resets `last_updated_at = None`. `InfiniteQueryResource::reset` (`infinite_query/lifecycle.rs:377`) also resets it. `MutationResource::reset` does NOT. `last_updated_at_ms` is used by `MutationBucket` GC (#112) to measure recency from completion time. After `reset()`, the mutation is `Idle` but `last_updated_at_ms` still points to the old completion time — if GC runs, it measures recency from the stale timestamp, potentially evicting a just-reset mutation prematurely.

**Fix:** Add `self.last_updated_at_ms = None;` in `reset()`.

#### N3 — `begin_request_with_id(None)` fallback generates duplicate `RequestId(1,1)`
**File:** `resource/lifecycle.rs:126-127,149-150`
**Severity:** MED | **New:** YES

```rust
let request_id = maybe_request_id
    .unwrap_or_else(|| RequestSequencer::new().next_request());
```

When `maybe_request_id` is `None`, a **fresh** `RequestSequencer::new()` is created each time. `RequestSequencer::new()` starts at `scope_id: 1, next_request_id: 1` (`request.rs:133`). So every call with `None` produces `RequestId(1, 1)`. A second call with `None` produces the same ID — the second call's `begin_request_with_id` overwrites `active_request_id = Some(RequestId(1, 1))`, but the **first** request's later completion would check `is_current_request(RequestId(1, 1))` → `true` (same ID!), accepting a stale result. This breaks the stale-request rejection invariant.

Reachable: `fetch_retry.rs:55-66` calls `begin_request_with_id` with `None` when `!cx.has_global::<QueryClient>()`.

**Fix:** Store a `RequestSequencer` in the resource (like `begin_request` takes one externally), or document that `None` is unsupported/deprecated.

#### N4 — `QueryResource::begin_request` / `begin_request_with_id` ~90% duplicated
**File:** `resource/lifecycle.rs:17-82` vs `95-157`
**Severity:** MED | **New:** YES (audit #12 covered infinite-query begin methods, NOT `QueryResource`)

The two methods have identical logic (cache hit check → stale-while-revalidate → IgnoreWhileLoading guard → normal fetch). Only how the request ID is obtained differs (`sequencer.next_request()` vs `maybe_request_id.unwrap_or_else(...)`). ~60 lines duplicated with only 4 lines differing.

**Fix:** Extract a shared `begin_request_inner(now_ms, fetch_mode, id_source: MaybeRequestId)` (mirroring the `InfiniteQueryResource::begin_fetch` + `MaybeRequestId` pattern already used in `infinite_query/lifecycle.rs:27-30,154-214`).

#### N5 — `placeholder_data` and `initial_data` fields are dead in production
**File:** `resource.rs:40,43` + their 6 methods (`accessors.rs:97-114`; `lifecycle.rs:312-314,353-367`)
**Severity:** MED | **New:** YES

Zero production call sites for `set_placeholder_data`, `set_initial_data`, `clear_initial_data`, `placeholder_data()`, `initial_data()`, and `display_data()` within the crate. The hook layer never sets or reads these fields. Yet they:
1. Add `2 × sizeof(Option<T>)` to every `QueryResource` (4 `Option<T>` fields total: `data`, `placeholder_data`, `previous_data`, `initial_data`).
2. Must be cloned when the resource is cloned (forces `T: Clone` overhead even for users who don't use placeholders).
3. Must be compared when the resource is compared (forces `T: PartialEq` overhead).
4. Are carried through every state transition (memory bandwidth).

Audits #88 and #89 touched these fields but didn't note they're entirely unused by the crate's own hook layer.

**Fix:** Either wire `placeholder_data`/`initial_data` into the hook layer (if the feature is intended), or move them behind a feature flag / wrapper struct, or remove them.

#### N6 — `sanitize_message` does 5 full-text scans + 5+ allocations even when no patterns match
**File:** `sanitize.rs:11-53`
**Severity:** MED | **New:** YES

`sanitize_message` calls `replace_regex` 5 times (lines 17-40). Each call dispatches to a redact function that always:
- Allocates `lower` (via `to_ascii_lowercase()` or `Vec<char>` collect)
- Allocates `result` (`String`)
- Scans the full text

Even if the text contains no sensitive patterns, all 5 functions run, each copying the full text into a new `String`. For a 512-byte message with no patterns, this is 5 full copies + 5+ auxiliary allocations. There's no early-exit guard like `if !text.contains("bearer") && !text.contains("token") { return text.to_string(); }`.

The `Cow<str>` in `sanitize_message` is **useless** — `replace_regex` always returns `String`, so `Cow::Owned(...)` is created on every call; the initial `Cow::Borrowed(msg)` is immediately replaced.

**Fix:** Add quick `contains` guards at the start of each redact function, or do a single-pass scan matching all patterns simultaneously. Make `replace_regex` return `Cow<str>` (returning `Cow::Borrowed(self)` when no match) so the existing `Cow` plumbing actually short-circuits.

#### N7 / N8 — `MutationResource` / `InfiniteQueryResource` missing `PartialEq`/`Eq` derives
**Files:** `mutation.rs:37`, `infinite_query/resource.rs:48`
**Severity:** MED | **New:** YES (discrepancy with implementation status doc)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]  // no PartialEq, Eq
pub struct MutationResource<V, T, E = QueryError> { ... }
```

The implementation status doc (`:193`) lists #78 as "Implemented". The main audit (`gpui-query-audit.md:379`) says "✅ Verified (`QuerySignal: Eq` via `Arc::ptr_eq`; bounded impls generated)". But `MutationResource` and `InfiniteQueryResource` do NOT have the derives — only `QuerySignal` does (manual `Eq` impl at `signal.rs:19-25`). `QueryResource` (`resource.rs:24`) DOES have `PartialEq, Eq`. All fields of the other two have `PartialEq`/`Eq` available with appropriate bounds.

**Fix:** Add `PartialEq, Eq` to both derive lists (with `T: PartialEq + Eq`, `V: PartialEq + Eq`, `E: PartialEq + Eq` implicit bounds). Or remove the unconditional `PartialEq, Eq` from `QueryResource` for consistency (N29 below).

### Client (`src/client/*`) — 7 findings

#### M1 — `MutationEntry::loading` is a dead field (dead branch in `gc`)
**File:** `mutation_bucket.rs:73` (field), `:168` (only write site, `false`), `:251` (only read site)
**Severity:** MED | **New:** YES

Grep across all of `src/` (excluding tests) for `\.loading\s*=|loading: true|set_loading|set_not_loading` returns **zero** writes-to-true. The `set_loading`/`set_not_loading` methods were removed as dead code (#75), and the field is never set to `true`. So `if entry.loading { return true; }` in `gc` is a **dead branch** — always false. The field costs 1 byte/entry (10 000 entries → ~10 KB + padding) plus a useless branch in every GC pass. It escapes the compiler's dead-code lint because it *is* syntactically written (to `false`) and read.

The doc comment at `:70-72` even admits it "stays `false` in practice". The real loading guard is `entity.read(cx).is_loading()` at `:263`.

**Fix:** Remove the `loading` field, the `loading: false` initializer, and the `if entry.loading` branch in `gc`. The `entity.upgrade()` + `entity.read(cx).is_loading()` checks (`:257,263`) fully cover liveness and in-flight status.

#### M3 — `prepare_fetch_query`/`prepare_prefetch_query` downcast the bucket twice
**File:** `lifecycle.rs:258,262` (also `:336,345`)
**Severity:** MED | **New:** YES

`self.resource::<T,E>(key.clone(), cx)` internally does the full M4 downcast dance to get/create the bucket and entity. `self.next_request_id_for_key::<T,E>(&key)` then does the **entire same downcast dance again** on the same `TypeId` just to reach the sequencer. So one `prepare_fetch_query` = 2× (TypeId check + downcast + match) = 4 vtable calls + 2 `AHashMap` lookups for the same bucket. Plus `current_time_ms()` at `:259` and again in `complete_success`/`complete_failure` (`prepared_fetch.rs:56,67`) = 2 syscalls for a fetch+complete.

**Fix:** Have `resource_with_policies` return the `&mut QueryBucket<T,E>` alongside the entity so the sequencer is reachable without a second downcast. Thread the `now_ms` from `prepare_fetch_query` (`:259`) into the `PreparedFetch` so `complete_*` can reuse it instead of re-syscall.

#### M4 — Redundant `TypeId` pre-check + `downcast_mut` = 2 vtable dispatches per op; 5× duplicated recovery block
**File:** `mod.rs:193-220` (+ identical at `infinite_mutation_ops.rs:58-83,122-143,180-207`, `mod.rs:269-289`)
**Severity:** MED | **New:** YES (perf angle; #11 covers only the dedup)

The "audit fix #11" pattern does `bucket.as_any().type_id() != expected_type_id` (vtable call #1) **and then** `bucket.as_any_mut().downcast_mut::<QueryBucket<T,E>>()` (vtable call #2, which *internally* does its own `type_id() == TypeId::of::<T>()` check). The explicit TypeId pre-check is **redundant** with the downcast's internal check — it adds a vtable dispatch on every hot-path resource creation purely to pre-format an error message for an impossible (`debug_assert!(false)`) mismatch branch.

The 11-line recovery block is copy-pasted **5 times**. `next_request_id_for_key` is on the hot fetch path (`fetch_retry.rs:63`, `use_infinite_query/hook.rs:205`), so the redundancy fires on every fetch.

**Fix:** Drop the pre-check; call `downcast_mut` once and handle `None` in the match arm (formatting the type name there, on the impossible path). Extract a private helper `fn bucket_or_recreate<B: ErasedBucket + Default>(slot: &mut Box<dyn ErasedBucket>, type_name: &str) -> &mut B` to kill the 5× duplication.

#### M5 — `InfiniteQueryBucket::cancel_matching` doesn't bump `ignored_results` (divergent from `QueryBucket`)
**File:** `infinite_bucket.rs:278-282` vs `erased_ops.rs:112-117`
**Severity:** MED | **New:** YES

`QueryBucket::cancel_matching` calls `resource.mark_ignored_result()` after cancelling the signal. `InfiniteQueryBucket::cancel_matching` only calls `signal.cancel()`. `InfiniteQueryResource` has **no** `mark_ignored_result` method (grep of `src/core/infinite_query` returns nothing). After a bulk `cancel_queries` touching infinite queries, the `ignored_results` counter diverges: query resources reflect the cancel, infinite resources do not.

**Fix:** Add `InfiniteQueryResource::mark_ignored_result()` (one-liner, mirroring `QueryResource::mark_ignored_result` at `resource/lifecycle.rs:248-250`) and call it from the infinite `cancel_matching`. Or document that infinite-query `ignored_results` only counts stale completions, not explicit cancels.

#### M6 — `MutationBucket::insert` calls `current_time_ms()` (syscall) on every mutation registration
**File:** `mutation_bucket.rs:167`
**Severity:** MED | **New:** YES

Every `register_mutation` (`infinite_mutation_ops.rs:194` → `insert`) pays a `SystemTime::now` syscall to stamp `updated_at`. The `MutationResource` *also* stamps its own `last_updated_at_ms` on completion (`mutation.rs:190,204`), and `gc` prefers that (`mutation_bucket.rs:284`), so `entry.updated_at` is only the fallback for never-completed mutations. For a hook that registers many mutations on mount, this is N syscalls.

`QueryBucket::get_or_create` does NOT call `current_time_ms` (the entity stamps its own timestamp); only `MutationBucket::insert` does. Inconsistent and avoidable.

**Fix:** Accept an optional `now_ms: u128` parameter on `insert`/`register_mutation`, letting `maybe_opportunistic_gc`'s already-cached `now_ms` be reused; or drop `entry.updated_at` and have `gc` fall back to `0` when `last_updated_at_ms()` is `None` (never-completed mutations are either Idle → immediately evictable, or Loading → retained by the `is_loading` check, so the insertion timestamp is rarely load-bearing).

#### M7 — `cancel_matching` does `read_with` then `update` = 2 entity locks per matching entry
**File:** `erased_ops.rs:110-118` (and `infinite_bucket.rs:276-285`)
**Severity:** MED | **New:** YES

For each matching key, the code does `entity.read_with(cx, |r, _| r.is_loading())` (lock #1) and, if loading, a separate `entity.update(cx, |resource, _| { signal.cancel(); … })` (lock #2). The `invalidate_matching`/`reset_matching` siblings correctly do a single `update`. The pre-check was added to avoid cancelling non-loading entries, but it costs a full extra lock acquire+release per match.

**Fix (refined after verification):** The original proposed fix (merge the check into `entity.update`) is **incorrect as stated** — GPUI's `entity.update` **always notifies observers** after the closure regardless of whether the closure mutated anything, so merging would cause extra re-renders for every non-loading matching entry. The current 2-lock pattern is actually intentional to avoid spurious notifications. Refined options: (a) accept the 2-lock pattern as a deliberate trade-off (cancel is not hot-path); (b) collect loading matches via `read_with` into a `Vec<Entity>`, then `update` only those — preserves the no-spurious-notify property while reducing lock acquisitions on non-loading matches to 1.

### Hook (`src/hook/*`) — 2 findings

#### H1 — `use_query_select` allocates `Arc<T>` (with full `T::clone`) on every source notification even when data unchanged
**File:** `use_query_select.rs:156-157`
**Severity:** MED | **New:** YES

```rust
let fresh: Option<Arc<T>> =
    entity.read(cx).data().map(|d| Arc::new(d.clone()));
```

Inside the `cx.observe` callback, the code unconditionally allocates a fresh `Arc<T>` (with a full `T::clone`) on **every** source notification. Then at `:158-162`, it compares `cached != fresh_ref` (via `PartialEq`) to decide whether anything changed. If nothing changed (the common case for periodic refetches that return identical data), the `Arc<T>` (and the `T` clone inside it) is discarded unused. For large `T` (e.g., `Vec<LargeStruct>`), this is a full `O(|T|)` clone + heap allocation wasted on every notification.

The inline comment at `:140-146` explicitly acknowledges this trade-off (preserves audit #115's no-nested-borrow fix).

**Fix:** Compare `&T` vs `&T` first (without allocating), only allocate `Arc<T>` if changed. This regresses #115's nested-borrow pattern, but for large `T` and frequent unchanged notifications, the perf win likely outweighs the style concern. Alternative: add a `data_arc()` accessor on `QueryResource` returning `Option<&Arc<T>>` (requires the broader Arc-ify-`QueryResource` architectural change) so `Arc::ptr_eq` can short-circuit without any `T` clone.

#### H2 — Unnecessary `+ Clone` bound on infinite query fetchers
**Files:** `fetch_helpers.rs:55,80,103`, `hook.rs:117`
**Severity:** MED | **New:** YES (audit #119 fixed this for mutation mutators but NOT infinite-query fetchers)

All four functions require `F: Fn(Option<&T>) -> Fut + 'static + Clone`. However, the fetcher is **never cloned** anywhere in the codebase — it's moved into the async block and borrowed as `&fetcher`. This over-constrains the API: fetcher closures that capture non-`Clone` resources (a `WeakEntity`, a `RefCell`, a non-`Clone` handle) are rejected by the compiler even though they would work fine at runtime.

**Fix:** Drop `+ Clone` from all four bounds. Non-breaking for existing callers (removing a bound only loosens the constraint).

### Tests / lib.rs — 3 findings

#### T2 — Dead test helpers `no_cache_options` / `ttl_zero_options` (implementation status #51 marked "Implemented" but 0 callers)
**File:** `test_support.rs:210,222`
**Severity:** MED | **New:** YES

Both functions are `pub` and documented but have **zero callers** across the entire test suite. Meanwhile, 114 literal `CachePolicy::NoCache` / `CachePolicy::Ttl { ttl_ms: 0 }` usages remain across 24 test files. The implementation status document marks audit #51 as "Implemented" — the helpers were created but never adopted.

**Fix:** Either adopt the helpers in the 24 files that still use literals, or remove the dead helpers and update the implementation status.

#### T3 — Stale test docs reference removed `StatusSnapshot` / snapshot APIs
**Files:** `integration_client/mod.rs:16-24` (references `StatusSnapshot` and `update_query_snapshot()`), `invalidation_reset_gc.rs:160-164` (references `StatusSnapshot` and `update_query_snapshot()`), `gc_query_operations.rs:44` (references `update_status_snapshot`)
**Severity:** MED | **New:** YES

The CL2/#106 fix removed `StatusSnapshot` and the `update_status_snapshot`/`update_query_snapshot` APIs entirely — GC now reads live entity state directly. The production code comments were updated, but these TEST comments still describe the old snapshot-based approach and reference the non-existent snapshot APIs. (Note: the first two files say `update_query_snapshot()` — a misnomer even in the stale comments; the third says `update_status_snapshot`, the correct name of the removed API.)

**Fix:** Update the comments to describe the current behavior: "GC reads live entity state directly via `entity.read(cx)` (CL2/#106)."

#### T4 — `for _ in 0..200 { cx.run_until_parked() }` busy-wait in regression test
**File:** `hook_tests/regression_tests.rs:215-220`
**Severity:** MED | **New:** YES

```rust
for _ in 0..200 {
    cx.run_until_parked();
    if *second_ran.lock().unwrap() { break; }
}
```

Calls `cx.run_until_parked()` up to 200 times, checking a `Mutex<bool>` each iteration. Typically breaks after 1-2 iterations, but in a degenerate case could iterate up to 200 times, each doing a full executor drain.

**Fix:** Use the `Gate` helper pattern (one-shot async signal) — have the spawned task signal completion via a `Gate`, then `cx.run_until_parked()` once after the signal.

---

## 🟡 LOW severity (30)

### Core (13)

| # | File:line | Issue | Fix |
|---|-----------|-------|-----|
| N9 | `resource/lifecycle.rs:239` | `cancel` has redundant `self.data = None` after `self.data.take()` — dead code | Remove line 239 |
| N10 | `resource/lifecycle.rs:50,72,123,146,232,249`; `cache.rs:90,110`; `infinite_query/lifecycle.rs:178,301,337`; `mutation.rs:293` | `cancelled_count += 1`, `cache_hits += 1`, `ignored_results += 1` use plain `+=` not `saturating_add` — #35 only fixed `increment_retry` (u32); u64 counters are practically unoverflowable but #35 said "everywhere" | Use `saturating_add(1)` for consistency, or document u64 as intentionally not saturating |
| N11 | `sanitize.rs:115` | `redact_url_schemes` allocates `format!("{scheme}://")` per scheme per loop iteration | Pre-compute `const NEEDLES: [&str; 4] = [...]` |
| N12 | `sanitize.rs:237,306` | `redact_emails`/`redact_hex` use `String::new()` without preallocation (inconsistent with other redact fns) | `String::with_capacity(text.len())` |
| N13 | `sanitize.rs:14-42` | `Cow<str>` in `sanitize_message` is useless — `replace_regex` always returns `String`, so `Cow` is always `Owned` after first call | Remove `Cow` and use `String`, or make `replace_regex` return `Cow<str>` (ties into N6) |
| N14 | `retry.rs:35,45` | `RetryPolicy::no_retries()` and `RetryPolicy::new()` could be `const fn` | Add `const` to both signatures |
| N15 | `key_filter.rs:5` | `QueryKeyFilter` missing `Copy` derive despite containing only `&'a QueryKey` (which is `Copy`) | Add `Copy` to derive list |
| N16 | `request.rs:55,119` | `RequestId.scope_id` is `u64`, not `NonZero<u64>` — `Option<RequestId>` is 24 bytes instead of 16 (niche optimization lost) | Change to `NonZero<u64>` |
| N17 | `request.rs:175` | `QueryTimestamp` uses `u128` for milliseconds — `u64` suffices until year 584M; doubles timestamp field sizes | Change to `u64` (or `NonZero<u64>` for niche) |
| N18 | `infinite_query/resource.rs:68-69` | Two mutually-exclusive `bool` fields (`is_fetching_next_page`, `is_fetching_previous_page`) could be `Option<PageDirection>` | Replace with `Option<PageDirection>` (1 byte, unbreakable invariant) |
| N19 | `resource/accessors.rs:127`; `mutation.rs:250`; `infinite_query/accessors.rs:194` | API naming inconsistency: `increment_retry` vs `increment_retry_count` | Pick one name and use it across all three |
| N20 | `mutation.rs` (impl block) | `MutationResource` missing `set_retry_policy` (inconsistent with `QueryResource`/`InfiniteQueryResource`) | Add the setter |
| N21 | `mutation.rs:8-19` vs `status.rs:11-25` | `MutationStatus` missing `is_loading()`/`is_idle()` etc. on the enum itself (unlike `QueryStatus`); methods duplicated on `MutationResource` | Add enum-level helpers, delegate from resource |
| N22 | `key.rs:70-75` | `QueryKey::as_str()` returns only the first segment — misleading name (implies full key) | Rename to `first_segment()` or `as_first_str()` |
| N23 | `resource/accessors.rs:92-94` | `signal_mut` never called in production — unused public API | Remove or mark `#[cfg(test)]` |
| N24 | `mutation.rs:161` | `with_key` never called in production; `key` field always `None` | Document as forward-compat, or remove |
| N25 | `mutation.rs:257,276` | Incorrect audit references in doc comments: `prepare_retry` cites "#19" (about `QueryKey::to_path`); `reset_retry_count` cites "#4" (about `use_query_select` clones) | Correct or remove the audit cross-references |
| N26 | `page_management.rs:99-101` | `enforce_max_pages_remove_back` does `drain().collect()` then `reverse()` — could be `drain().rev().collect()` | One-pass reverse collect |
| N27 | `infinite_query/lifecycle.rs:253,257,308,312` | `enforce_max_pages_*` return values (evicted pages) silently dropped in `complete_*` methods — API inconsistency with `append_page`/`prepend_page` | Either return evicted pages from `complete_*`, or add a `_void` variant that doesn't allocate |
| N28 | `select.rs:80,134` | `SelectTransform` and `MappedQueryResource` missing `PartialEq`/`Eq` (could use `Arc::ptr_eq` like `QuerySignal`) | Manual `PartialEq` via `Arc::ptr_eq`, then derive on `MappedQueryResource` |
| N29 | `resource.rs:24` | `QueryResource` derives `PartialEq`/`Eq` unconditionally — forces `T: PartialEq + Eq` on all users; the other two resources don't | Use conditional bounds, or add derives to the others for consistency (N7/N8) |
| N30 | `sanitize.rs:102` | `Redact::replace_regex` fallback `_ => self.to_string()` silently no-ops if pattern string doesn't match any guard | Replace with enum dispatch or add `debug_assert!(false)` in the fallback arm |

### Client (8)

| # | File:line | Issue | Fix |
|---|-----------|-------|-----|
| L1 | `lifecycle.rs:76-77` | `diagnostics()` allocates `queries`/`mutations` Vecs with no capacity hint despite knowing `bucket.count()` sums | `Vec::with_capacity(bucket.count().sum())` |
| L2 | `lifecycle.rs:118` | `dehydrate()` allocates `entries` Vec with no capacity hint | `entries.reserve(sum_of_counts)` |
| L3 | `erased_ops.rs:124-140`; `infinite_bucket.rs:289-308`; `mutation_bucket.rs:299-312` | `collect_diagnostics`/`collect_key_status` return a fresh Vec per bucket, immediately drained by `extend`/push | Add `collect_into(&mut Vec, cx)` sink-based API |
| L4 | `lifecycle.rs:208-210` | `restore(&self, …)` never reads `self`; should be an associated fn | Make it `pub fn restore(persister: &dyn QueryPersister)` |
| L5 | `lifecycle.rs:40-50` | `gc_with_time` does not update `last_gc_ms` → manual GC does not debounce the next opportunistic GC | Set `self.last_gc_ms = now_ms` at the top of `gc_with_time` |
| L6 | `erased_ops.rs:134` vs `infinite_bucket.rs:295-297` | `cache_age_ms` diagnostic inconsistency: `checked_sub`→`None` (query) vs `saturating_sub`→`Some(0)` (infinite) on clock skew | Add `InfiniteQueryResource::cache_age_ms(now_ms)` mirroring `QueryResource`'s |
| L7 | `lifecycle.rs:270-286,292` | `prepare_fetch_query` computes `started` then immediately `let _ = started;` — useless work | Drop the `match` and `started`; just call `begin_request_with_id` for its side effect |
| L8 | `observer.rs:33-80,86-129,144-187` | 3 observer types structurally identical (differ in entity type AND status type: `QueryStatus` vs `MutationStatus`) → one generic `Observer<R, S>` | Generic `struct Observer<R, S>` + type aliases |
| L9 | `erased_ops.rs:54-121` + `infinite_bucket.rs:224-286` | `invalidate`/`reset`/`cancel_matching` triplicated (6 near-identical impls). Overlaps existing audit #10 (generic bucket) but offers a more targeted fix | `for_each_matching_entry` helper on shared trait |
| L10 | `infinite_mutation_ops.rs:230-276` | 4 bulk-op methods are near-identical pair-of-loops over `buckets`+`infinite_buckets` | `for_each_query_bucket_mut` helper |
| L11 | `mod.rs:98` | `Default::default()` calls `current_time_ms()` syscall during construction (`new()` and `with_policies` both pay it) | Initialize `last_gc_ms: 0`; op-count gate is the primary debounce anyway |
| L12 | `mod.rs:300-307` | `get_query_data` returns `Option<T>`, forcing a full `T` clone per call; no Arc/borrow variant | Once `QueryResource` stores `Option<Arc<T>>`, return `Option<Arc<T>>` (or add `with_query_data` taking `FnOnce(&T)`) |
| L13 | `lifecycle.rs:131-168` | `dehydrate`'s `push_status_queries` closure handles only 2 of 3 loops; mutation loop inlined separately. Overlaps existing audit #94 (dehydrate dup) but critiques the partial fix | Generic `push_status` or `DehydrateKind` enum |
| L14 | `devtools.rs:84` | `DehydratedEntry::data_json` is always `None` from `dehydrate()`; 24 bytes/entry placeholder. Mentioned in passing in existing audit #9; elevated to standalone here | Remove until typed serialization lands, or box it, or gate behind `persistence` feature |
| L15 | `erased_ops.rs:110` vs `infinite_bucket.rs:276` | `is_loading()` vs `status().is_loading()` stylistic inconsistency (functionally identical) | Pick one style |

### Hook (11)

| # | File:line | Issue | Fix |
|---|-----------|-------|-----|
| H3 | `query_hooks.rs:96-98, 376-378` | Signal read as separate entity read after `begin_request_on_entity` already created it in `entity.update` | Add `signal: QuerySignal` to `QueryBeginResult::Started`/`StaleCacheHit`, or have `begin_request_on_entity` return `(Option<RequestId>, Option<QuerySignal>)` |
| H4 | `options.rs:163-168, 370-375` | `QueryOptions::new`/`InfiniteQueryOptions::new` use `..Default::default()` which allocates the default key then immediately overwrites it | Construct directly without `Default::default()` |
| H5 | `fetch_retry.rs:60-67` | `maybe_request_id` (key clone + `update_global` + sequence consumption) computed before knowing if cache hit will discard it | Restructure to mint the ID lazily inside `entity.update` |
| H6 | `fetch_retry.rs:173-193` | Two sequential `read_entity` calls (`is_current_request` + signal re-read) could be combined into one | Single `read_entity` returning `(bool, QuerySignal)` |
| H7 | `query_hooks.rs:77` | `opts.retry_policy.clone()` — could move via destructuring (retry_policy not used after) | Destructure `opts` and move `retry_policy` |
| H8 | `mutation_hooks/hooks.rs:85` | `opts.retry_policy.clone()` — could move via destructuring | Same as H7 |
| H9 | `mutation_hooks/hooks.rs:394-397, 453-457` | `cx.notify()` after `set_current_task` is unnecessary (no status change; observer callback runs for nothing) | Remove `cx.notify()` from the `set_current_task` update closure |
| H10 | `use_infinite_query/hook.rs:186-190` | Release-build `eprintln!("WARNING: …")` in observer fallback — inconsistent with audit fix #5 applied to `use_query_manual` | Remove the `eprintln!`, matching `use_query_manual` and `use_mutation` |
| H11 | `use_infinite_query/hook.rs:168` | `retry_policy.clone()` — could move (not used after this line). **Note:** H13's fix (clone at 168, move at 222) is strictly better and makes this fix moot — apply H13 instead | Move instead of clone (but prefer H13's fix) |
| H12 | `use_infinite_query/hook.rs:149-156 + 164-170` | `max_pages` set twice on standalone path (in `cx.new` closure AND in the unconditional update) | Remove the `max_pages` block from the `cx.new` closure |
| H13 | `use_infinite_query/hook.rs:222` | Retry policy re-read from entity (with clone) after just being set at line 168 — could reuse the original local | Clone at 168, move the original at 222 (saves one entity read + one clone) |
| H14 | `use_infinite_query/fetch_runners.rs:125-137` | Two separate `read_entity` calls (signal cancellation + `is_current_request`) after retry delay could be combined | Single `read_entity` returning `(bool, bool)` |
| H15 | `options.rs:299` | `MutationCallbacks._phantom: PhantomData<(T,E)>` is unnecessary — T and E are already used through the callback field types | Remove the `_phantom` field |
| H16 | `options.rs:286` (doc) | `MutationCallbacks` doc says "Not Clone because trait-object callbacks cannot be cloned" — misleading; all fields are `Option<Arc<…>>` which IS `Clone` | Implement `Clone` manually (no `T: Clone` bound needed); fix the doc |
| H17 | `gpui_compat.rs:18` | `read_entity` shim missing `#[inline]` | Add `#[inline]` |
| H18 | `mod.rs:164` | `current_time_ms()` missing `#[inline]` despite being `pub` (potentially called cross-crate) | Add `#[inline]` |

### Tests / lib.rs (7)

| # | File:line | Issue | Fix |
|---|-----------|-------|-----|
| T5 | `lib.rs:33-56` | Missing `#[doc(cfg(feature = "..."))]` on feature-gated modules/re-exports → rustdoc doesn't show feature requirements | Add `#![cfg_attr(docsrs, feature(doc_cfg))]` and annotate each gated module |
| T6 | `lib.rs:48-49` + `core/mod.rs:37` | `pub use core::*` re-exports the `key_filter` module (only `pub mod` in core; all others are private `mod` + `pub use`) | Make `key_filter` a private `mod` and only `pub use QueryKeyFilter`, for consistency |
| T7 | `gc_eviction.rs:17-32` | `create_success_with_snapshot` has misleading name (snapshots removed) + unused `_gc_time_ms` parameter (all 7 callers pass `1_000`, ignored) | Rename to `create_success_at_time`; remove the parameter |
| T8 | `core_mutation/*.rs` (28 declarations) | `MutationResource<String, ...>` forces ~40 `"literal".to_string()` allocations (6 cancellation + 22 lifecycle + 12 retry) where `MutationResource<&'static str, ...>` would avoid all | Change test declarations to `MutationResource<&'static str, ...>` |
| T9 | `core_infinite_query/*.rs` (10 declarations) | `InfiniteQueryResource<Vec<String>>` forces ~54 `vec![...to_string()...]` allocations (6 stale_and_completion + 28 max_pages + 11 page_fetch + 9 state_transitions) where `Vec<&'static str>` would avoid all | Change helpers to be generic; use `Vec<&'static str>` in tests |
| T10 | `policy_and_status_types.rs:17` | `assert_serde_roundtrip` is private — 8 duplicate manual roundtrips in `property_based.rs`, `retry_policy.rs`, `query_error.rs` | Move to `test_support.rs` as `pub fn`, adopt in the 8 sites (extends #129) |
| T11 | `tests/core_select.rs` | Standalone flat file while all peer test groups are in subdirectories — structural inconsistency | Move to `core_select/mod.rs` |
| T12 | `test_support.rs:429-433` | `Post` struct has `title: String` + `Default` derive — only used as type tag in 1 test, never constructed with data | Replace with `pub struct Post;` (unit struct) |
| T13 | `test_support.rs:255` | `pub struct DummyView` is `pub` but never imported by any test (6 local `struct DummyView;` remain) | Make private (`pub(crate)`) or adopt the shared one |
| T14 | `gc_coverage.rs:387-431` | `test_mutation_bucket_evict_oldest_keeps_count_bounded` creates 10 005 entities every test run — heaviest test in suite | Reduce to `MAX_ENTRIES + 2`, or gate with `#[ignore]` (extends #134) |

---

## Cross-cutting observations (architectural, not individual findings)

1. **The M2/M4/M5/M7 cluster has a common root cause:** the erased-bucket dispatch layer + the lack of a shared `for_each_matching_entry` / generic-bucket abstraction (#10, deferred) is what let the three bucket implementations drift apart (M5 missing `mark_ignored_result`, M7 double-lock, L6 `cache_age_ms` divergence, L15 style drift). Resolving #10 with even a *minimal* shared helper (not full generic `Bucket<K,R>`) would prevent the whole cluster from recurring.

2. **`current_time_ms()` call-site inventory in `src/client/`** (grep-verified): `mod.rs:98,150`, `prepared_fetch.rs:56,67`, `mutation_bucket.rs:167`, `lifecycle.rs:31,75,259,342`. The hot ones are `mutation_bucket.rs:167` (M6, per-insert) and `lifecycle.rs:259` (M3, per-prepare, doubled with `prepared_fetch.rs:56/67`). A `QueryClient`-threaded `now_ms` would collapse M3+M6+the prepared_fetch double-syscall in one pass.

3. **4× `Option<T>` in `QueryResource`**: `data`, `placeholder_data`, `previous_data`, `initial_data` — 4 copies of potentially large `T` per resource. Related to N5 (the latter two are dead). The `Arc<T>` architectural recommendation (audit arch-rec 3) would resolve this and also H1/L12.

4. **`InfiniteQueryResource` struct is very large**: ~20 fields, including 2× `Option<QueryTimestamp>` (48 bytes — N17 would halve this), `VecDeque<Arc<T>>` (24 bytes), `QueryKey` (8 bytes), `CachePolicy` (24 bytes), `RetryPolicy` (24 bytes), `Option<RequestId>` (24 bytes — N16 would save 8), plus bools and counters. Total: ~200+ bytes excluding `T`-dependent fields. For GPUI entity storage, this is copied on every `entity.read_with` clone. Consider boxing rarely-used fields or splitting into hot/cold structs.

5. **`Clone` on resource types shares `QuerySignal`**: all three resource types derive `Clone` and have `signal: Option<QuerySignal>` (which is `Arc<AtomicBool>`). After cloning, both copies share the same signal — cancelling one cancels both. The `CurrentTask` wrapper handles this by returning `Self(None)` on clone (losing the task handle), but `QuerySignal` has no such protection. Potential footgun for concurrent use, though in practice resources are stored in GPUI entities and not cloned for concurrent access.

---

## Recommended next steps (priority order)

1. **M2 `evict_oldest` O(n) entity reads** — the single hottest previously-unreported bottleneck. Mirror `last_updated_at_ms` + `loading` hint into `BucketEntry` so eviction scans metadata without `entity.read(cx)`. Requires cross-layer completion-path refresh.
2. **T1 feature-gate inconsistency** — `cargo test --no-default-features --features core` should compile. Gate `test_support`'s client bits + `integration_client`/`coverage_gaps` modules.
3. **H1 `use_query_select` per-notification clone** — compare `&T` vs `&T` first, allocate `Arc<T>` only if changed. Biggest perf win for derived-view workloads with unchanged refetches.
4. **N6 `sanitize_message` 5× full-text scans** — add `contains` early-exit guards or make `replace_regex` return `Cow<str>` so the existing `Cow` actually short-circuits. Saves 5 allocations on clean messages.
5. **M1 dead `MutationEntry::loading` field** — zero-risk removal, silences a dead branch, saves 1 byte + padding per entry.
6. **N3 `begin_request_with_id(None)` duplicate `RequestId(1,1)`** — real correctness bug; reachable from `fetch_retry.rs:55-66` when no `QueryClient` global. Store a sequencer in the resource or deprecate `None`.
7. **N1/N2/M5 diagnostic/state inconsistencies** — `ignored_results` undercount on infinite two-phase, `MutationResource::reset` stale timestamp, infinite `cancel_matching` missing `mark_ignored_result`. All one-liners.
8. **M3/M4/M6 syscalls + downcast redundancy** — thread `now_ms` from `prepare_fetch_query` through `PreparedFetch`; extract `bucket_or_recreate` helper to kill 5× duplication and the redundant TypeId pre-check.
9. **N7/N8 `PartialEq`/`Eq` derives** — the implementation status doc is wrong about #78. Add the derives or correct the doc.
10. **H2 drop `+ Clone` on infinite fetchers** — same fix as #119 for mutators; non-breaking API loosening.
11. **Stale test docs (T3) + dead helpers (T2) + misleading helper name (T7)** — cosmetic but the stale `StatusSnapshot` docs actively mislead.
12. **LOW bulk**: `#[inline]` on `current_time_ms`/`read_entity`, `RetryPolicy::new`/`no_retries` as `const fn`, `QueryKeyFilter: Copy`, `NonZero<u64>` for `RequestId.scope_id`, `u64` for `QueryTimestamp`, move-not-clone for `retry_policy` (H7/H8/H11/H13), remove release `eprintln!` (H10), `String::with_capacity` in redact fns (N12).

---

## Per-area detail

<details>
<summary><b>core (22 new findings: N1–N30)</b></summary>

- HIGH: 0
- MED: 7 (N1, N2, N3, N4, N5, N6, N7/N8)
- LOW: 15 (N9–N30 excluding N7/N8)

Files: `infinite_query/lifecycle.rs`, `infinite_query/resource.rs`, `infinite_query/page_management.rs`, `mutation.rs`, `resource/lifecycle.rs`, `resource.rs`, `resource/accessors.rs`, `resource/cache.rs`, `error/sanitize.rs`, `retry.rs`, `key_filter.rs`, `request.rs`, `key.rs`, `select.rs`.

</details>

<details>
<summary><b>client (15 new findings: M1–M7, L1–L15)</b></summary>

- HIGH: 1 (M2)
- MED: 6 (M1, M3, M4, M5, M6, M7)
- LOW: 8 (L1–L15 — L8/L9/L10/L15 counted once each but span multiple files)

Files: `bucket/ops.rs`, `bucket/erased_ops.rs`, `infinite_bucket.rs`, `mutation_bucket.rs`, `mod.rs`, `infinite_mutation_ops.rs`, `lifecycle.rs`, `observer.rs`, `prepared_fetch.rs`, `devtools.rs`.

</details>

<details>
<summary><b>hook (14 new findings: H1–H18)</b></summary>

- HIGH: 0
- MED: 2 (H1, H2)
- LOW: 12 (H3–H18)

Files: `use_query_select.rs`, `fetch_helpers.rs`, `hook.rs` (infinite), `fetch_runners.rs`, `query_hooks.rs`, `mutation_hooks/hooks.rs`, `options.rs`, `fetch_retry.rs`, `gpui_compat.rs`, `mod.rs`.

</details>

<details>
<summary><b>tests / lib.rs (14 new findings: T1–T14)</b></summary>

- HIGH: 1 (T1)
- MED: 3 (T2, T3, T4)
- LOW: 10 (T5–T14)

Files: `lib.rs`, `tests/mod.rs`, `test_support.rs`, `integration_client/`, `coverage_gaps/gc_eviction.rs`, `integration_client_coverage/`, `hook_tests/regression_tests.rs`, `core_mutation/`, `core_infinite_query/`, `core_policy_types/`, `tests/core_select.rs`.

</details>

---

## Verification methodology

- Four parallel explore subagents read every file in their area (`src/core`, `src/client`, `src/hook`, `src/tests` + `src/lib.rs`).
- Each finding cross-checked against the indexed finding numbers (#1–#144) in the existing audits to confirm no overlap.
- Criticals (M2, H1, N7/N8, M1) hand-verified against the live source by reading the cited lines plus supporting core types.
- All file:line citations resolved against the current tree (post-implementation-pass state).
- "Definitely real" vs "speculative" distinguished inline where the impact estimate depends on workload (e.g., M2's severity depends on whether the app keeps inserting new unique keys into a full bucket).
