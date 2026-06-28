//! Precise whole-client dirty signal for the persistence layer.
//!
//! [`CacheMutation`] is a marker [`gpui::Global`] bumped (via
//! `cx.update_global::<CacheMutation, _>(|_, _| ())`) at every point in the
//! client/hook layer that mutates cached query data. [`persist_with`](super::QueryClient::persist_with)
//! subscribes to it with `cx.observe_global::<CacheMutation>()`, so a save is
//! scheduled exactly when the cache actually changed — not on every GPUI
//! observer tick. This is "Open Question 2 → Option B" from the design doc: a
//! dedicated signal is both more precise (no false positives from unrelated
//! observers) and more complete (it fires for `set_query_data` and the three
//! completion sites, where `observe_global::<QueryClient>` would miss
//! mutations that don't go through `update_global::<QueryClient>`).

/// Marker [`gpui::Global`] bumped whenever cached query data changes.
///
/// The value itself carries no state — bumping it (via
/// `cx.default_global::<CacheMutation>()`, which creates the marker if absent
/// and always fires GPUI's `NotifyGlobalObservers` effect) is what
/// `observe_global::<CacheMutation>()` listeners react to. `Default` is
/// derived so the bump is infallible: the marker is created on first bump if
/// no `persist_with` driver has seeded it yet, so the bump sites never panic.
#[derive(Default)]
pub struct CacheMutation;

impl gpui::Global for CacheMutation {}
