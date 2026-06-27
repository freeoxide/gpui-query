import { CodeBlock } from "#/components/code-block";

/**
 * Body for the "Why gpui-query" post. Frontmatter lives in `web/src/lib/blog.ts`.
 */
export default function WhyGpuiQueryPost() {
  return (
    <>
      <p>
        When you build an editor like{" "}
        <a href="https://zed.dev" target="_blank" rel="noreferrer noopener">
          Zed
        </a>
        , your components constantly need data that arrives asynchronously: language-server
        responses, file listings, collaborator presence, telemetry. The GPUI render loop is
        synchronous, which means juggling in-flight requests, caches, retries, and stale data by
        hand gets old fast.
      </p>

      <p>
        <code>gpui-query</code> adapts{" "}
        <a href="https://tanstack.com/query" target="_blank" rel="noreferrer noopener">
          TanStack Query
        </a>
        &apos;s proven model to Rust and GPUI. It keeps the ideas you already know — declarative
        queries, automatic caching, background refetching, cooperative cancellation — and grounds
        them in Rust&apos;s type system and GPUI&apos;s reactive primitives.
      </p>

      <h2>Three layers, one crate</h2>
      <p>The crate is split into layers so you can adopt only what you need:</p>
      <ul>
        <li>
          <strong>
            <code>core</code>
          </strong>{" "}
          — a Serde-only state machine (<code>QueryResource</code>, <code>CachePolicy</code>,{" "}
          <code>RetryPolicy</code>, <code>QuerySignal</code>). Framework-agnostic and serializable.
        </li>
        <li>
          <strong>
            <code>client</code>
          </strong>{" "}
          — the GPUI <code>QueryClient</code> registry, with type-partitioned buckets for resource
          lifecycle, garbage collection, and persistence.
        </li>
        <li>
          <strong>
            <code>hook</code>
          </strong>{" "}
          — the ergonomic <code>use_query</code> / <code>use_mutation</code> hooks that components
          subscribe through.
        </li>
      </ul>

      <h2>A query in five lines</h2>
      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`use gpui_query::hook::use_query;
use gpui_query::{CachePolicy, QueryKey, RequestPolicy};

let (users, _subscription) = use_query(
    QueryKey::from(["users"]),
    CachePolicy::Ttl { ttl_ms: 60_000 },
    RequestPolicy::LatestWins,
    || async {
        let resp = reqwest::get("/api/users").await?;
        let users: Vec<User> = resp.json().await?;
        Ok(users)
    },
    cx,
);`}</code>
        </pre>
      </CodeBlock>

      <p>
        <code>use_query</code> returns a tuple <code>(data, status)</code> by Rust convention —
        ergonomic to destructure, with the status enum carrying the full lifecycle state. You never
        write <code>useEffect</code> glue or manage a <code>loading</code> boolean by hand.
      </p>

      <h2>What you get for free</h2>
      <ul>
        <li>
          <strong>Caching</strong> via <code>CachePolicy</code> (<code>NoCache</code>,{" "}
          <code>Ttl</code>, <code>StaleWhileRevalidate</code>).
        </li>
        <li>
          <strong>Retries</strong> with fixed delays or exponential backoff and a cap.
        </li>
        <li>
          <strong>Cooperative cancellation</strong> through <code>QuerySignal</code> (
          <code>Arc&lt;AtomicBool&gt;</code>) so a component unmounting does not leak work.
        </li>
        <li>
          <strong>Persistence</strong> via the <code>QueryPersister</code> trait — serialize and
          restore cache state across sessions.
        </li>
        <li>
          <strong>Infinite queries</strong> for paginated data, with bidirectional fetching.
        </li>
      </ul>

      <h2>Who is this for?</h2>
      <p>
        If you are building anything nontrivial on GPUI and find yourself re-implementing a cache, a
        refetch timer, or a cancellation token, <code>gpui-query</code> exists for you. The core
        layer is testable in isolation; the hook layer slots straight into your views. Give it a try
        and open an issue if anything feels off.
      </p>
    </>
  );
}
