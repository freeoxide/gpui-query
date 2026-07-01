import { CodeBlock } from "#/components/code-block";

/**
 * Body for the "Introducing gpui-query" post. Frontmatter lives in
 * `web/src/lib/blog.ts`.
 */
export default function IntroducingGpuiQueryPost() {
  return (
    <>
      <p>
        <code>gpui-query</code> brings the battle-tested patterns of{" "}
        <a href="https://tanstack.com/query" target="_blank" rel="noreferrer noopener">
          TanStack Query
        </a>{" "}
        to the{" "}
        <a href="https://gpui.rs" target="_blank" rel="noreferrer noopener">
          GPUI
        </a>{" "}
        framework. If you&apos;ve ever wrestled with loading states, caching, or race conditions in
        your GPUI app — this is for you.
      </p>

      <h2>Why gpui-query?</h2>
      <p>
        Building async-driven UIs in GPUI meant writing repetitive boilerplate: manual fetch logic,
        cache management, stale-while-revalidate heuristics, and cooperative cancellation.{" "}
        <code>gpui-query</code> handles all of this out of the box.
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`let (data, status) = use_query(cx, || QueryArgs::new(
    "users".into(),
    || async { fetch_users().await },
));`}</code>
        </pre>
      </CodeBlock>

      <p>
        That&apos;s it. Automatic caching, background refetching, retry with exponential backoff —
        all configured by default.
      </p>

      <h2>What&apos;s included</h2>
      <ul>
        <li>
          <strong>Queries</strong> (<code>use_query</code>) — declarative async data fetching with
          caching
        </li>
        <li>
          <strong>Mutations</strong> (<code>use_mutation</code>) — server-side modifications with
          success/error callbacks
        </li>
        <li>
          <strong>Infinite Queries</strong> (<code>use_infinite_query</code>) — paginated data with
          bidirectional fetching
        </li>
        <li>
          <strong>Query Observers</strong> — fine-grained subscriptions for derived data
        </li>
        <li>
          <strong>Persistence API</strong> — custom serialization backends for offline support
        </li>
        <li>
          <strong>DevTools</strong> — diagnostic types for inspecting cache state
        </li>
      </ul>

      <h2>What&apos;s next</h2>
      <p>
        We&apos;re working on DevTools UI integration, server-state synchronization patterns, and a
        plugin system for custom query backends.{" "}
        <a href="https://github.com/freeoxide/gpui-query" target="_blank" rel="noreferrer noopener">
          Star the repo
        </a>{" "}
        to follow along.
      </p>
    </>
  );
}
