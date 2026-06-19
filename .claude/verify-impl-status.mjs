export const meta = {
  name: 'verify-audit-impl-status',
  description: 'Verify the gpui-query audit implementation-status doc claims against the live source tree',
  phases: [
    { title: 'Review', detail: 'one agent per claim cluster reads actual source vs claimed changes' },
    { title: 'Verify', detail: 'skeptic independently re-checks each DONE/PARTIAL verdict' },
    { title: 'Synthesize', detail: 'combine into a single accuracy verdict' },
  ],
}

const ROOT = '/Users/hmziq/fo/gpui-query/crates/gpui-query/src'

const CLUSTERS = [
  {
    key: 'criticals-gc',
    label: 'criticals-and-GC',
    prompt: `Verify these claims from docs/audits/gpui-query-audit-implementation-status.md against the ACTUAL source in ${ROOT}. For each, read the cited file/symbol and decide DONE / PARTIAL / NOT_DONE / MISLEADING. Quote file:line evidence.

Claims (claimed DONE):
- C1/#98: redact_tokens uses Vec<char> char-indexed scan; non-ASCII panic fixed. Cited sanitize.rs:147-181 with regression tests :338-353. CHECK core/error/sanitize.rs.
- C2/#99: sanitize_message truncation uses char_indices().rfind(); mid-char panic fixed. Cited sanitize.rs:44-49. CHECK core/error/sanitize.rs.
- CL1/#105: GC runs from production via maybe_opportunistic_gc() every 64 ops. Cited client/mod.rs:142-156,210 and infinite_mutation_ops.rs:74,179. CHECK client/mod.rs and client/infinite_mutation_ops.rs.
- CL2/#106: StatusSnapshot REMOVED entirely; GC reads live entity state. Cited client/bucket/ops.rs:154-196. CHECK client/bucket/ (find ops.rs) and grep for "StatusSnapshot" across the whole src tree (should be gone).
- CL7/#111: deprecated use_mutation_with_options delegates to use_mutation. Cited hook/mutation_hooks/hooks.rs:130-141. CHECK that file.
- #1: InfiniteQueryBucket gained max_entries + evict_oldest + SUCCESS_GC_MULTIPLIER. Cited client/infinite_bucket.rs:39,102,156-160.
- #2: MutationBucket gained max_entries + evict_oldest + insert-limit check. Cited client/mutation_bucket.rs:77,111,147-148.`,
  },
  {
    key: 'task-and-races',
    label: 'task-cancellation-races',
    prompt: `Verify these claims against the ACTUAL source in ${ROOT}. Read cited symbols; report DONE/PARTIAL/NOT_DONE/MISLEADING with file:line evidence.

- #6 (claimed DONE for mutation+infinite): CurrentTask wrapper exists (core/mod.rs:76-97); set_current_task on MutationResource and InfiniteQueryResource; tasks cancelled on replacement/unmount via Drop. Query sites DELIBERATELY detached (hook/query_hooks.rs:96-102,143-148). VERIFY the wrapper, the set_current_task methods, and confirm query spawn sites are detached by reading them. Note: doc's status section flags #6 as "implemented for mutation+infinite, deliberately not for queries" — confirm.
- #7 (claimed DONE): mutation race fixed — is_loading check now atomic inside the same entity.update as begin. Cited hook/mutation_hooks/hooks.rs:353-361. CHECK.
- #131 (claimed DONE as regression test): a test that a stored task is cancelled on unmount/replacement. Search src/tests for it.
- #132 (claimed DONE as regression test): a test firing two mutate calls from different async spawn contexts (cross-context TOCTOU). Search src/tests for it.`,
  },
  {
    key: 'second-pass',
    label: 'second-pass-specifics',
    prompt: `Verify these "second implementation pass" claims against the ACTUAL source in ${ROOT}. These are the most likely to be aspirational — read each cited symbol carefully. Report DONE/PARTIAL/NOT_DONE/MISLEADING with file:line evidence + a short code quote.

- #86 (claimed DONE): complete_*_with_guard methods now CONSUME RequestGuard by value (not &RequestGuard). Check core/infinite_query/lifecycle.rs and core/resource/completion.rs for the signatures. Also confirm call sites pass by value (grep for complete_success_with_guard / complete_failure_with_guard callers).
- #112 (claimed DONE): MutationBucket GC measures recency from MutationResource's last terminal-completion time (last_updated_at_ms), stamped in complete_success/complete_failure, reset on begin; falls back to insertion time for never-completed. Check core/mutation.rs (or wherever MutationResource lives) + client/mutation_bucket.rs GC.
- #9/#16/#94 (claimed DONE): dehydrate uses lightweight collect_key_status (no per-entry QueryDiagnostic alloc); prepare_fetch_query captures the real started bool; the 3 dehydrate loops collapsed into one. Check core/resource/ and client/lifecycle.rs + grep collect_key_status / dehydrate / prepare_fetch_query.
- #12/#39/#87 (claimed DONE): infinite-query begin methods consolidated behind a PRIVATE begin_fetch(direction,…) (public signatures preserved); match→== for RequestPolicy. Check core/infinite_query/lifecycle.rs + hook/use_infinite_query/. grep begin_fetch.
- #20/#96 (claimed DONE): MappedQueryResource stores Option<Arc<T>> (cheap clones); QuerySelectResult<T,U,E> type alias added. Check core/select.rs. CRITICAL: the pre-implementation baseline said #20 was NOT done (stored Option<T>) — confirm whether it's NOW done per the top-of-doc claim.
- #13 (claimed DONE): fetch_next/previous runners unified via PageDirection enum + single helper. Check hook/use_infinite_query/fetch_runners.rs. grep PageDirection. (Baseline said this was PARTIAL — confirm it's now fully merged.)
- #58/#59/#60/#41/#71/#29/#75 (claimed DONE): bucket hash/clone reductions; consolidated current_time_ms; Observer::observe → &self; client .expect()→safe fallbacks; dead-code allows removed. Spot-check: Observer::observe signature in client/observer.rs (should be &self); current_time_ms consolidation; get_or_create re-hash reduction in client/bucket/.
- #10 cleanup (claimed DONE): the dead symbols maybe_gc, should_run_opportunistic_gc, abort_current_task were DELETED. grep the whole src tree for each — they should be GONE.`,
  },
  {
    key: 'documented-exceptions',
    label: 'documented-exceptions',
    prompt: `The doc lists 3 "documented exceptions" that were intentionally NOT force-completed. Verify each is genuinely in the claimed state (i.e. the doc is honest about what's missing), reading the ACTUAL source in ${ROOT}. Report DONE(honest)/MISLEADING for each.

- #10 generic Bucket<K,R> (claimed INFEASIBLE): doc says MutationBucket uses u64 keys + 3 type params (V,T,E) vs QueryKey + 2 (T,E) for the others, and a dyn ErasedBucket dispatch layer prevents clean unification; bucket/shared.rs consolidates constants only. Confirm: are the 3 buckets still separate? Does bucket/shared.rs exist with only shared constants? Is there a dyn ErasedBucket?
- #38 (claimed INEVITABLE): InfiniteQueryResource's #[serde(bound(serialize=…))] and (deserialize=…) attributes cannot merge onto the #[derive] line. Confirm they're still separate multi-line attrs in core/infinite_query/resource.rs.
- #4 (claimed PARTIAL): Arc::ptr_eq skip-the-clone is unachievable because QueryResource stores T by value (lends &T); the achievable part — Arc<T> storage in MappedQueryResource — IS done. Confirm use_query_select still does data().cloned() (the T clone remains) AND that MappedQueryResource uses Option<Arc<T>> (overlaps with #20).`,
  },
  {
    key: 'prod-invariants',
    label: 'correctness-invariants-and-clippy-doc',
    prompt: `Verify these additional "big wins" claims against the ACTUAL source in ${ROOT}. Report DONE/PARTIAL/NOT_DONE/MISLEADING with file:line evidence.

- #3: mutate_by_ref / mutate_arc added; retry loop borrows &V from Arc<V>, no per-attempt clone. Cited hook/mutation_hooks/internals.rs:54-66 and hooks.rs:396-454. grep mutate_by_ref/mutate_arc.
- #5: infinite pages stored as VecDeque<Arc<T>>; fetchers get Option<&T> via Arc deref, no page clone. Cited core/infinite_query/resource.rs:54 and hook/use_infinite_query/fetch_runners.rs:45-51.
- #8: retain/release/observer_count dead code removed; GC relies on WeakEntity::upgrade() liveness. grep for fn retain / fn release / observer_count across src (should be gone in client/).
- #14: two query retry loops collapsed into one run_query_retry_loop parameterized by Option<QuerySignal>. Cited hook/fetch_retry.rs:108-233.
- #15: mutation retry loops collapsed into one run_mutation_loop_inner + 4 thin wrappers. Cited hook/mutation_hooks/internals.rs:54-237.
- #72: cx.notify() moved inside the accept_current_request success branch. Cited hook/use_infinite_query/fetch_runners.rs (also query runners).
- #73: infinite runners verify is_current_request(request_id) after retry delay. Cited fetch_runners.rs:107,224.
- #100/#101/#102: rollback_to_previous clears error; clear_data transitions Success→Idle; record_stale_cache_hit sets Success. Check core/resource/.
- #107/#108/#109/#110/#113/#114: GC evicts Cancelled; evicts aged Success mutations; evict_oldest skips in-flight; dehydrate includes mutations; insert uses its own cx. Check client/bucket/ + mutation_bucket.rs.
- Also: doc claims clippy "no dead-code warnings". Run: grep -rn "#\[allow(dead_code)\]" ${ROOT} and report how many remain (doc's top says "no dead-code warnings" but its own partials section says 4 remain in mutation_bucket.rs — flag the contradiction).`,
  },
]

const FINDING_SCHEMA = {
  type: 'object',
  properties: {
    cluster: { type: 'string' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          claim_id: { type: 'string' },
          claimed_status: { type: 'string' },
          actual_status: { type: 'string', enum: ['DONE', 'PARTIAL', 'NOT_DONE', 'MISLEADING', 'HONEST-EXCEPTION'] },
          holds: { type: 'boolean', description: 'does actual match what the doc claims?' },
          evidence: { type: 'string', description: 'file:line + short code quote proving the verdict' },
          notes: { type: 'string' },
        },
        required: ['claim_id', 'claimed_status', 'actual_status', 'holds', 'evidence', 'notes'],
      },
    },
  },
  required: ['cluster', 'findings'],
}

const VERDICT_SCHEMA = {
  type: 'object',
  properties: {
    claim_id: { type: 'string' },
    agrees: { type: 'boolean', description: 'does the skeptic agree with the reviewer verdict?' },
    corrected_status: { type: 'string', enum: ['DONE', 'PARTIAL', 'NOT_DONE', 'MISLEADING', 'HONEST-EXCEPTION'] },
    reason: { type: 'string' },
  },
  required: ['claim_id', 'agrees', 'corrected_status', 'reason'],
}

phase('Review')

const results = await pipeline(
  CLUSTERS,
  // Stage 1: review the cluster against real source
  (cluster) =>
    agent(cluster.prompt, {
      label: `review:${cluster.key}`,
      phase: 'Review',
      schema: FINDING_SCHEMA,
      agentType: 'Explore',
    }),
  // Stage 2: adversarially verify each DONE/PARTIAL/HONEST finding (the risky ones)
  (review, cluster) => {
    if (!review || !review.findings) return review
    const risky = review.findings.filter(
      (f) => f.actual_status === 'DONE' || f.actual_status === 'HONEST-EXCEPTION' || f.actual_status === 'PARTIAL',
    )
    if (risky.length === 0) return { cluster: cluster.key, verified: review.findings, disputes: [] }
    return parallel(
      risky.map((f) => () =>
        agent(
          `You are a skeptic verifying a claim about the gpui-query source at ${ROOT}. A reviewer claimed finding "${f.claim_id}" is ${f.actual_status} based on: ${f.evidence}. Independently read the cited source yourself. Try to REFUTE the reviewer — is the claim actually weaker than stated (e.g. method exists but is never called, signature is wrong, line drifted, test exists but doesn't actually test the stated behavior)? Default to disagrees=true (disputing) only if you find concrete evidence the verdict is wrong; otherwise agree. Be precise with file:line.`,
          { label: `verify:${f.claim_id}`, phase: 'Verify', schema: VERDICT_SCHEMA, agentType: 'Explore' },
        ).then((v) => v ? { ...v, claimed: f.actual_status } : null),
      ),
    ).then((verdicts) => ({
      cluster: cluster.key,
      verified: review.findings,
      disputes: verdicts.filter(Boolean),
    }))
  },
)

phase('Synthesize')

const clean = results.filter(Boolean)
const synthesis = await agent(
  `You are synthesizing a verification report. Here are the per-cluster review + adversarial-verification results for the gpui-query audit implementation-status doc (JSON):

${JSON.stringify(clean, null, 2)}

Produce a concise accuracy verdict. Count: how many claims HOLD vs DO NOT HOLD (where a "dispute" with agrees=false overrides the reviewer). List any claims that are refuted (the doc overstates them) by claim_id with a one-line reason. Also call out internal contradictions in the doc. Return the structured summary.`,
  { label: 'synthesize', phase: 'Synthesize', schema: {
    type: 'object',
    properties: {
      total_claims_checked: { type: 'number' },
      holds: { type: 'number' },
      does_not_hold: { type: 'number' },
      refuted_claims: { type: 'array', items: { type: 'object', properties: { claim_id: { type: 'string' }, reason: { type: 'string' } }, required: ['claim_id', 'reason'] } },
      doc_contradictions: { type: 'array', items: { type: 'string' } },
      bottom_line: { type: 'string', description: 'one-paragraph verdict on whether the doc accurately reflects the source' },
    },
    required: ['total_claims_checked', 'holds', 'does_not_hold', 'refuted_claims', 'doc_contradictions', 'bottom_line'],
  } },
)

return { synthesis, per_cluster: clean }
