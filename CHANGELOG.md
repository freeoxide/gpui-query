# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2026-06-17

### Added

- Author metadata (authors) and an Author section in both crate READMEs, linking the maintainer's website, GitHub, and X.
- readme field on gpui-query-legacy so its README renders on crates.io.

## [0.1.3] - 2026-06-14

### Changed

- `gpui-query-legacy` is now fully decoupled from the main crate, with standalone docs, tests, and improved hook error handling, and it publishes independently.

### Added

- A crate-level README for `gpui-query` on crates.io.

### Fixed

- `read_with` calls in the hook module are now source-compatible across gpui versions.

## [0.1.2] - 2025-06-13

### Changed

- The CI pipeline now publishes both crates in a single workflow run. The changelog-release workflow handles tag, GitHub Release, publish, and website deploy without needing a separate trigger.
- `gpui-query-legacy` is now a fully independent crate on crates.io. The `legacy` feature flag and re-export were removed from `gpui-query`. If you need the v1 API, add `gpui-query-legacy` to your Cargo.toml directly.
- Legacy crate publish step tolerates "already uploaded" errors so the main crate can still publish when re-running a workflow.

## [0.1.1] - 2025-06-12

### Changed

- The v2 rewrite at `crates/gpui-query-v2` is now the main crate at `crates/gpui-query`. The old v1 code lives at `crates/gpui-query-legacy`.
- Fixed `read_with` calls in the hook module that returned `Result` instead of the raw value when called from `AsyncApp` context. The fix covers 9 call sites in `fetch_retry.rs`, `internals.rs`, and `fetch_runners.rs`.

### Added

- The legacy crate has `#![deprecated]` and a README pointing to v2.
- All 12 documentation pages have real content now. No more "coming soon" stubs.

### Removed

- All `gpui_query_v2` references in source and docs replaced with `gpui_query`.

## [0.1.0] - 2025-06-10

### Added

#### Core Layer
- `QueryResource<T, E>` reactive async state container with request lifecycle management
- `QueryStatus` enum (Idle, Loading, Success, Failure) for state tracking
- `CachePolicy` with TTL, Stale-While-Revalidate, LatestWins, and IgnoreWhileLoading variants
- `RequestPolicy` for controlling cache-first vs network-first behavior
- `RetryPolicy` with configurable max attempts, delay, and exponential backoff
- `QueryKey` type-safe cache key with string-based identification
- `QueryKeyFilter` glob-based key matching for cache invalidation patterns
- `QuerySignal` cooperative cancellation via `Arc<AtomicBool>` for clean async lifecycle
- `QueryError` / `QueryErrorKind` structured error types
- `MutationResource` and `MutationStatus` for mutation state tracking
- `NetworkMode` enum (Online, Offline, Always) for connectivity-aware fetching
- `RefetchTrigger` for imperative and automatic revalidation
- `RequestId` / `RequestGuard` / `RequestSequencer` for deduplication and race handling
- `MappedQueryResource` / `SelectTransform` for derived/transformed query state
- `InfiniteQueryResource` with bidirectional pagination and page management

#### Client Layer
- `QueryClient` implementing `gpui::Global` application-wide query registry
- Type-partitioned `QueryBucket<T, E>` for ergonomic typed access
- `MutationBucket<V, T, E>` for mutation state management
- `QueryObserver` and `MutationObserver` for reactive subscriptions
- Built-in garbage collection for stale query entries

#### Hook Layer
- `use_query()` declarative data fetching hook for GPUI components
- `use_mutation()` mutation hook with success/error callbacks
- `use_infinite_query()` pagination hook with bidirectional fetching
- `QueryOptions` and `MutationOptions` for per-hook configuration

#### gpui-query-v2 (Experimental Rewrite)
- Options-first API: `use_query(QueryOptions::new("key"), fetcher, cx)`
- Signal-always fetcher signature: `Fn(QuerySignal) -> Fut`
- `QueryError` with `Display` + `Error` impls and `.sanitized()` for security redaction
- `AHashMap` for faster key lookups
- `QueryPersister` trait with `dehydrate()`/`hydrate()` for state persistence
- `PreparedFetch` for imperative one-shot fetches
- Bounded `max_pages` (default 50) for infinite queries
- Mutation garbage collection
- Property-based testing with proptest
- `use_query_select()` for derived query state
- Extracted `fetch_retry` module with configurable retry logic
- Richer devtools: `ClientDiagnostic`, `DehydratedState`, `DehydratedEntry`

### Changed
- Initial public release

[0.1.4]: https://github.com/freeoxide/gpui-query/releases/tag/v0.1.4
[0.1.3]: https://github.com/freeoxide/gpui-query/releases/tag/v0.1.3
[0.1.2]: https://github.com/freeoxide/gpui-query/releases/tag/v0.1.2
[0.1.1]: https://github.com/freeoxide/gpui-query/releases/tag/v0.1.1
[0.1.0]: https://github.com/freeoxide/gpui-query/releases/tag/v0.1.0
