/// Returns current time as milliseconds since UNIX epoch.
pub fn current_time_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Read a value from a GPUI entity in a way that compiles against **both**
/// older gpui (e.g. the Zed git revisions used by some apps) where
/// `Entity::read_with` returns `R` directly, **and** gpui 0.2.2 (crates.io)
/// where `Entity::read_with` returns `C::Result<R>` — which is `Result<R>`
/// for `AsyncApp`.
///
/// This is achieved by making the inner closure return `()`, so `read_with`
/// yields `()` on the former and `Result<()>` on the latter. Both are
/// discarded; the real value is captured through a mutable local, which is
/// identical on either version.
///
/// Returns `Some(R)` on success, or `None` if the entity could not be read
/// (e.g. dropped or accessed off-thread) under the `Result`-returning API —
/// matching the previous `.unwrap_or(default)` fallback behaviour.
pub(crate) fn read_entity<T: 'static, R, C: gpui::AppContext>(
    entity: &gpui::Entity<T>,
    cx: &C,
    f: impl FnOnce(&T, &gpui::App) -> R,
) -> Option<R> {
    let mut out: Option<R> = None;
    let _ = entity.read_with(cx, |value, app| {
        out = Some(f(value, app));
    });
    out
}
