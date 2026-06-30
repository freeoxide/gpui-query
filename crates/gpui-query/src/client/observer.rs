//! Query observer for reactive state tracking.
//!
//! **v2 improvements**:
//! - `observe()` returns `Option<Subscription>` instead of panicking on dropped entity
//! - Status deduplication to avoid unnecessary `cx.notify()` calls
//!
//! **Audit L8**: the three formerly-structurally-identical observer types
//! (`QueryObserver`, `InfiniteQueryObserver`, `MutationObserver`) are now a
//! single generic [`Observer<R>`] plus type aliases. They differed only in
//! their entity type and status type (`QueryStatus` vs `MutationStatus`); the
//! observe logic (dedup `cx.notify()` via a `Cell<Option<S>>`, fall back to
//! unconditional notify) is shared by the one generic impl.

use std::cell::Cell;

use gpui::{Context, Entity, Subscription};

use crate::core::{
    InfiniteQueryResource, MutationResource, MutationStatus, QueryResource, QueryStatus,
};

/// Bridges a resource type to its status for the generic [`Observer`].
///
/// Each resource exposes its status via an *inherent* `status()` method, which
/// cannot be called generically without a trait; this trait (pub(crate), not
/// part of the public API) surfaces it with an associated `Status` type so
/// [`Observer<R>`] can dedup notifications for any resource kind.
pub trait ObservableResource {
    type Status: PartialEq + Copy + 'static;

    fn observable_status(&self) -> Self::Status;
}

impl<T: 'static, E: 'static> ObservableResource for QueryResource<T, E> {
    type Status = QueryStatus;

    fn observable_status(&self) -> QueryStatus {
        self.status()
    }
}

impl<T: 'static, E: 'static> ObservableResource for InfiniteQueryResource<T, E> {
    type Status = QueryStatus;

    fn observable_status(&self) -> QueryStatus {
        self.status()
    }
}

impl<V: 'static, T: 'static, E: 'static> ObservableResource for MutationResource<V, T, E> {
    type Status = MutationStatus;

    fn observable_status(&self) -> MutationStatus {
        self.status()
    }
}

/// Configuration for a query observer.
#[derive(Clone, Debug)]
pub struct ObserverConfig {
    /// Only notify when status changes (dedup re-renders).
    pub notify_on_status_change_only: bool,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            notify_on_status_change_only: true,
        }
    }
}

/// Observes a resource and triggers re-renders only on status changes.
///
/// In v2, the observer only calls `cx.notify()` when the status actually
/// changes, preventing excessive re-renders from intermediate state updates
/// like retry count increments.
///
/// This is a single generic implementation shared by every resource kind
/// (audit L8). Use the [`QueryObserver`] / [`InfiniteQueryObserver`] /
/// [`MutationObserver`] type aliases for the concrete kinds.
///
/// This is also the fix for audit findings #1/#11: the raw `cx.observe` in
/// `use_mutation` unconditionally called `cx.notify()` on every entity
/// mutation, causing 2-3 re-renders per retry attempt. By tracking the last
/// status and only notifying on change, `increment_retry()` / `prepare_retry()`
/// calls (which don't change status — it stays Loading) no longer trigger
/// re-renders.
pub struct Observer<R> {
    entity: gpui::WeakEntity<R>,
    config: ObserverConfig,
}

impl<R: ObservableResource + 'static> Observer<R> {
    /// Create a new observer for the given entity.
    pub fn new(entity: &Entity<R>) -> Self {
        Self {
            entity: entity.downgrade(),
            config: ObserverConfig::default(),
        }
    }

    /// Set the observer configuration.
    pub fn with_config(mut self, config: ObserverConfig) -> Self {
        self.config = config;
        self
    }

    /// Start observing the entity. Returns `None` if the entity was already dropped.
    ///
    /// **v2 fix**: Returns `Option<Subscription>` instead of panicking.
    ///
    /// **Audit #71**: takes `&self` (was `&mut self`) — the body only reads the
    /// weak entity handle and the `Copy` config flag, so no interior mutation
    /// is required. `&mut` callers coerce to `&` with no ripple.
    pub fn observe<W: 'static>(&self, cx: &mut Context<W>) -> Option<Subscription> {
        let upgraded = self.entity.upgrade()?;
        let notify_on_change = self.config.notify_on_status_change_only;
        let last_status: Cell<Option<R::Status>> = Cell::new(None);

        let subscription = cx.observe(&upgraded, move |_, entity, cx| {
            let current_status = entity.read(cx).observable_status();
            if notify_on_change {
                let previous = last_status.get();
                if previous != Some(current_status) {
                    last_status.set(Some(current_status));
                    cx.notify();
                }
            } else {
                cx.notify();
            }
        });

        Some(subscription)
    }
}

/// Observer for a [`QueryResource`] (status type [`QueryStatus`]).
pub type QueryObserver<T, E> = Observer<QueryResource<T, E>>;

/// Observer for an [`InfiniteQueryResource`] (status type [`QueryStatus`]).
pub type InfiniteQueryObserver<T, E> = Observer<InfiniteQueryResource<T, E>>;

/// Observer for a [`MutationResource`] (status type [`MutationStatus`]).
pub type MutationObserver<V, T, E> = Observer<MutationResource<V, T, E>>;
