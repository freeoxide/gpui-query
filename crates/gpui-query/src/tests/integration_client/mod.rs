//! Integration tests for the QueryClient layer (v2).
//!
//! Tests use `#[gpui::test]` with `TestAppContext` and the `test_support` helpers.
//! They exercise the full client API: resource creation, type partitioning,
//! invalidation, reset, GC, mutations, diagnostics, signals, data access,
//! and observers.
//!
//! # Context pattern
//!
//! All tests use `cx.update_global::<QueryClient, _>(|client, cx| ...)` to
//! get `(&mut QueryClient, &mut App)`. Methods like `resource()` require
//! `&mut self` and `&mut App`, so `cx.global()` (immutable) cannot be used.
//!
//! # GC test design
//!
//! The bucket's GC reads live entity state directly via `entity.read(cx)`
//! (CL2/#106 removed the cached `StatusSnapshot`). Direct entity
//! manipulation (`apply_success`, etc.) and `PreparedFetch` completions
//! are therefore visible to GC immediately, with no separate snapshot
//! refresh step.
//!
//! For deterministic GC tests, we drive resources to a known status and
//! `last_updated_ms` via direct entity updates (e.g. `apply_success`),
//! then call `gc_with_time()` and assert the expected eviction /
//! preservation behavior without the hook layer.

mod client_basics;
mod data_access;
mod invalidation_reset_gc;
mod mutations_lifecycle;
