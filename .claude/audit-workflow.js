export const meta = {
  name: 'features-audit',
  description: 'Audit docs/features.md implementation against code + rust/gpui standards',
  phases: [
    { title: 'Audit', detail: 'one agent per track verifies features.md claims + standards' },
    { title: 'Verify', detail: 'adversarially confirm each correctness/standards issue' },
  ],
}

// ---- Standards context embedded into every auditor -----------------------------------------
const STANDARDS = `
STANDARDS TO AUDIT AGAINST (apply all that are relevant per track):

[rust-best-practices — Apollo handbook]
- ch4: Return Result<T,E>; NEVER panic in prod; NEVER unwrap()/expect() outside #[cfg(test)]. Use thiserror for library errors (typed enums covering ALL failure modes). Prefer ? over match chains.
- ch6: Prefer generics (static dispatch) for perf-critical paths; Box<dyn>/&dyn only for heterogeneous collections or runtime dispatch. Box at API boundaries, not internally.
- ch8: #![deny(missing_docs)] on libraries; /// doc comments on ALL public items (what/how); // comments explain WHY (safety, workarounds, rationale). Every TODO needs a linked issue.
- ch9: Arc<T> for shared resources across threads; Send/Sync correctness; prefer &T over .clone(); &str/&[T] in params; small Copy types (<=24B) by value.
- impl Future<Output=..> + Send on trait methods (NOT async fn in traits) to GUARANTEE spawnable Send futures.

[rust-async-patterns]
- NEVER hold a std::sync::Mutex / lock guard across a .await. Pattern: lock/read/drop-lock → await → lock/write/drop-lock.
- Spawned futures must be Send + 'static. Don't block in async (no std::thread::sleep, no blocking IO).
- Propagate errors with ?; use tracing; JoinSet for many tasks; channels over shared state; handle cancellation.

[rust-testing]
- #[cfg(test)] unit modules; descriptive test names (behavior); one logical assertion per test; test behavior not implementation.
- Test error paths: prefer Result::is_err() over #[should_panic].
- #[gpui::test] + TestAppContext for code touching Global/Entity; PLAIN #[test] for pure logic (header parsing, debounce math).
- proptest for round-trips/invariants; doc-tests for public API examples.
- NEVER use sleep() in tests — use channels/barriers/tokio::time::pause(). Integration tests in tests/ dir.

[gpui-global]
- impl Global for T; cx.set_global / cx.global / cx.update_global::<T,_>.
- CRITICAL: cx.update_global::<T,_> pushes a NotifyGlobalObservers effect → it DOES auto-fire cx.observe_global::<T>() subscribers. But it does NOT cx.notify() the calling entity itself.
- So: a mutation written via update_global::<QueryClient,_> IS seen by observe_global::<QueryClient>(); a mutation written via bare entity.update(...)+cx.notify() on a resource entity is NOT seen by observe_global.

[gpui-async]
- cx.spawn → foreground task (may update entities). cx.background_spawn → worker thread (MUST NOT update entities directly).
- Entity updates happen on the foreground thread: entity.update(cx, |state, cx| { ...; cx.notify(); }).
- Use WeakEntity / entity.downgrade() inside spawned tasks to avoid retaining entities; tasks cancel on drop, store the Task to keep alive.
- background_spawn(...).then(cx.spawn(move |result, cx| { entity.update(...) })) brings results to the foreground.

[gpui-test]
- #[gpui::test] fn t(cx: &mut TestAppContext); async fn t(cx: &mut TestAppContext); #[gpui::test(iterations = N)] for property.
- Rule: if a test needs NO window/rendering, a plain rust #[test] is acceptable. Touching a Global (QueryClient) REQUIRES #[gpui::test] + TestAppContext.
`

const AUDIT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['track', 'summary', 'claims', 'issues'],
  properties: {
    track: { type: 'string' },
    summary: { type: 'string', description: '2-4 sentence verdict for this track' },
    claims: {
      type: 'array',
      description: 'Each features.md claim verified against code',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['claim', 'status', 'evidence'],
        properties: {
          claim: { type: 'string' },
          status: { type: 'string', enum: ['implemented', 'partial', 'missing'] },
          evidence: { type: 'string', description: 'file:line or symbol proving the status' },
          notes: { type: 'string', description: 'caveats, divergence from design, or empty' },
        },
      },
    },
    issues: {
      type: 'array',
      description: 'Concrete problems found (correctness bugs, standards violations, missing tests, doc drift). Empty if none.',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['severity', 'kind', 'title', 'detail', 'evidence'],
        properties: {
          severity: { type: 'string', enum: ['high', 'medium', 'low'] },
          kind: { type: 'string', enum: ['correctness', 'standards', 'missing-test', 'doc-drift'] },
          title: { type: 'string' },
          detail: { type: 'string', description: 'what is wrong, why it matters, and the standards/design reference' },
          evidence: { type: 'string', description: 'file:line proving the issue' },
          standards_ref: { type: 'string', description: 'which standard/skill it violates, or empty' },
        },
      },
    },
  },
}

const VERIFY_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['issue_title', 'is_real', 'confidence', 'reasoning'],
  properties: {
    issue_title: { type: 'string' },
    is_real: { type: 'boolean', description: 'true only if you confirmed the issue against the actual source' },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
    reasoning: { type: 'string', description: 'cite file:line you read to decide' },
    correction: { type: 'string', description: 'if not real (or only partly), what is actually true' },
  },
}

const COMMON = `
You are auditing the gpui-query Rust workspace (working dir /Users/hmziq/fo/gpui-query).
Goal: determine whether each features.md design claim is IMPLEMENTED and whether it is CORRECT and meets the standards.

METHOD (do all of this, do not skim):
1. Read the exact source files named below with the Read tool. Use Grep to locate symbols across the crate when needed.
2. For each claim, find the concrete code that implements it (or prove it is absent). Record file:line.
3. Check correctness against the design intent AND the standards block below.
4. Be precise and skeptical. "implemented" means it genuinely works as the design specifies — not just that a symbol exists. If the impl diverges from the design in a way that matters, mark status 'partial' and file an issue.
5. Distinguish REAL problems from style nits. Only file an issue if a maintainer should act on it. Cite file:line for every issue.
6. Do NOT invent file:line — only cite lines you actually read. If you cannot find evidence, say so in notes and set status accordingly.

${STANDARDS}
`

const TRACKS = [
  {
    key: '1-fetched-server-wins',
    prompt: `${COMMON}
TRACK 1 — Required Core Change 1: Fetched<T,E> + use_query_with_policy (HTTP "server wins").

FEATURES.MD CLAIMS TO VERIFY:
A. A \`Fetched<T,E>\` result type exists in core with fields: \`data: T\`, \`cache_policy: Option<CachePolicy>\` (None → keep caller's policy), \`meta: Option<serde_json::Value>\` (proposed CacheMeta round-trip), and \`_error: PhantomData<E>\` (design Option A).
B. A \`use_query_with_policy\` hook mirrors the shipped \`use_query\`: same \`(Entity<QueryResource<T,E>>, Subscription)\` tuple return, same QuerySignal-accepting fetcher shape (\`F: Fn(QuerySignal) -> Fut + Send + 'static\`), except the fetcher returns \`Result<Fetched<T,E>, E>\`.
C. On success the bucket/resource APPLIES the returned \`cache_policy\` (Some overrides, None keeps caller's). This is the "server wins" path.
D. The existing \`Result<T,E>\` \`use_query\` is UNCHANGED — the new variant is additive/non-breaking.
E. \`Fetched.meta\` has a real persistence target (depends on Core Change 3's value-carrying entry existing).

FILES TO READ: crates/gpui-query/src/core/fetched.rs, crates/gpui-query/src/hook/query_hooks.rs, crates/gpui-query/src/hook/options.rs, crates/gpui-query/src/core/resource/{lifecycle.rs,cache.rs,accessors.rs}, crates/gpui-query/src/lib.rs, and tests crates/gpui-query/src/tests/hook_tests/query_tests/with_policy.rs.

Pay special attention to: how the fetcher's returned CachePolicy reaches the resource (is there a set_cache_policy call on success?), whether the Tuple return + signal-always bound are preserved, and whether with_policy has test coverage for Some-override and None-keep.`,
  },
  {
    key: '2-completion-notify',
    prompt: `${COMMON}
TRACK 2 — Required Core Change 2: whole-client mutation observation so persist_with sees COMPLETIONS. THIS IS THE MOST CORRECTNESS-CRITICAL TRACK.

BACKGROUND (from features.md, verified reasoning): QueryClient is a Global. cx.update_global::<QueryClient,_> DOES auto-fire observe_global::<QueryClient>() subscribers (pushes NotifyGlobalObservers). BUT the crate's update_global sites fire at fetch-start/hook-setup, NOT at completion. The completion writes (complete_success/complete_failure) are bare entity.update(...)+cx.notify() on the resource ENTITY — which wakes observe(&entity) subscribers but NOT observe_global. So a persist_with built on observe_global would MISS every completion unless the completion sites are changed.

FEATURES.MD CLAIMS TO VERIFY (the design recommended Option B: a typed dirty signal emitted from the completion sites; Option A: route completions through update_global):
A. The THREE completion paths now notify the QueryClient global (or emit a dedicated dirty/changed signal) so a global-observing persist_with actually sees fetched data land:
   - hook/fetch_retry.rs (complete_success / complete_failure for queries)
   - hook/mutation_hooks/internals.rs (mutation completion)
   - hook/use_infinite_query/fetch_runners.rs (infinite completion)
B. set_query_data (client/mod.rs) still notifies (it's &mut self on a Global, reached via update_global).
C. persist_with (client/persist.rs) actually subscribes to whatever notify mechanism now exists, so that when a fetch resolves the debounced save fires with the new data.

Determine WHICH option was implemented (A: update_global at completions; B: typed dirty signal; or neither/the gap is still open). This determines whether persistence actually captures completions or silently drops them. Read the actual completion sites and trace whether observe_global::<QueryClient>() (or equivalent) fires on completion.

FILES TO READ: crates/gpui-query/src/hook/fetch_retry.rs, crates/gpui-query/src/hook/mutation_hooks/internals.rs, crates/gpui-query/src/hook/use_infinite_query/fetch_runners.rs, crates/gpui-query/src/hook/mutation_hooks/hooks.rs, crates/gpui-query/src/client/mod.rs (set_query_data + any dirty signal), crates/gpui-query/src/client/persist.rs.

If completions are NOT observable by persist_with, that is a HIGH severity correctness issue.`,
  },
  {
    key: '3-persist-surface',
    prompt: `${COMMON}
TRACK 3 — Required Core Change 3 + persistence surface (async Persister, value-carrying entries, persist_with, hydrate, versioning, options).

FEATURES.MD CLAIMS TO VERIFY:
A. An async \`Persister\` trait whose load/save return \`impl Future<Output=..> + Send\` (NOT plain async fn — to guarantee Send/spawnable, and therefore NOT object-safe; design says use generics persist_with<P>, with Pin<Box<dyn Future+Send>> as a documented fallback for runtime dispatch). Verify the actual return-type choice and whether object-safety is handled sanely.
B. \`PersistedEntry { value: serde_json::Value, cached_at: u64 (epoch-millis, SystemTime-based), cache_policy: CachePolicy, meta: Option<serde_json::Value> }\` — the value-carrying entry the shipped DehydratedEntry lacked.
C. \`PersistSnapshot { entries: HashMap<QueryKey, PersistedEntry>, version: u32 }\`.
D. \`PersistOptions { filter, max_age, debounce }\` where \`filter\` is an OWNED filter — it MUST NOT reuse \`QueryKeyFilter<'a>\` directly (that borrows &'a QueryKey and is not Serialize). Verify the owned-filter choice.
E. \`persist_with<P: Persister>(&self, p, opts) -> PersistHandle\` — debounced + filtered subscription to whole-client mutations; PersistHandle drops to unsubscribe.
F. A typed \`hydrate\` (not the shipped no-op) that loads entries and primes each via QueryClient::set_query_data on the foreground thread, returning Result<usize, PersistError>.
G. Version mismatch on load surfaces as a typed PersistError::VersionMismatch.
H. The shipped synchronous QueryPersister/DehydratedEntry skeleton still exists (or was it replaced?).

FILES TO READ: crates/gpui-query/src/client/persist.rs, crates/gpui-query/src/client/lifecycle.rs, crates/gpui-query/src/client/erased.rs, crates/gpui-query/src/client/devtools.rs, crates/gpui-query/src/core/key_filter.rs, crates/gpui-query/src/lib.rs.

Standards focus: async Send bounds, no lock held across await in the debounce/spawn path, typed errors via thiserror, no unwrap/expect outside tests, doc comments on all public items.`,
  },
  {
    key: '4-http-crate',
    prompt: `${COMMON}
TRACK 4 — gpui-query-http crate: CacheMeta, cache_policy_from_headers, HttpCache.

FEATURES.MD CLAIMS TO VERIFY:
A. \`CacheMeta\` is #[derive(Clone, Debug, Serialize, Deserialize)] with fields: etag: Option<String>, last_modified: Option<String>, stored_at: SystemTime (MUST be SystemTime, NEVER Instant — Instant has no serde impl and is meaningless across restarts), fresh_for: Duration, stale_for: Duration.
B. \`cache_policy_from_headers(&http::HeaderMap) -> Result<CachePolicy, ParseError>\` parsing Cache-Control / ETag / Last-Modified / Age; returns a typed ParseError.
C. \`HttpCache<C = reqwest::Client>\` generic over the client (default reqwest) so tests can inject a stub; owns \`meta: Mutex<HashMap<Url, CacheMeta>>\` and \`bodies: Mutex<HashMap<Url, Bytes>>\` using std::sync::Mutex.
D. \`fetch(&url) -> Result<(Bytes, CachePolicy, Option<CacheMeta>), HttpError>\`: on 200 stores body+meta and returns (body, policy, Some(meta)); on 304 returns the cached body + refreshed policy + stored meta; on non-cacheable returns (body, NoCache, None).
E. The std Mutexes are NEVER held across a .await (lock/read/drop then await then lock/write/drop), keeping HttpCache: Send+Sync without a tokio mutex.
F. On refetch, conditional request headers (If-None-Match from etag, If-Modified-Since from last_modified) are attached so 304 works.
G. reqwest/http/bytes deps live ONLY in gpui-query-http, never in core gpui-query (Guiding Principle 1).

FILES TO READ: crates/gpui-query-http/src/{lib.rs,cache.rs,backend.rs,reqwest_backend.rs}, crates/gpui-query-http/Cargo.toml. Also grep crates/gpui-query/src for any reqwest/http/bytes leak into core.

Standards focus: no Mutex held across await (CRITICAL async correctness), Send+Sync, generic-over-client for test injection, typed HttpError/ParseError via thiserror, SystemTime not Instant, no unwrap/expect outside tests, #![deny(missing_docs)].`,
  },
  {
    key: '5-file-persister-disk',
    prompt: `${COMMON}
TRACK 5 — gpui-query-persist FilePersister disk robustness (the headline adapter). This track is about cross-platform durability/correctness, the most failure-prone area.

FEATURES.MD CLAIMS TO VERIFY:
A. OS-appropriate location via the \`dirs\` crate (data_dir OR cache_dir), joined with <app>; always through PathBuf; when dir is None (sandboxed/headless) returns PersistError::BadPath — NEVER unwraps the dir.
B. ATOMIC writes via tempfile's NamedTempFile::persist (rename(2) on POSIX, MoveFileExW/ReplaceFile on Windows) — does NOT call raw std::fs::rename (which fails ERROR_ACCESS_DENIED on Windows when a reader/AV holds the destination, rust#123985). A Windows access-denied must map to a retryable PersistError::Permission.
C. DURABILITY: fsync the temp file BEFORE the replace (F_FULLFSYNC on macOS where plain fsync is advisory), and fsync the PARENT directory fd AFTER the replace on POSIX. Never overwrite the target in place.
D. TOLERANT load: missing file = empty store (first run), NOT an error; corrupt/partial file = logged + treated as empty, NEVER panics/never aborts; version mismatch = typed PersistError::VersionMismatch.
E. \`PersistFormat::Json\` (default, human-readable) and \`PersistFormat::Bincode\` (compact) both round-trip the same PersistSnapshot.
F. CONCURRENCY: writes serialized (one save at a time) via a serialization Mutex or chained in-flight future; load is independent; NO lock held across the gpui<->tokio boundary.
G. Typed \`PersistError\` enum with variants covering: Io, Serialize, Deserialize, VersionMismatch, Permission, BadPath (at least). No unwrap/expect anywhere.
H. NoopPersister exists for tests/in-memory.

FILES TO READ: crates/gpui-query-persist/src/lib.rs, crates/gpui-query-persist/tests/file_persister.rs, crates/gpui-query-persist/Cargo.toml.

Standards focus: ch4 typed errors, no unwrap/expect, thiserror, no blocking IO in async. Check carefully whether fsync/F_FULLFSYNC/parent-dir-fsync are actually present or were skipped (a common shortcut). Check whether atomic write really uses NamedTempFile::persist vs a raw rename.`,
  },
  {
    key: '6-feature-gating',
    prompt: `${COMMON}
TRACK 6 — Feature gating correctness (the three gating prerequisites + dep promotion).

FEATURES.MD CLAIMS TO VERIFY:
A. A \`persist\` feature exists and gates the persistence surface (offline-data only). Today's skeleton shipped unconditionally under client; the proposal was to gate it. Verify the gate actually compiles OUT the persistence code when persist is off (cfg correctness), and that enabling persist pulls serde_json + thiserror.
B. PREREQ 1: \`current_time_ms\` was EXTRACTED out of client/erased.rs (it is consumed by non-persistence client code: gc, lifecycle, infinite_mutation_ops) so the module can be gated. Verify it lives in its own module (client/time.rs) and erased.rs no longer defines it.
C. PREREQ 2: \`collect_key_status_into\` — a required method on the always-compiled ErasedBucket/ErasedInfiniteBucket/ErasedMutationBucket traits whose only caller is dehydrate. Verify the gating decision (method + impls gated together, OR deliberately left when persist off). Flag if it's now dead code with persist off.
D. PREREQ 3: the itemized \`pub use devtools::{…}\` and \`pub use erased::{…}\` lists in client/mod.rs are cfg-gated in lockstep with the gated symbols.
E. serde_json promoted from dev-dependency to a real (gated under persist) dep because PersistedEntry.value/Fetched.meta are typed serde_json::Value.
F. The hook layer's own current_time_ms (hook/mod.rs) is untouched (lib.rs re-exports both) — gating erased.rs must not break the hook layer.
G. QueryClient::diagnostics() is unaffected by the gate.

FILES TO READ: crates/gpui-query/Cargo.toml, crates/gpui-query/src/lib.rs, crates/gpui-query/src/client/mod.rs, crates/gpzi-query/src/client/time.rs (NOTE: path is crates/gpui-query/src/client/time.rs), crates/gpui-query/src/client/erased.rs, crates/gpui-query/src/client/devtools.rs, crates/gpui-query/src/hook/mod.rs, crates/gpui-query/src/client/bucket/erased_ops.rs.

Try to confirm the crate compiles both with and without persist by reasoning about the cfg attributes (do not run cargo unless quick). Report any symbol that leaks out of the gate.`,
  },
  {
    key: '7-test-strategy',
    prompt: `${COMMON}
TRACK 7 — Test strategy compliance (audit the tests against features.md's Test Strategy table + rust-testing/gpui-test standards).

FEATURES.MD CLAIMS TO VERIFY (each row of the Test Strategy table):
A. Shipped persistence skeleton: QueryPersister load/save round-trip; restore returns entries; dehydrate→persist→restore→caller set_query_data priming. These touch QueryClient so MUST use #[gpui::test] + TestAppContext.
B. Core 1 (Fetched / *_with_policy): Fetched policy override + None-keeps unit tests; *_with_policy end-to-end. gpui ctx required.
C. Core 3 (upgraded persistence): debounce coalescing, max_age filtering, version reject; hydrate→persist→hydrate round-trip. gpui ctx required.
D. gpui-query-http: header-parsing edge cases as PLAIN #[test] (no gpui ctx); 304 / SWR / no-store integration via a mock server (mockito or equivalent) on tokio.
E. gpui-query-persist: FilePersister round-trip + concurrent saves; large snapshot + schema migration. No gpui ctx needed.
F. cross-crate end-to-end (HTTP→CacheMeta in PersistedEntry::meta→restart→hydrate→If-None-Match→304). Per design this lives in the consuming app; note if absent here.
G. No sleep() used in tests (use channels/barriers/async); tests are independent (no shared mutable state); descriptive names.

ALSO CHECK overall: is there a proptest presence? doc-tests on public API? Are error paths tested (Result::is_err over should_panic)? Are tests actually wired into the test module tree (mod.rs chains)?

FILES TO READ: crates/gpui-query/src/tests/integration_client_coverage/client_operations/{diagnostics_dehydrate_persister.rs,persist_with_hydrate.rs}, crates/gpui-query/src/tests/hook_tests/query_tests/with_policy.rs, crates/gpui-query/src/tests/test_support.rs, crates/gpui-query/src/tests/mod.rs (module wiring), crates/gpui-query-persist/tests/file_persister.rs, and grep the gpui-query-http crate for any #[test]/#[cfg(test)] (it had no tests/ dir). Also grep all test files for "sleep(" and "std::thread::sleep".

Standards focus: rust-testing + gpui-test. Flag missing coverage as missing-test issues with severity proportional to the risk (e.g. no 304/no-store test in http = medium; no debounce test = high).`,
  },
  {
    key: '8-standards-sweep',
    prompt: `${COMMON}
TRACK 8 — Cross-cutting standards sweep over the two companion crates + the persist-feature code (rust-best-practices ch4/6/8/9 + async).

FEATURES.MD CLAIMS TO VERIFY (Guiding Principle 8):
A. #![deny(missing_docs)] is set on gpui-query-http AND gpui-query-persist (crate-level, in lib.rs). Every public item has /// docs.
B. Typed errors via thiserror in BOTH companion crates (HttpError, ParseError, PersistError, etc.) — no anyhow, no String errors, no panic-based error paths.
C. Arc<T> used for shared resources (e.g. Arc<HttpCache> pattern is feasible; FilePersister path ownership sane).
D. NO unwrap()/expect()/panic!/unreachable!/todo! outside #[cfg(test)] across the companion crates AND crates/gpui-query/src/client/persist.rs AND crates/gpui-query/src/core/fetched.rs. (This is an explicit design rule.) Enumerate every occurrence you find with file:line.
E. impl Future<Output=..> + Send on async trait methods (not async fn in traits) where Send futures are needed for spawning — verify the Persister trait shape.
F. thiserror is actually a dependency of the crates that use it.

FILES TO READ: crates/gpui-query-http/src/lib.rs, crates/gpui-query-persist/src/lib.rs, crates/gpui-query/src/client/persist.rs, crates/gpui-query/src/core/fetched.rs, both companion Cargo.toml files. Then GREP the whole crates/gpui-query-http/src, crates/gpui-query-persist/src, crates/gpui-query/src/client/persist.rs, crates/gpui-query/src/core/fetched.rs for: \\.unwrap\\(\\), \\.expect\\(, panic!, unreachable!, todo!, unimplemented!, anyhow.

Report every unwrap/expect/panic occurrence as an issue (severity by context: high if it can fire on normal runtime input, low if guarded). This is the most concrete, checkable track — be exhaustive with file:line.`,
  },
]

phase('Audit')

const results = await pipeline(
  TRACKS,
  // stage 1 — audit each track
  (track) => agent(track.prompt, {
    label: `audit:${track.key}`,
    phase: 'Audit',
    schema: AUDIT_SCHEMA,
  }),
  // stage 2 — adversarially verify every issue this track raised (as soon as the audit lands)
  (audit, track) => {
    if (!audit || !audit.issues || audit.issues.length === 0) {
      return { track: track.key, audit, verifiedIssues: [] }
    }
    return parallel(
      audit.issues.map((issue) => () =>
        agent(
          `${COMMON}
You are an ADVERSARIAL VERIFIER. Another auditor filed this issue against track "${track.key}". Your job is to REFUTE it if possible — default to is_real=false unless you can independently confirm it by reading the actual source at the cited location. Re-read the file:line yourself.

ISSUE:
  title: ${issue.title}
  severity: ${issue.severity}
  kind: ${issue.kind}
  detail: ${issue.detail}
  evidence (claimed): ${issue.evidence}
  standards_ref: ${issue.standards_ref || '(none)'}

Decide is_real: true ONLY if the issue is genuinely present and materially correct. If the auditor misread the code, the cited line does not exist, or the issue is a non-actionable nit, set is_real=false and explain in correction. Set confidence honestly.`,
          { label: `verify:${track.key}`, phase: 'Verify', schema: VERIFY_SCHEMA }
        ).then((v) => ({ ...issue, verdict: v }))
      )
    ).then((verifiedIssues) => ({ track: track.key, audit, verifiedIssues: verifiedIssues.filter(Boolean) }))
  }
)

return results
