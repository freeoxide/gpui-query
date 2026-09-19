//! Integration tests for the QueryClient layer.
//!
//! Tests use `#[gpui::test]` with `TestAppContext` and the `test_support`
//! helpers, exercising the full client API: resource creation, type
//! partitioning, invalidation, reset, GC, mutations, diagnostics, signals,
//! data access, and observers.
//!
//! All tests use `cx.update_global::<QueryClient, _>(|client, cx| ...)`:
//! methods like `resource()` need `&mut self` and `&mut App`, so the
//! immutable `cx.global()` cannot be used.
//!
//! GC reads live entity state via `entity.read(cx)`, so tests drive
//! resources to a known status / timestamp with direct entity updates
//! (e.g. `apply_success`) before calling `gc_with_time()`.

mod client_basics;
mod data_access;
mod invalidation_reset_gc;
mod mutations_lifecycle;
