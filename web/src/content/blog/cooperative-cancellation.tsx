import { CodeBlock } from "#/components/code-block";

/**
 * Body for the "Cooperative Cancellation" post. Frontmatter lives in
 * `web/src/lib/blog.ts`.
 */
export default function CooperativeCancellationPost() {
  return (
    <>
      <p>
        A subtle bug in async UI code: a component kicks off a network request, the user navigates
        away, and the request&apos;s result eventually lands — overwriting fresh state or panicking
        on a stale handle. Most web frameworks paper over this with abort signals and effect
        cleanup. In a Rust + GPUI app you want something cheaper and more explicit.
      </p>

      <p>
        <code>gpui-query</code> solves this with cooperative cancellation via{" "}
        <code>QuerySignal</code>.
      </p>

      <h2>What QuerySignal actually is</h2>
      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`#[derive(Debug, Clone)]
pub struct QuerySignal {
    cancelled: Arc<AtomicBool>,
}`}</code>
        </pre>
      </CodeBlock>

      <p>
        That is the whole type. A shared atomic flag wrapped in an <code>Arc</code>. Clones share
        the same underlying flag, so cancelling any clone cancels all of them:
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`use gpui_query::core::QuerySignal;

let signal = QuerySignal::new();
let clone = signal.clone();

signal.cancel();
assert!(signal.is_cancelled());
assert!(clone.is_cancelled());`}</code>
        </pre>
      </CodeBlock>

      <p>
        <code>Ordering::SeqCst</code> on both the store and load keeps the cancellation observable
        across threads without locking. It is a runtime-only type — it deliberately cannot be
        serialized, because the shared state has no meaningful persisted form.
      </p>

      <h2>Why cooperative, not preemptive?</h2>
      <p>
        Rust does not let you kill a future from the outside. Cancellation has to be{" "}
        <em>cooperative</em>: the running task has to opt in by checking a flag at safe points.{" "}
        <code>QuerySignal</code> gives fetchers a tiny, allocation-free way to do exactly that.
      </p>

      <p>The contract is simple:</p>
      <ul>
        <li>
          The client hands your fetcher a <code>QuerySignal</code> clone when a request starts.
        </li>
        <li>
          When the request is superseded (newer request, component unmount, manual cancel), the
          client calls <code>signal.cancel()</code>.
        </li>
        <li>
          Your fetcher checks <code>signal.is_cancelled()</code> periodically and aborts early.
        </li>
      </ul>

      <h2>Where to check the signal</h2>
      <p>
        The most important place is <strong>between retry attempts</strong>.{" "}
        <code>RetryPolicy</code> will keep retrying a failing request up to <code>max_retries</code>{" "}
        times; without cancellation that means a doomed query can keep working for seconds after
        nobody cares about the result anymore.
      </p>

      <CodeBlock>
        <pre className="overflow-x-auto p-4 text-sm leading-relaxed">
          <code>{`use gpui_query::core::{QuerySignal, RetryPolicy};

async fn fetch_users(signal: QuerySignal) -> Result<Vec<User>, MyError> {
    let policy = RetryPolicy::default(); // 3 retries, exponential backoff
    for attempt in 0..=policy.max_retries {
        if signal.is_cancelled() {
            return Err(MyError::Cancelled);
        }
        match do_request().await {
            Ok(users) => return Ok(users),
            Err(e) if attempt == policy.max_retries => return Err(e),
            Err(_) => sleep(backoff(attempt)).await,
        }
    }
    unreachable!()
}`}</code>
        </pre>
      </CodeBlock>

      <p>
        Long-running scans or paginated fetches should also poll <code>is_cancelled()</code> at
        their natural chunk boundaries.
      </p>

      <h2>How LatestWins uses it</h2>
      <p>
        <code>RequestPolicy::LatestWins</code> is built on this primitive. When a new request
        arrives for the same key while an older one is still in flight, the older request&apos;s
        signal is cancelled and the client waits only on the newest result. That prevents stale data
        from clobbering fresh data — a classic race in any query cache — without forcing the
        framework to abort futures it does not own.
      </p>

      <h2>Takeaways</h2>
      <ul>
        <li>
          <code>QuerySignal</code> is an <code>Arc&lt;AtomicBool&gt;</code>: cheap to clone,
          lock-free to read.
        </li>
        <li>
          Cancellation is cooperative; check <code>is_cancelled()</code> at retry and chunk
          boundaries inside your fetcher.
        </li>
        <li>
          <code>LatestWins</code> and component unmount both flow through the same signal, so there
          is one consistent teardown story across the crate.
        </li>
      </ul>

      <p>
        If your fetcher never checks the signal, nothing breaks — but it also will not abort early.
        Treat the signal as an optimization contract between you and the client, and your app will
        leak less work.
      </p>
    </>
  );
}
