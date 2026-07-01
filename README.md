# gpui-query

Async state management for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui), inspired by [TanStack Query](https://tanstack.com/query).

Fetch, cache, and synchronize async data in GPUI applications without manual lifecycle management. Built for the framework that powers the [Zed editor](https://zed.dev).

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## what it is

GPUI renders synchronously on the main thread. That makes async data fetching awkward: you need to track loading states, handle errors, cache responses, deduplicate concurrent requests, and retry on failure. gpui-query handles all of it.

You write a fetcher function. The library manages caching, retry, deduplication, stale-while-revalidate, garbage collection, and cooperative cancellation. It works with GPUI's `Entity` and `ViewContext` system, not against it.

The API mirrors what TanStack Query popularized in the JavaScript ecosystem: `use_query`, `use_mutation`, and `use_infinite_query` hooks that return `Entity` handles you read from in your view's `render` method.

## install

```toml
[dependencies]
gpui-query = "0.1.0"
```

This pulls in the `client` layer (which includes `core`). To use the declarative hooks:

```toml
[dependencies]
gpui-query = { version = "0.1.0", features = ["hook"] }
```

To use only the core state machine with no GPUI dependency:

```toml
[dependencies]
gpui-query = { version = "0.1.0", default-features = false, features = ["core"] }
```

## quick start

Set up the `QueryClient` as a GPUI global during app initialization:

```rust
use gpui::App;
use gpui_query::QueryClient;

App::new().run(|cx| {
    cx.set_global(QueryClient::new());
    // ... your views
});
```

Fetch data with `use_query`:

```rust
use gpui_query::{use_query, QueryOptions};

fn setup_query(cx: &mut ViewContext<MyView>) -> (Entity<QueryResource<Vec<User>, MyError>>, Subscription) {
    use_query(
        "users",
        |signal| async move {
            // Your async fetch. Check signal.is_cancelled() for cooperative cancellation.
            let users = fetch_users().await?;
            Ok::<Vec<User>, MyError>(users)
        },
        cx,
    )
}
```

Read the state in your render method:

```rust
fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    let entity = self.query_entity.clone();
    entity.read_with(cx, |resource| {
        match resource.status() {
            QueryStatus::LoadingEmpty => "Loading...",
            QueryStatus::Success => "Got data",
            QueryStatus::Failure => "Error",
            _ => "Idle",
        }
    })
}
```

## architecture

The library is three layers, each gated by a Cargo feature flag.

**Core** (`feature = "core"`) is a serde-only state machine with zero framework coupling. This is where `QueryResource<T,E>`, `MutationResource<V,T,E>`, `CachePolicy`, `RetryPolicy`, `QueryKey`, and all the request lifecycle types live. You can use this layer in any Rust project, not just GPUI.

**Client** (`feature = "client"`, the default) builds on core and adds `QueryClient`, a GPUI `Global` that provides type-partitioned storage via `QueryBucket<T,E>`. This layer handles garbage collection, cache invalidation, observers, persistence, and devtools diagnostics.

**Hook** (`feature = "hook"`) provides the declarative hooks (`use_query`, `use_mutation`, `use_infinite_query`) that wire the client layer into GPUI views. All hooks return `(Entity, Subscription)` tuples.

```toml
[features]
default = ["client"]
core = []
client = ["core", "dep:gpui"]
hook = ["client"]
```

## queries

The primary hook. Pass a key (string or `QueryOptions`), a fetcher function, and the view context. The fetcher receives a `QuerySignal` for cooperative cancellation.

```rust
use gpui_query::{use_query, QueryOptions, CachePolicy, RetryPolicy};

// Simple key
let (entity, sub) = use_query("users", fetcher, cx);

// With options
let (entity, sub) = use_query(
    QueryOptions::new("users")
        .cache_policy(CachePolicy::Ttl { ttl_ms: 300_000 })
        .retry_policy(RetryPolicy::new(5).with_exponential_backoff()),
    fetcher,
    cx,
);
```

`QueryResource<T,E>` tracks the full lifecycle: idle, loading (with or without previous data), success, failure, or cancelled. You get `data()`, `error()`, `status()`, `is_loading()`, `has_data()`, `display_data()` (returns data or a placeholder), `cache_age_ms()`, and `retry_count()`.

For manual control with no auto-fetch, use `use_query_manual` and trigger fetches with `fetch_query` when you're ready.

## mutations

```rust
use gpui_query::{use_mutation, mutate, MutationCallbacks};

let (entity, sub) = use_mutation((), cx);

// Trigger the mutation
mutate(&entity, NewUser { name: "Alice" }, |vars| async move {
    Ok::<User, MyError>(create_user(vars).await)
}, cx);

// With success/error/settled callbacks
mutate_with_callbacks(
    &entity,
    variables,
    mutator,
    MutationCallbacks::new()
        .on_success(|data| { /* invalidate user queries */ })
        .on_error(|err| eprintln!("mutation failed: {err:?}")),
    cx,
);
```

Mutations track their own state in `MutationResource<V,T,E>` with a begin/complete/retry/reset lifecycle. They don't touch the query cache unless you explicitly invalidate queries in an `on_success` callback.

## infinite queries

For paginated data. The fetcher receives the last page (or `None` for the first request) and returns `(page_data, has_more)`.

```rust
use gpui_query::{use_infinite_query, InfiniteQueryOptions, QueryKey};

let (entity, sub) = use_infinite_query(
    InfiniteQueryOptions::new(QueryKey::from(["feed"])).max_pages(Some(10)),
    |last_page| async move {
        let cursor = last_page.map(|p| p.cursor());
        let page = fetch_page(cursor).await?;
        Ok::<_, MyError>((page.items, page.has_more))
    },
    cx,
);
```

Pages are stored in a `VecDeque`. Default cap is 50 pages, configurable via `max_pages()`. Supports bidirectional fetching with `fetch_next_page_infinite` and `fetch_previous_page_infinite`.

## caching

Three policies:

- `NoCache` fetches every time.
- `Ttl { ttl_ms }` caches data for the specified duration. This is the default (60 seconds).
- `StaleWhileRevalidate { ttl_ms, stale_ms }` returns cached data immediately after TTL expires and kicks off a background refetch.

Bulk operations on the `QueryClient`:

```rust
let client = cx.global::<QueryClient>();

// Invalidate all queries with a matching key prefix
client.invalidate_queries(&QueryKeyFilter::Prefix(&QueryKey::from(["users"])), cx);

// Remove everything
client.remove_queries(&QueryKeyFilter::All, cx);

// Optimistic update
client.set_query_data::<Vec<User>, MyError>(&key, Some(vec![new_user]), cx);
client.rollback_query_data::<Vec<User>, MyError>(&key, cx);
```

Invalidation matching supports `Exact`, `Prefix`, and `All` filters via `QueryKeyFilter`.

## retry

```rust
use gpui_query::RetryPolicy;

// Defaults: 3 retries, 1s base delay, exponential backoff, 30s cap
let policy = RetryPolicy::default();

// Custom
let policy = RetryPolicy::new(5)          // max retries
    .with_delay(2_000)                     // 2s base
    .with_exponential_backoff()            // 2s, 4s, 8s, 16s, 32s
    .with_max_delay(60_000);               // cap at 60s
```

Retry delay is `base * 2^attempt`, capped at `max_delay`. The fetcher's `QuerySignal` is checked between attempts so cancelled queries stop retrying immediately.

## persistence

Implement `QueryPersister` to serialize and restore query state across app restarts:

```rust
use gpui_query::{QueryPersister, DehydratedEntry};

struct FilePersister;

impl QueryPersister for FilePersister {
    fn load(&self) -> Vec<DehydratedEntry> {
        // Read from disk, database, etc.
    }

    fn save(&self, entries: Vec<DehydratedEntry>) {
        // Write to disk, database, etc.
    }
}
```

Call `client.dehydrate()` to extract state and `client.hydrate(entries)` to restore it.

## other things worth knowing

`QueryObserver` and `MutationObserver` wrap entities and only call `cx.notify()` when the status actually changes, cutting down on unnecessary re-renders.

`QueryError::sanitized()` redacts connection strings, bearer tokens, file paths, emails, and hex keys from error messages. Useful for logging without leaking secrets.

`use_query_select` projects a `QueryResource<T,E>` through a `SelectTransform<T,U>`, giving you a `MappedQueryResource` that derives values from cached data without extra fetches.

`ClientDiagnostic`, `QueryDiagnostic`, and `MutationDiagnostic` give you runtime introspection of the query client's internal state for debugging.

`PreparedFetch` holds an entity, request ID, and signal for imperative one-shot fetches that you complete manually.

Garbage collection runs on idle resources older than `gc_time_ms` (default: 5 minutes). Configurable per-query via `QueryOptions::gc_time_ms()`.

## links

- Documentation: <https://gpui-query.freeoxide.com/docs/>
- Website: <https://gpui-query.freeoxide.com>
- Source: <https://github.com/freeoxide/gpui-query>
- GPUI framework: <https://github.com/zed-industries/zed/tree/main/crates/gpui>
- TanStack Query: <https://tanstack.com/query>

## license

MIT. See [LICENSE](LICENSE).
