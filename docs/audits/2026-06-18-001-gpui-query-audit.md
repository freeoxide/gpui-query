# gpui-query Performance & Boilerplate Audit

**Date:** 2026-06-18  
**Crate:** `crates/gpui-query/`  
**Scope:** `src/core`, `src/client`, `src/hook`, and `src/tests`  
**Total code:** ~25.7 kloc (tests included), ~10 kloc production code  
**Focus:** Performance optimizations, boilerplate reduction, GPUI-specific correctness, and idiomatic Rust.

## Executive Summary

`gpui-query` has a clean layered architecture (core state machine → GPUI client registry → hooks) and a robust request lifecycle, but it still carries significant duplication and several hot-path inefficiencies. The biggest opportunities are:

1. **Unbounded memory growth** in infinite-query buckets and mutation buckets.
2. **Large per-update clones** in `use_query_select`, mutation retries, and infinite page fetches.
3. **Detached async tasks** that keep running after unmount or replacement.
4. **Massive bucket/hook duplication** that can be collapsed with generics and macros.
5. **Many mechanical clippy/idiom fixes** that remove warning noise and production `expect` calls.

This document lists 55 actionable findings, grouped by impact and implementation order.

---

## Audit Methodology

- Loaded all available GPUI skills and the `rust-best-practices` skill.
- Ran five parallel subagent audits covering core, client, hooks, tests, and cross-cutting idioms.
- Verified findings with `cargo clippy --package gpui-query --all-features -- -W clippy::perf -W clippy::complexity -W clippy::style`.

---

## 🔴 High-Impact Fixes

### 1. `InfiniteQueryBucket` never evicts successful resources — memory leak

- **File:** `src/client/infinite_bucket.rs` (lines 187–226)
- **Issue:** `gc()` treats `QueryStatus::Success` as non-evictable and has no `max_entries` / `evict_oldest`. Long-running apps with many distinct infinite keys will grow without bound.
- **Fix:** Mirror `QueryBucket::gc()`:
  - Add `max_entries`.
  - Apply `SUCCESS_GC_MULTIPLIER`.
  - Use stale-while-revalidate checks.
  - Implement `evict_oldest`.
- **Impact:** Prevents unbounded memory growth.

### 2. `MutationBucket` has no entry limit

- **File:** `src/client/mutation_bucket.rs` (lines 68–71)
- **Issue:** Uses an unbounded `AHashMap<u64, MutationEntry<...>>`.
- **Fix:** Add `max_entries` and `evict_oldest` for completed mutations.
- **Impact:** Prevents unbounded mutation cache growth.

### 3. Mutation retry clones the entire variables payload every attempt

- **File:** `src/hook/mutation_hooks/internals.rs` (lines 53–54, 175–176)
- **Issue:** `mutator((*variables).clone()).await` clones `V` on every retry → `O(|V| × retries)`.
- **Fix options:**
  - **Breaking:** change mutator bound from `Fn(V) -> Fut` to `Fn(&V) -> Fut`.
  - **Non-breaking:** add `mutate_by_ref` / `mutate_arc` that stores `Arc<V>` and passes `&V` to the mutator.
- **Impact:** Eliminates large per-retry allocations.

### 4. `use_query_select` clones the entire source dataset on every update

- **File:** `src/hook/use_query_select.rs` (lines 112–138)
- **Issue:** Two entity locks plus `entity.read(cx).data().cloned()` on each source change.
- **Fix:**
  1. In `src/core/select.rs`, store source data in `MappedQueryResource` as `Option<Arc<T>>` instead of `Option<T>`.
  2. In the hook, compare `Arc::ptr_eq` before re-transforming.
- **Impact:** Removes full clones of `T` for derived views.

### 5. Infinite query runners clone the full last/first page to call the fetcher

- **File:** `src/hook/use_infinite_query/fetch_runners.rs` (lines 40–46, 150–156)
- **Issue:** `last_page().cloned()` / `first_page().cloned()` copies `T` just to pass a reference to the fetcher.
- **Fix:** Store pages internally as `Arc<T>` and hand `Arc::clone` to fetchers.
- **Impact:** Removes `O(|T|)` copy per page fetch.

### 6. Detached async tasks are never cancelled

- **Files:**
  - `src/hook/query_hooks.rs` (lines 80–92)
  - `src/hook/mutation_hooks/hooks.rs` (lines 234–238)
  - `src/hook/use_infinite_query/hook.rs` (lines 227–238)
  - `src/hook/use_infinite_query/fetch_helpers.rs` (lines 83–88)
- **Issue:** All hooks use `cx.spawn(...).detach()` with no abort handle. Fetchers that do not poll `QuerySignal` continue executing after unmount or after being superseded.
- **Fix options:**
  - Store `Task<()>` handles on the resource entity (`Option<Task<()>>`) and call `.abort()` on replacement / drop.
  - Return a `Subscription`-like guard that aborts the task when dropped.
- **Impact:** Saves CPU/network and prevents late side effects.

### 7. `mutate` has a read-then-update race window

- **File:** `src/hook/mutation_hooks/hooks.rs` (lines 219–228, 270–280)
- **Issue:** `entity.read_with(..., is_loading())` followed by a separate `entity.update(..., begin())` lets two rapid `mutate` calls both begin.
- **Fix:** Perform the `is_loading` check inside the same `entity.update` that calls `begin`:
  ```rust
  let started = entity.update(cx, |resource, cx| {
      if resource.is_loading() { return false; }
      resource.begin(variables.clone());
      cx.notify();
      true
  });
  if !started { return; }
  ```
- **Impact:** Prevents double mutation execution under rapid input.

### 8. `retain` / `release` are dead code — GC may evict observed resources

- **Files:**
  - `src/client/bucket/ops.rs` (lines 183–203)
  - `src/client/infinite_bucket.rs` (lines 124–138)
  - `src/client/mutation_bucket.rs` (lines 161–176)
- **Issue:** Marked `#[allow(dead_code)]` and never wired to observer lifecycle. `observer_count` is the only GC protection but is never incremented/decremented.
- **Fix options:**
  - Remove `observer_count` and rely on `WeakEntity`/GPUI lifetime.
  - Create an `ObserverGuard` / `Subscription` wrapper that calls `retain()` on creation and `release()` on drop.
- **Impact:** Ensures GC respects active observers.

### 9. `dehydrate()` does expensive full diagnostics just to discard them

- **File:** `src/client/lifecycle.rs` (lines 239–271)
- **Issue:** Calls `collect_diagnostics()` per bucket, which upgrades every entity and allocates `Vec<QueryDiagnostic>`, but uses only `diag.status` and `diag.key`. `data_json` is hard-coded to `None`.
- **Fix:** Iterate bucket entries directly, reading only key + `StatusSnapshot`/entity status. Cache one `now_ms` value.
- **Impact:** Removes `O(n)` allocations and entity reads per bucket.

---

## 🟠 Medium-Impact Fixes

### 10. Collapse duplicated bucket implementations

- **Files:** `src/client/bucket/ops.rs`, `src/client/bucket/erased_ops.rs`, `src/client/infinite_bucket.rs`
- **Issue:** `QueryBucket` and `InfiniteQueryBucket` are ~90% identical.
- **Fix:** Introduce a generic `Bucket<K, R>` parameterized by resource type; implement the two as thin aliases or wrappers.
- **Impact:** Large maintainability win; fixes apply once.

### 11. Dedupe the erased-bucket downcast-recovery block

- **Files:**
  - `src/client/mod.rs` (lines 134–151)
  - `src/client/infinite_mutation_ops.rs` (lines 57–70, 107–120, 158–171)
- **Issue:** Same “check downcast, replace if mismatch, downcast again” pattern repeated.
- **Fix:** Replace the first `downcast_mut` with a cheaper `type_id()` comparison, or extract a helper macro.
- **Impact:** Less duplication and slightly cheaper bucket creation.

### 12. Simplify infinite-query begin/completion duplication

- **File:** `src/core/infinite_query/lifecycle.rs` (lines 26–232, 260–342)
- **Issue:** Four near-identical begin methods and two near-identical completion paths.
- **Fix:**
  - One `begin_fetch(direction, maybe_id, now_ms)` helper.
  - One `insert_page(...)` / `finish_completion(...)` helper.
- **Impact:** Removes the largest duplication block in the crate.

### 13. Merge `fetch_next_page_infinite` / `fetch_previous_page_infinite`

- **Files:**
  - `src/hook/use_infinite_query/fetch_helpers.rs` (lines 45–145)
  - `src/hook/use_infinite_query/fetch_runners.rs` (lines 23–230)
- **Issue:** ~230 lines of duplicated retry/completion loop.
- **Fix:** Introduce `PageDirection { Next, Previous }` and single helpers `begin_page_fetch(...)` / `run_fetch_page_with_id(..., is_next: bool)`.
- **Impact:** ~70% reduction in infinite runner code.

### 14. Merge the two query retry loops

- **File:** `src/hook/fetch_retry.rs` (lines 119–353)
- **Issue:** `fetch_with_retry` and `fetch_signal_with_retry` duplicate backoff/cancel/complete logic.
- **Fix:** Extract a generic `run_query_retry_loop(..., mut attempt: impl FnMut(&mut AsyncApp) -> Fut)` helper.
- **Impact:** One retry-loop implementation instead of two.

### 15. Merge mutation retry loops

- **File:** `src/hook/mutation_hooks/internals.rs` (lines 37–315)
- **Issue:** `run_mutation_loop` and `run_mutation_loop_with_callbacks` duplicate the loop body.
- **Fix:** One loop that takes `Option<&MutationCallbacks<T,E>>` and invokes callbacks when present.
- **Impact:** Uniform behavior and easier maintenance.

### 16. `prepare_fetch_query()` has a no-op boolean chain

- **File:** `src/client/lifecycle.rs` (lines 365–383)
- **Issue:** `.then(|| false).unwrap_or(false)` always returns `false`; the `entity.update` result is discarded.
- **Fix:** Capture the closure’s `bool` in a `started` variable, mirroring `prepare_prefetch_query()`.
- **Impact:** Clarity and removes dead logic.

### 17. `QueryBucket::gc()` upgrades every entry just to check liveness

- **File:** `src/client/bucket/erased_ops.rs` (lines 60–65)
- **Issue:** Creates and immediately drops a strong `Entity` per entry.
- **Fix:** Cache a dead flag on the entry, or use a cheaper liveness probe if GPUI exposes one.
- **Impact:** Reduces atomic refcount traffic during GC sweeps.

### 18. `InfiniteQueryBucket::invalidate_matching()` pins entities

- **File:** `src/client/infinite_bucket.rs` (lines 232–250)
- **Issue:** Builds `Vec<Entity<...>>` of all matches before updating, holding strong references.
- **Fix:** Collect `Vec<QueryKey>` and upgrade inside the loop, matching `QueryBucket::invalidate_matching()`.
- **Impact:** Avoids delaying GC of invalidated resources.

### 19. `QueryKey::to_path()` is O(n²)

- **File:** `src/core/key.rs` (lines 74–83)
- **Issue:** `acc + "::" + s` allocates a new `String` per segment.
- **Fix:** Build with a single `String`:
  ```rust
  pub fn to_path(&self) -> String {
      let mut s = String::new();
      for (i, part) in self.0.iter().enumerate() {
          if i > 0 { s.push_str("::"); }
          s.push_str(part);
      }
      s
  }
  ```
- **Impact:** Linear path building for diagnostics and keys.

### 20. `MappedQueryResource` stores source data by value, not reference

- **File:** `src/core/select.rs` (lines 128–132, 165–167, 189–191)
- **Issue:** Clones `T` into every mapped view.
- **Fix:** Store `Option<Arc<T>>` and clone only the `Arc`.
- **Impact:** Matches documented “shared source data” semantics.

### 21. `QueryClient::Default` gives `gc_time_ms: 0`

- **File:** `src/client/mod.rs` (lines 62–77)
- **Issue:** `#[derive(Default)]` yields `0`, while `with_policies()` sets `300_000`. `QueryClient::new()` therefore gets `0`.
- **Fix:** Implement explicit `Default` with `gc_time_ms: 300_000` and remove the redundant assignment from `with_policies()`.
- **Impact:** Predictable default behavior.

---

## 🟡 Mechanical / Idiomatic Fixes

| # | Issue | File(s) | Fix |
|---|-------|---------|-----|
| 22 | Deprecated re-export warning | `src/hook/mod.rs:134`, `src/hook/mutation_hooks/mod.rs:8` | Remove `use_mutation_with_options` from public re-exports |
| 23 | `manual_saturating_arithmetic` | `src/core/retry.rs:90` | Use `self.retry_delay_ms.saturating_mul(factor)` |
| 24 | `redundant_closure` | `src/hook/fetch_retry.rs:325` | `.unwrap_or_else(QuerySignal::new)` |
| 25 | `collapsible_if` | `src/core/infinite_query/page_management.rs`, `src/client/bucket/erased_ops.rs`, `src/client/bucket/ops.rs`, `src/client/infinite_bucket.rs`, `src/client/lifecycle.rs` | Combine nested `if let` with `&& let` |
| 26 | `doc_overindented_list_items` | `src/client/bucket/erased_ops.rs:34,36,38` | Indent with 3 spaces |
| 27 | `unused_must_use` on `entity.update` | `src/hook/fetch_retry.rs`, `src/hook/mutation_hooks/internals.rs`, `src/hook/query_hooks.rs`, `src/hook/use_infinite_query/fetch_runners.rs` | Prefix with `let _ =` |
| 28 | Missing `#[must_use]` | `RequestGuard`, `RequestId`, `QueryBeginResult`, `PreparedFetch` | Add attribute |
| 29 | `expect` in production | `src/client/mod.rs:150,211`, `src/client/infinite_mutation_ops.rs:69,119,170`, `src/client/mutation_bucket.rs:106`, `src/hook/mutation_hooks/hooks.rs:87,124` | Use `debug_assert!` + fallback or return `Option`/`Result` |
| 30 | Release-build `eprintln!` | `src/hook/use_infinite_query/hook.rs:187-193`, `src/hook/use_infinite_query/fetch_runners.rs` | Remove or guard with `#[cfg(debug_assertions)]` |
| 31 | `QueryKey::starts_with()` manual zip | `src/core/key.rs:90-102` | Use `slice::starts_with` |
| 32 | `CachePolicy::is_stale_but_serveable()` duplicates arithmetic | `src/core/policy.rs:137-145` | Reuse `total_valid_ms()` |
| 33 | `RetryPolicy::default` redundant builder calls | `src/core/retry.rs:102-109` | `Self::new(3).with_exponential_backoff()` |
| 34 | `QueryResource::cancel()` can be simplified | `src/core/resource/lifecycle.rs:234-239` | `self.previous_data = self.data.take();` |
| 35 | `increment_retry()` inconsistent overflow behavior | `src/core/resource/accessors.rs:126-129`, `src/core/infinite_query/accessors.rs:174-177` | Use `saturating_add` everywhere |
| 36 | `enforce_max_pages_remove_back()` uses `pop` loop | `src/core/infinite_query/page_management.rs:85-97` | Use `drain` like the front variant |
| 37 | `SelectTransform` has unnecessary `PhantomData` | `src/core/select.rs:80-83, 100-107` | Drop `_marker` |
| 38 | `InfiniteQueryResource` split derives | `src/core/infinite_query/resource.rs:43-44` | Combine to one line |
| 39 | `match` on two-variant `RequestPolicy` | `src/core/infinite_query/lifecycle.rs:36,90,145,201` | Use `if policy == IgnoreWhileLoading { return None; }` |
| 40 | `map(...).unwrap_or(false)` | `src/hook/use_infinite_query/fetch_runners.rs:98,205` | `map_or(false, ...)` or `is_some_and` |
| 41 | Duplicate `current_time_ms()` | `src/client/erased.rs:17`, `src/hook/mod.rs:144`, `src/client/mutation_bucket.rs:75` | Single crate-internal helper |
| 42 | `InfiniteQueryOptions` missing `From` impls | `src/hook/options.rs:334-338` | Add `From<String>` and `From<QueryKey>` |
| 43 | `MutationOptions` missing builder methods | `src/hook/options.rs:183-198` | Add `.retry_policy()`, `.gc_time()` |
| 44 | `QueryOptions` / `InfiniteQueryOptions` duplicate builders | `src/hook/options.rs:116-161, 309-331` | Use a declarative macro |
| 45 | `RequestId::label()` / `CachePolicy::label()` force allocation | `src/core/request.rs:74-77`, `src/core/policy.rs:54-66` | Add `impl Display`; keep `label()` for compatibility |

---

## 🧪 Test-Specific Recommendations

| # | Issue | Fix |
|---|-------|-----|
| 46 | ~100× repeated `setup_query_client(cx)` | Single `setup_test(cx)` helper or `#[gpui::test]` wrapper macro |
| 47 | ~50× one-off harness structs | Generic helpers in `test_support.rs`: `query_harness()`, `mutation_harness()` |
| 48 | 82× `cx.run_until_parked()` + read block | `run_until_parked_and_read()` helper or `assert_query_after_parked!` macro |
| 49 | Busy-wait gate loops with 1 ms ticks | Reusable `Gate` helper with a one-shot async signal |
| 50 | Repeated `MutationOptions { retry_policy: ..., gc_time_ms: ... }` | Constants `no_retry_mutation_options()`, `DEFAULT_MUTATION_GC_MS` |
| 51 | Repeated `CachePolicy::Ttl { ttl_ms: 0 }` / `NoCache` | `no_cache_options(key)`, `ttl_zero_options(key)` helpers |
| 52 | Copy-pasted `DummyView` + observer pattern | `observe_with_dummy_view(cx, observer)` helper |
| 53 | Property-test heavy allocations | Lower long-segment bounds, reduce `with_cases` from 1000 to 256 for arithmetic invariants |
| 54 | Large deterministic stress tests always run | Gate 2000-char / 200-segment tests behind a `stress` feature or `#[ignore]` |
| 55 | Table-driven opportunities | Consolidate policy/status/error roundtrip tests |

---

## 🏗️ Architectural Recommendations

1. **Generic bucket abstraction.** `QueryBucket`, `InfiniteQueryBucket`, and much of `MutationBucket` share the same map/sequencer/observer-count/GC shape. A `Bucket<K, R>` generic would cut hundreds of lines and prevent fixes from being applied twice.

2. **Unified request runner.** The query, infinite-query, and mutation hook runners all do: downgrade entity → check signal/is_current_request → retry/backoff → complete. A single `run_retry_loop` helper parameterized by the attempt closure would remove ~400 lines.

3. **Arc-ify large cached data.** Consider storing `T` as `Arc<T>` inside `QueryResource`, `InfiniteQueryResource`, and `MutationResource`. This makes derived views, optimistic updates, and page fetches cheap to clone, at the cost of one indirection per access.

4. **Task lifetime management.** Move from “fire and forget” to stored `Task<()>` handles that are aborted on replacement and on resource drop. This is the most important correctness improvement after the memory leaks.

5. **Observer retain/release or remove it.** The current `observer_count` mechanism is dead code. Either implement a real `ObserverGuard` or delete it and rely on GPUI’s `WeakEntity` lifetime.

6. **Persistence cleanup.** `dehydrate()` currently serializes metadata only. Decide whether persistence is a real feature; if so, serialize actual cached data and implement typed `hydrate()`. If not, remove the stub.

---

## Suggested Implementation Order

1. **Mechanical clippy/idiom fixes** (#22–#45) — safe, fast, removes warning noise.
2. **Memory bounds** (#1, #2) — prevent unbounded growth.
3. **Task cancellation** (#6) + **mutation race** (#7) — correctness.
4. **Clone reduction** (#3, #4, #5, #20, #21) — measurable perf wins.
5. **Bucket/hook deduplication** (#10, #11, #12, #13, #14, #15) — maintainability.
6. **Test helpers** (#46–#55) — reduces suite size and flakiness.

---

## Appendix: Files with the Most Findings

| File | Main Concerns |
|------|---------------|
| `src/client/infinite_bucket.rs` | No success eviction, no entry cap, entity pinning in invalidate |
| `src/client/mutation_bucket.rs` | No entry cap, dead retain/release, duplicate `now_ms` |
| `src/client/bucket/ops.rs` | Duplicated bucket logic, repeated key clones, dead retain/release |
| `src/client/lifecycle.rs` | Expensive dehydrate, duplicated retain/release helpers, no-op boolean chain |
| `src/client/mod.rs` | Double downcast, `Default` gc_time inconsistency |
| `src/hook/mutation_hooks/internals.rs` | Variables clone per retry, duplicated mutation loops |
| `src/hook/mutation_hooks/hooks.rs` | Race window, `expect` in production, duplicated mutate paths |
| `src/hook/use_infinite_query/fetch_runners.rs` | Page clones, duplicated next/previous runners, release `eprintln` |
| `src/hook/use_infinite_query/fetch_helpers.rs` | Duplicated next/previous begin helpers |
| `src/hook/fetch_retry.rs` | Duplicated retry loops, redundant closures, discarded results |
| `src/hook/use_query_select.rs` | Full source clone per update, nested entity reads |
| `src/core/infinite_query/lifecycle.rs` | Four duplicated begin methods, request guard not consumed |
| `src/core/select.rs` | `MappedQueryResource` clones `T`, unnecessary `PhantomData` |
| `src/core/key.rs` | Quadratic `to_path`, `From<String>` copies |
| `src/core/error/sanitize.rs` | Inefficient redaction, Unicode corruption in `redact_tokens` |
| `src/core/resource/lifecycle.rs` | Duplicated begin paths, cancel simplification |
| `src/tests/` | Heavy setup/harness/gate duplication, property-test tuning |
