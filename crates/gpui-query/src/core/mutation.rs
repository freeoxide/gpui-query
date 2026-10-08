use serde::{Deserialize, Serialize};

use super::{QueryError, QueryKey, QuerySignal, RetryPolicy};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationStatus {
    #[default]
    Idle,
    Loading,
    Success,
    Failure,
}

impl MutationStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Loading => "Loading",
            Self::Success => "Success",
            Self::Failure => "Failure",
        }
    }

    pub fn is_loading(self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn is_failure(self) -> bool {
        matches!(self, Self::Failure)
    }
}

/// `V` is the variables (input) type, `T` the success output, `E` the error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationResource<V, T, E = QueryError> {
    key: Option<QueryKey>,
    status: MutationStatus,
    data: Option<T>,
    error: Option<E>,
    variables: Option<V>,
    retry_count: u32,
    cancelled_count: u64,
    retry_policy: RetryPolicy,
    /// Terminal-completion time (not insertion time) drives `MutationBucket`
    /// GC recency; runtime state, not persisted.
    #[serde(skip)]
    last_updated_at_ms: Option<u64>,
    #[serde(skip)]
    signal: Option<QuerySignal>,
    /// Stored so a replacement mutation or entity drop aborts the prior
    /// in-flight task instead of leaving it detached.
    #[cfg(feature = "client")]
    #[serde(skip)]
    pub(crate) current_task: crate::core::current_task::CurrentTask,
}

/// Clamps to 0 when the system clock is before the epoch.
fn completion_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

impl<V, T, E> MutationResource<V, T, E> {
    pub fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            key: None,
            status: MutationStatus::Idle,
            data: None,
            error: None,
            variables: None,
            retry_count: 0,
            cancelled_count: 0,
            retry_policy,
            last_updated_at_ms: None,
            signal: None,
            #[cfg(feature = "client")]
            current_task: crate::core::current_task::CurrentTask::default(),
        }
    }

    pub fn status(&self) -> MutationStatus {
        self.status
    }

    // Read only by the client layer; core-only builds (e.g. wasm32) have no caller.
    #[cfg_attr(not(feature = "client"), allow(dead_code))]
    pub(crate) fn last_updated_at_ms(&self) -> Option<u64> {
        self.last_updated_at_ms
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn error(&self) -> Option<&E> {
        self.error.as_ref()
    }

    pub fn variables(&self) -> Option<&V> {
        self.variables.as_ref()
    }

    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    pub fn cancelled_count(&self) -> u64 {
        self.cancelled_count
    }

    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    pub fn set_retry_policy(&mut self, policy: RetryPolicy) {
        self.retry_policy = policy;
    }

    pub fn is_loading(&self) -> bool {
        self.status.is_loading()
    }

    pub fn is_idle(&self) -> bool {
        self.status.is_idle()
    }

    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }

    pub fn key(&self) -> Option<&QueryKey> {
        self.key.as_ref()
    }

    /// The hook layer never sets a key; kept for callers tagging mutations
    /// for diagnostics or invalidation.
    pub fn with_key(mut self, key: QueryKey) -> Self {
        self.key = Some(key);
        self
    }

    /// Cancels any in-flight signal and resets `retry_count`, so each
    /// invocation starts fresh.
    pub fn begin(&mut self, variables: V) {
        if let Some(old_signal) = self.signal.as_ref() {
            old_signal.cancel();
        }
        self.status = MutationStatus::Loading;
        self.variables = Some(variables);
        self.error = None;
        self.retry_count = 0;
        self.last_updated_at_ms = None;
        self.signal = Some(QuerySignal::new());
    }

    pub fn complete_success(&mut self, data: T) {
        self.status = MutationStatus::Success;
        self.data = Some(data);
        self.error = None;
        self.last_updated_at_ms = Some(completion_now_ms());
        self.signal = None;
    }

    /// Clears `data` so consumers never see stale success data beside a
    /// `Failure` status.
    pub fn complete_failure(&mut self, error: E) {
        self.status = MutationStatus::Failure;
        self.data = None;
        self.error = Some(error);
        self.retry_count = self.retry_count.saturating_add(1);
        self.last_updated_at_ms = Some(completion_now_ms());
        self.signal = None;
    }

    pub fn should_retry(&self) -> bool {
        self.retry_policy.should_retry(self.retry_count)
    }

    /// Only from `Failure` with retries remaining; creates a fresh signal.
    pub fn retry(&mut self) -> bool {
        if self.status != MutationStatus::Failure || !self.should_retry() {
            return false;
        }
        self.status = MutationStatus::Loading;
        self.error = None;
        self.signal = Some(QuerySignal::new());
        true
    }

    pub fn reset(&mut self) {
        if let Some(signal) = self.signal.as_ref() {
            signal.cancel();
        }
        self.status = MutationStatus::Idle;
        self.data = None;
        self.error = None;
        self.variables = None;
        self.retry_count = 0;
        self.cancelled_count = 0;
        self.last_updated_at_ms = None;
        self.signal = None;
    }

    pub fn signal(&self) -> Option<&QuerySignal> {
        self.signal.as_ref()
    }

    /// Tracks attempts without transitioning through a terminal `Failure`.
    pub fn increment_retry(&mut self) {
        self.retry_count = self.retry_count.saturating_add(1);
    }

    /// Stays in `Loading` so observers never see a transient `Failure` flash
    /// between retry attempts.
    pub fn prepare_retry(&mut self) {
        if self.status != MutationStatus::Loading {
            return;
        }
        if let Some(old_signal) = self.signal.as_ref() {
            old_signal.cancel();
        }
        self.error = None;
        self.signal = Some(QuerySignal::new());
    }

    pub fn reset_retry_count(&mut self) {
        self.retry_count = 0;
    }

    /// No-op unless `Loading`; when effective, clears data, sets `Failure`,
    /// and increments `cancelled_count`.
    pub fn cancel(&mut self, error: E) {
        if self.status != MutationStatus::Loading {
            return;
        }
        self.cancelled_count = self.cancelled_count.saturating_add(1);
        self.status = MutationStatus::Failure;
        self.data = None;
        self.error = Some(error);
        self.last_updated_at_ms = Some(completion_now_ms());
        if let Some(signal) = self.signal.as_ref() {
            signal.cancel();
        }
        self.signal = None;
    }
}

#[cfg(feature = "client")]
impl<V, T, E> MutationResource<V, T, E> {
    pub(crate) fn set_current_task(&mut self, task: gpui::Task<()>) {
        self.current_task.set(task);
    }
}
