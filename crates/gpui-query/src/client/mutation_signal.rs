/// Marker [`gpui::Global`] bumped whenever cached query data changes.
///
/// The value carries no state: bump sites call
/// `cx.default_global::<CacheMutation>()`, which pushes GPUI's
/// `NotifyGlobalObservers` effect exactly like `set_global`, and that
/// notification is what `observe_global::<CacheMutation>()` listeners
/// (the `persist_with` driver) react to. `default_global` is infallible and
/// seeds the marker on first bump, so bump sites never panic.
#[derive(Default)]
pub struct CacheMutation;

impl gpui::Global for CacheMutation {}
