# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/hmziqrs/gpui-query/releases/tag/v0.1.0
