# gpui-query Audit — Implementation Status (Re-verification)

**Date:** 2026-06-18
**Source audit:** [`gpui-query-audit.md`](./gpui-query-audit.md) (97 primary + 42 addenda + 5 criticals)
**Method:** 5 parallel haiku verification agents (core / client / hook / tests / clippy) read the live
`crates/gpui-query/src` tree against every cited finding, then criticals were cross-checked by hand
against the actual source + `cargo clippy` ground truth (640 warnings, down from the audit's 695).
Cited line numbers were re-resolved by symbol where they had drifted.

---

## ✅ Implementation Complete (2026-06-19)

A second dynamic-workflow pass implemented the remaining actionable findings to completion.
**Current state: 787 tests pass (0 failed), clippy warnings 695 → ~560, no dead-code warnings.**

What landed in the implementation pass:
- **All 5 criticals** + high-impact items were already done (C1/C2/CL1/CL2/CL7, #1/#2/#6/#7/#3/#5).
- **#86** — `complete_*_with_guard` now consume `RequestGuard` by value (mirrors `QueryResource`'s two-phase protocol); all 10 call sites updated.
- **#112** — `MutationBucket` GC now measures recency from `MutationResource`'s last terminal-completion time (`last_updated_at_ms`, stamped in `complete_success`/`complete_failure`, reset on `begin`), not insertion time. Falls back to insertion time for never-completed mutations.
- **#9/#16/#94** — `dehydrate` uses a lightweight `collect_key_status` (no per-entry `QueryDiagnostic` allocation); `prepare_fetch_query` captures the real `started` bool; the 3 dehydrate loops collapsed.
- **#12/#39/#87** — infinite-query begin methods consolidated behind a private `begin_fetch(direction,…)` (public signatures preserved); `match`→`==` for `RequestPolicy`; per-direction guard documented.
- **#20/#96** — `MappedQueryResource` stores `Option<Arc<T>>` (cheap clones); `QuerySelectResult<T,U,E>` alias added.
- **#13** — `fetch_next/previous` runners unified via `PageDirection`.
- **#58/#59/#60/#41/#71/#29/#75** — bucket hash/clone reductions, consolidated `current_time_ms`, `Observer::observe` → `&self`, client `.expect()`→safe fallbacks, dead-code allows removed.
- **#10 cleanup** — deleted dead `maybe_gc`/`should_run_opportunistic_gc`/`abort_current_task`.
- **Tests** — added regression tests **#131** (task cancellation on drop) and **#132** (cross-context TOCTOU rejection); **#134** (MutationBucket eviction at cap); helpers `complete_success_id`/`HookHarness` + table-driven serde roundtrips; adopted `run_until_parked_and_read`/`observe_with_dummy_view`/`setup_test`.
- **Clippy** — `clippy --fix` + manual: #25/#26/#56/#136/#137/#138 cleaned in production; #28 (`#[must_use]`), #77 (default-key note), #83 (clock-fallback doc) addressed.

**Documented exceptions (3 — not force-completed, with rationale):**
- **#10** (generic `Bucket<K,R>`): **infeasible.** `MutationBucket` uses `u64` keys + 3 type params (`V,T,E`) vs `QueryKey` + 2 (`T,E`) for the others, and the `dyn ErasedBucket` dispatch layer prevents a clean unification without a risky rewrite. `bucket/shared.rs` already consolidates shared constants/helpers.
- **#38**: **inevitable.** `InfiniteQueryResource`'s `#[serde(bound(serialize=…))]`/`(deserialize=…)` attributes cannot merge onto the `#[derive]` line — each is a distinct load-bearing key.
- **#4** (partial): the `Arc::ptr_eq` skip-the-clone is unachievable in isolation because the source `QueryResource` stores `T` by value (only lends `&T`); it would require the broader "Arc-ify `QueryResource`" architectural change. The achievable part — `Arc<T>` storage in `MappedQueryResource` making derived clones cheap — **is** done.

Everything else from the original 144 findings is implemented or N/A (misleading/inaccurate per the audit).

---

## Headline (pre-implementation baseline)

**Of 144 verified findings: 80 fully implemented, 18 partial, 31 not implemented, 13 N/A
(audit itself flagged misleading/inaccurate), 2 already-fixed.**

Excluding the 13 N/A and 2 already-fixed (which need no action), **98 of 129 actionable findings
are substantially done (76%)**. Every 🚨 critical bug and every architectural high-impact item
is implemented. What remains is a predictable tail: a few intentionally-deferred big refactors,
mechanical clippy lint sweeps (reduced but not zeroed), two missing regression tests, and small
style/perf cleanups.


| Area | Findings | ✅ Done | 🟡 Partial | ❌ Not done | ➖ N/A | 👻 Fixed |
|------|---------:|--------:|-----------:|-----------:|-------:|---------:|
| `src/core/*` | 31 | 12 | 2 | 10 | 7 | 0 |
| `src/client/*` | 32 | 21 | 2 | 8 | 0 | 1 |
| `src/hook/*` | 38 | 29 | 5 | 1 | 2 | 1 |
| `src/tests/*` | 29 | 15 | 6 | 5 | 3 | 0 |
| clippy/mech | 14 | 3 | 3 | 7 | 1 | 0 |
| **Total** | **144** | **80** | **18** | **31** | **13** | **2** |

---

## ✅ The big wins — fully implemented

**All criticals and high-impact correctness items are done.**

- 🚨 **C1 / #98** — `redact_tokens` rewritten with `Vec<char>` char-indexed scan (`sanitize.rs:147-181`); non-ASCII panic gone + regression tests (`:338-353`).
- 🚨 **C2 / #99** — `sanitize_message` truncation uses `char_indices().rfind()` (`sanitize.rs:44-49`); mid-character panic gone + regression tests.
- 🚨 **CL1 / #105** — GC now runs from production via `maybe_opportunistic_gc()` every 64 ops (`client/mod.rs:142-156, 210`; `infinite_mutation_ops.rs:74,179`).
- 🚨 **CL2 / #106** — `StatusSnapshot` **removed entirely**; GC reads live entity state directly (`bucket/ops.rs:154-196`). The "stale snapshot" problem is moot.
- 🚨 **CL7 / #111** — deprecated `use_mutation_with_options` now delegates to `use_mutation`, so it registers (`hooks.rs:130-141`).
- **#1** — `InfiniteQueryBucket` gained `max_entries` + `evict_oldest` + `SUCCESS_GC_MULTIPLIER` success eviction (`infinite_bucket.rs:39,102,156-160`).
- **#2** — `MutationBucket` gained `max_entries` + `evict_oldest` + insert-limit check (`mutation_bucket.rs:77,111,147-148`).
- **#6** — `CurrentTask` wrapper (`core/mod.rs:76-97`) + `set_current_task` on `MutationResource`/`InfiniteQueryResource`; tasks stored and cancelled on replacement/unmount via GPUI `Drop` semantics. **Mutation + infinite-query sites fully converted** (5 spawn sites now `set_current_task`). *Query sites deliberately left detached* — see Partial.
- **#7** — mutation race fixed: `is_loading` check now atomic inside the same `entity.update` as `begin` (`hooks.rs:353-361`).
- **#8** — `retain`/`release`/`observer_count` dead code removed; GC relies on `WeakEntity::upgrade()` liveness.
- **#3** — `mutate_by_ref` / `mutate_arc` added; retry loop borrows `&V` from `Arc<V>`, no per-attempt clone (`internals.rs:54-66`; `hooks.rs:396-454`).
- **#5** — infinite pages stored as `VecDeque<Arc<T>>`; fetchers get `Option<&T>` via `Arc` deref, no page clone (`resource.rs:54`; `fetch_runners.rs:45-51`).
- **#14** — two query retry loops collapsed into one `run_query_retry_loop` parameterized by `Option<QuerySignal>` (`fetch_retry.rs:108-233`).
- **#15** — mutation retry loops collapsed into one `run_mutation_loop_inner` + 4 thin wrappers (`internals.rs:54-237`).
- **#72** — `cx.notify()` moved inside the `accept_current_request` success branch (`fetch_runners.rs`).
- **#73** — infinite runners now verify `is_current_request(request_id)` after retry delay (`fetch_runners.rs:107,224`).
- **#100/#101/#102** — core state invariants: `rollback_to_previous` clears `error`, `clear_data` transitions Success→Idle, `record_stale_cache_hit` sets `Success`.
- **#107/#108/#109/#110/#113/#114** — GC evicts `Cancelled`, evicts aged `Success` mutations, `evict_oldest` skips in-flight entries, `dehydrate` includes mutations, `insert` uses its `cx`.
- Full list of 80 implemented items is itemized per-area below.

---

## 🟡 Partial (18) — fix started, gap is specific

| # | Gap |
|---|-----|
| **#4** | `use_query_select` still does `data().cloned()` (full `T` clone); the `Arc<T>` storage in `MappedQueryResource` (#20) was **not** applied — so the root cause remains. Observer now reads source once (#115 ✅). |
| **#6 (scope)** | Fully done for mutations + infinite queries; the **4 plain-query spawn sites stay detached by design** (`query_hooks.rs:96-102, 143-148`) — cooperative signal/`is_current_request` already prevents stale writes, and hard-abort would break the superseded-fetcher contract. Defensible; document if anything. |
| **#13** | `fetch_next_page`/`fetch_previous_page` runners share structure but were **not** merged into a `PageDirection { Next, Previous }` + single helper. |
| **#36** | `enforce_max_pages_remove_front` uses `drain` ✅; `enforce_max_pages_remove_back` still uses a `pop_back()` loop. |
| **#41** | Hook-layer `current_time_ms()` consolidated ✅; client-side duplicates (`erased.rs`, `mutation_bucket.rs`) remain. |
| **#46** | `setup_test` alias exists but 190 call sites still use the verbose `setup_query_client(cx)`. |
| **#48** | `run_until_parked_and_read` helper exists but has **0 callers** — 82 sites not refactored. |
| **#52** | `observe_with_dummy_view` helper exists but unused; 8 local `struct DummyView;` remain. |
| **#70 / #85** | `begin_request_id` helper widely adopted (47 callers) ✅; companion `complete_success_id` never added. |
| **#75** | `#[allow(dead_code)]` cut from 20+ to 4 (all in `mutation_bucket.rs`). |
| **#76** | (same 4 dead-code allows as #75). |
| **#80** | `force_fetch` now wired ✅; `gc_time_ms`/`keep_previous_data`/`refetch_on_*` still documented forward-compat only. |
| **#96** | Mutation callback type aliases added ✅; `QuerySelectResult<T,U,E>` alias for `use_query_select` not added. |
| **#112** | `MutationBucket::touch()` exists but is never called from hooks → `updated_at` still insertion-time. |
| **#125** | Dead `impl Post` methods removed ✅; unused `Post` struct remains. |
| **#134** | `MutationBucket` eviction implemented in prod ✅, but no test exercises >10_000 entries to verify eviction fires. |
| **#28** | 3 of 4 named types got `#[must_use` (`RequestId`, `RequestGuard`, `QueryBeginResult`); `PreparedFetch` missing; ~94 candidate lints remain. |
| **#29** | Hook-side `expect()` fixed; 6 client-side `expect()` (all `TypeId` downcast assertions) remain. |

---

## ❌ Not implemented (31) — grouped by why

### A. Intentionally-deferred big refactors (4)
These were flagged as the largest changes and conservatively skipped (`bucket/shared.rs:1-8` documents the conservative pass).
- **#10** — generic `Bucket<K,R>` abstraction (QueryBucket / InfiniteQueryBucket / MutationBucket still separate; only shared constants extracted).
- **#12** — infinite-query begin/completion consolidation (still 4 `begin_fetch_*` + 2 `complete_*_with_guard` in `infinite_query/lifecycle.rs`).
- **#20** — `MappedQueryResource` still stores `Option<T>`, not `Option<Arc<T>>`. (Resolving this also closes the #4 partial and the #88/#89 misleading items.)
- **#94** — `dehydrate` still has 3 near-identical loops; `dehydrate_bucket_entries(kind,…)` helper not extracted (mutation loop was added per #113).

### B. Missing regression tests (2) — *production fix is done, only the test is missing*
- **#131** — no test that a stored task is cancelled on unmount/replacement. (The agent's note that "#6's production fix is also unfixed" is **wrong** — `CurrentTask` + `set_current_task` exist and work via `Drop`; this is purely a test gap.)
- **#132** — mutation race only tested synchronously; no test fires two `mutate` calls from different async spawn contexts.

### C. Mechanical clippy sweeps — reduced but not zeroed (7)
Ground truth (live `cargo clippy`): 640 total warnings (audit baseline 695).
- **#25** `collapsible_if` — 17 still firing.
- **#136** `items_after_statements` — 83 (was 95).
- **#137** `uninlined_format_args` — 59 (was 70).
- **#138** `manual_let_else` — 31 (was 37).
- **#139** `must_use` candidates — ~94 (was 100).
- **#77** `QueryOptions::default` still allocates the default key (no `const`).
- **#83** clock-before-epoch `.unwrap_or_default()` still undocumented.

### D. Small perf / API / style cleanups (9)
- **#9** — `dehydrate()` still calls full `collect_diagnostics()` (allocates `Vec<QueryDiagnostic>`) but only uses `key`+`status`.
- **#16** — `prepare_fetch_query()` still has the `.then(|| false).unwrap_or(false)` no-op chain (`lifecycle.rs:278-280`).
- **#33** — `RetryPolicy::default` still has redundant `.with_delay(1000).with_max_delay(30_000)`.
- **#38** — `InfiniteQueryResource` derives still multi-line (serde-bound attrs can't merge; trivial).
- **#39** — `match` on 2-variant `RequestPolicy` not simplified to `==` (4 sites).
- **#58 / #59 / #60** — `get_or_create` re-hashes key 3× on dead-ref path; `evict_oldest` clones key per-entry; `all_entities` allocates per call. (Audit itself flagged #58 as non-trivial.)
- **#71** — `Observer::observe()` still `&mut self` (body only reads).
- **#79** — `use_query_manual` / `use_query_unsignalled` still take raw policy params (no `Into<QueryOptions>` overload).
- **#84 / #86 / #87 / #103 / #104** — minor: `AsRef<Arc<str>>` not added (but `message_arc()` exists); `complete_*_with_guard` still takes `&RequestGuard`; per-direction `IgnoreWhileLoading` undocumented; `redact_paths`/`redact_hex` not single-pass (low-impact DevTools path).

### E. Test-suite consolidation deferred (5)
- **#47** — 73 one-off `struct H {}` harness structs; no generic helper.
- **#55** — policy/status/error roundtrip tests not table-driven.
- **#122** — duplicate `nocache_resource` helpers with conflicting signatures not reconciled.
- (plus the partials #46/#48/#52/#70/#125/#134 above, where helpers exist but aren't adopted)

---

## 🧹 New dead code introduced by the fixes (cleanup recommended)

`cargo check` / clippy flag two pairs of now-unused symbols left over from the GC and task work:
- **`maybe_gc`** (×2, `bucket/ops.rs:116` + `infinite_bucket.rs:123`) and **`should_run_opportunistic_gc`** (`bucket/shared.rs:28`) — the bucket-level debounce helpers are **never called**. The real trigger is client-level `maybe_opportunistic_gc` (`mod.rs:142`) which uses `GC_INTERVAL` directly. Safe to delete (or wire `maybe_gc` in if you want per-bucket debouncing instead).
- **`abort_current_task`** (×2, `mutation.rs:286` + `infinite_query/resource.rs:194`) — never called. Cancellation already works because `set_current_task` *replaces* the `Option<Task>`, dropping the previous task, and GPUI cancels dropped tasks. The explicit method is redundant; either delete or use it for clarity.

These are cosmetic; neither is a functional gap.

---

## 🔧 Corrections made to the agent verdicts (cross-checked by hand)

The haiku agents were right ~95% of the time; these were corrected by direct source reads:
- **#6**: agent said "implemented"; accurate status is **implemented for mutation+infinite, deliberately not for queries** (detached by design).
- **#13**: agent said "implemented" then contradicted itself — corrected to **partial** (not merged into `PageDirection`).
- **#96**: agent said "implemented" then noted alias missing — corrected to **partial**.
- **#131**: agent's *status* (not_implemented) is right, but its reasoning ("#6 production fix also unfixed") was **wrong** — `CurrentTask`/`set_current_task` exist and work; this is purely a missing test.
- **#40**: pattern remains but the proposed `is_some_and` fix doesn't apply (the closure transforms the value); effectively **no-action-needed** rather than a real gap.
- Clippy counts taken from a fresh `cargo clippy` run (authoritative), not the agent's reported counts.

---

## Recommended next steps (priority order)

1. **Add the two missing regression tests** (#131 task-cancellation, #132 mutation async-race) — cheap, locks in the two most important correctness fixes.
2. **Delete the new dead code** (`maybe_gc`, `should_run_opportunistic_gc`, `abort_current_task`) — zero-risk, silences 6 warnings.
3. **One targeted perf refactor: `Arc<T>` in `MappedQueryResource` (#20)** — closes #4 partial and resolves the #88/#89 items in one move.
4. **Finish the clippy sweeps** (#25, #77, #83, #136–#139) — bulk mechanical, gets warning count near zero.
5. **Small cleanups**: #16 no-op chain, #33 retry default, #71 `&self`, #122 helper rename.
6. **(Optional, large)** generic `Bucket<K,R>` (#10) + infinite begin consolidation (#12) — only if pursuing the architectural recommendations; currently stable and documented as deferred.

---

## Per-area detail

<details>
<summary><b>core (31 findings)</b></summary>

Implemented (12): C1, C2, #23, #31, #35, #57, #78, #98, #99, #100, #101, #102
Partial (2): #36, #82
Not implemented (10): #12, #20, #33, #38, #39, #84, #86, #87, #103, #104
N/A (7): #19 (inaccurate), #32, #34 (inaccurate), #37, #88, #89, #90

</details>

<details>
<summary><b>client (32 findings)</b></summary>

Implemented (21): CL1, CL2, #1, #2, #8, #11, #17, #18, #21, #69, #91, #92, #93, #105, #106, #107, #108, #109, #111, #113, #114
Partial (2): #75, #112
Not implemented (8): #9, #10, #16, #58, #59, #60, #71, #94
Already-fixed (1): #110

</details>

<details>
<summary><b>hook (38 findings)</b></summary>

Implemented (29): CL7, #3, #5, #6, #7, #14, #15, #24, #27, #30, #40, #42, #43, #44, #45, #61, #62, #63, #67, #68, #72, #73, #115, #116, #117, #118, #119, #120, #121
Partial (5): #4, #13, #41, #80, #96
Not implemented (1): #79
N/A (2): #74, #97
Already-fixed (1): #64

</details>

<details>
<summary><b>tests (29 findings)</b></summary>

Implemented (15): #49, #50, #51, #53, #54, #65, #85, #95, #123, #124, #126, #127, #128, #129, #133
Partial (6): #46, #48, #52, #70, #125, #134
Not implemented (5): #47, #55, #122, #131, #132
N/A (3): #66, #130, #135

</details>

<details>
<summary><b>clippy / mechanical (14 findings)</b></summary>

Implemented (3): #22, #26, #56
Partial (3): #28, #29, #76
Not implemented (7): #25, #77, #83, #136, #137, #138, #139
N/A (1): #81

</details>
