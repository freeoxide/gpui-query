/// Returns the current time as milliseconds since the UNIX epoch.
///
/// Callers can cache the value and pass it to
/// [`gc_with_time`](crate::client::QueryClient::gc_with_time) to avoid
/// repeated syscalls.
///
/// A clock reading before the Unix epoch clamps to `0` rather than
/// propagating an error; GC treats `0` as "ancient", so the only effect of
/// such a clock anomaly is that entries become immediately GC-eligible.
pub fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
