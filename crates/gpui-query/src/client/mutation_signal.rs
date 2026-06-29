//! Precise whole-client dirty signal for the persistence layer.
//!
//! [`CacheMutation`] is a marker [`gpui::Global`] bumped (via
//! `cx.default_global::<CacheMutation>()`) at every point in the client/hook
//! layer that mutates cached query data. In GPUI, `default_global` pushes
//! `Effect::NotifyGlobalObservers` exactly like `set_global` and `global_mut`,
//! so [`persist_with`](super::QueryClient::persist_with) — which subscribes via
//! `cx.observe_global::<CacheMutation>()` — wakes on every bump and a save is
//! scheduled exactly when the cache actually changed. This is "Open Question 2
//! → Option B" from the design doc: a dedicated signal is both more precise
//! (it avoids the spurious fetch-start noise of observing the `QueryClient`
//! global) and more complete (it fires for `set_query_data` and the three
//! completion-site families, where `observe_global::<QueryClient>` would miss
//! bare `entity.update` completions that don't touch the `QueryClient` global).

/// Marker [`gpui::Global`] bumped whenever cached query data changes.
///
/// The value itself carries no state — the bump sites call
/// `cx.default_global::<CacheMutation>()`, which (like `set_global` and
/// `global_mut`) unconditionally pushes GPUI's `NotifyGlobalObservers` effect,
/// and that notification is what `observe_global::<CacheMutation>()`
/// listeners — including the `persist_with` driver — react to. `default_global`
/// is infallible and seeds the marker on first bump if no driver has set it
/// yet, so the bump sites never panic.
#[derive(Default)]
pub struct CacheMutation;

impl gpui::Global for CacheMutation {}
