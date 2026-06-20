import { CodeBlock } from "#/components/code-block";

/**
 * Body for the "Cache Policies, Explained" post. Frontmatter lives in
 * `web/src/lib/blog.ts`.
 */
export default function CachePoliciesExplainedPost() {
  return (
    <>
      <p>
        Caching is the part of a query library where good intentions turn into stale bugs.{" "}
        <code>gpui-query</code> keeps the surface area deliberately small: three{" "}
        <code>CachePolicy</code> variants, each with a clear contract.
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`pub enum CachePolicy {
    NoCache,
    Ttl { ttl_ms: u64 },
    StaleWhileRevalidate { ttl_ms: u64 },
}`}</code>
        </pre>
      </CodeBlock>

      <p>Let&apos;s walk through each one and when to reach for it.</p>

      <h2>NoCache</h2>
      <p>
        <code>CachePolicy::NoCache</code> never caches. Every observer mount triggers a fresh fetch.
        Use it when the data is cheap to recompute and must always be current — live presence,
        ephemeral session state, or anything where showing even one millisecond of stale data is
        wrong.
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`use gpui_query::CachePolicy;

let policy = CachePolicy::NoCache;
assert_eq!(policy.ttl_ms(), None);
assert!(!policy.can_short_circuit());`}</code>
        </pre>
      </CodeBlock>

      <p>
        It is also the safe default when you are unsure: you trade a little performance for never
        having to reason about staleness.
      </p>

      <h2>Ttl</h2>
      <p>
        <code>CachePolicy::Ttl {"{ ttl_ms }"}</code> caches a successful result for{" "}
        <code>ttl_ms</code> milliseconds. While the entry is fresh, new observers{" "}
        <strong>short-circuit</strong> — they get the cached value immediately without firing a
        request. <code>can_short_circuit()</code> returns <code>true</code> only for this variant.
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`let policy = CachePolicy::Ttl { ttl_ms: 60_000 }; // 60s
assert_eq!(policy.ttl_ms(), Some(60_000));
assert!(policy.can_short_circuit());`}</code>
        </pre>
      </CodeBlock>

      <p>
        <code>Ttl</code> is the workhorse for most API data: a list of projects, a user profile, a
        config document. Pick the TTL from how stale your UI can tolerate the data being. For
        collaborative data that changes a lot, keep it short; for slow-moving reference data, let it
        ride for minutes.
      </p>

      <p>
        Once the TTL elapses the entry is considered stale; the next observer triggers a refetch.
      </p>

      <h2>StaleWhileRevalidate</h2>
      <p>
        <code>CachePolicy::StaleWhileRevalidate {"{ ttl_ms }"}</code> is the variant that makes UIs
        feel instant. It shares the same <code>ttl_ms</code> semantics for <em>freshness</em>, but
        after the TTL elapses the cache does not just discard the entry: it serves the stale value{" "}
        <strong>immediately</strong> to the observer and kicks off a background refetch.
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`let policy = CachePolicy::StaleWhileRevalidate { ttl_ms: 30_000 };
assert_eq!(policy.ttl_ms(), Some(30_000));
assert!(!policy.can_short_circuit());`}</code>
        </pre>
      </CodeBlock>

      <p>
        Note that <code>can_short_circuit()</code> is <code>false</code> here — unlike{" "}
        <code>Ttl</code>, a stale-while-revalidate entry is never silently trusted as fresh. The
        point is to render optimistically with old data while a new request runs, then swap in the
        result. This is the right choice for dashboards and feeds where the user would rather see{" "}
        <em>something</em> now and an update a moment later.
      </p>

      <h2>How policies interact with the rest of the crate</h2>
      <p>A cache policy never acts alone. A few interactions worth knowing:</p>
      <ul>
        <li>
          <strong>Retries.</strong> A failed fetch follows <code>RetryPolicy</code> (exponential
          backoff, capped) regardless of cache policy. <code>NoCache</code> +{" "}
          <code>RetryPolicy::no_retries()</code> is the most pessimistic combination;{" "}
          <code>StaleWhileRevalidate</code> + default retries is the most forgiving.
        </li>
        <li>
          <strong>Request policies.</strong> <code>RequestPolicy::LatestWins</code> cancels
          in-flight requests when a newer one arrives for the same key, so a revalidation triggered
          by a stale-while-revalidate hit can be cleanly superseded.
        </li>
        <li>
          <strong>Observers.</strong> Multiple observers on the same key share one underlying{" "}
          <code>QueryResource</code>, so a single background revalidation fans its result out to
          everyone subscribed.
        </li>
        <li>
          <strong>Persistence.</strong> Because <code>CachePolicy</code> is <code>Serialize</code>/
          <code>Deserialize</code>, cached state round-trips through a <code>QueryPersister</code>{" "}
          implementation with its TTL intact.
        </li>
      </ul>

      <h2>Choosing</h2>
      <div className="overflow-x-auto">
        <table>
          <thead>
            <tr>
              <th>Need</th>
              <th>Pick</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Must always be live</td>
              <td>
                <code>NoCache</code>
              </td>
            </tr>
            <tr>
              <td>Reference-ish data, fine to refetch on stale</td>
              <td>
                <code>Ttl {"{ ttl_ms }"}</code>
              </td>
            </tr>
            <tr>
              <td>Instant render, refresh in the background</td>
              <td>
                <code>StaleWhileRevalidate {"{ ttl_ms }"}</code>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <p>
        Start with <code>Ttl</code> for everything that is not obviously transient, switch to{" "}
        <code>StaleWhileRevalidate</code> for lists and feeds you want to feel snappy, and reserve{" "}
        <code>NoCache</code> for data where staleness is a correctness bug. Three variants, clear
        contracts, and the rest of the crate plays nicely with whichever one you pick.
      </p>
    </>
  );
}
