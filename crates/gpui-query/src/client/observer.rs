//! The three observer kinds (`QueryObserver`, `InfiniteQueryObserver`,
//! `MutationObserver`) are aliases over one generic [`Observer<R>`]; they
//! differ only in entity and status type.

use std::cell::Cell;

use gpui::{Context, Entity, Subscription};

use crate::core::{
    InfiniteQueryResource, MutationResource, MutationStatus, QueryResource, QueryStatus,
};

/// Surfaces each resource's inherent `status()` generically (with a `Status`
/// assoc type) so [`Observer<R>`] can dedup notifications for any kind.
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

#[derive(Clone, Debug)]
pub struct ObserverConfig {
    pub notify_on_status_change_only: bool,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            notify_on_status_change_only: true,
        }
    }
}

/// With the default config, `cx.notify()` fires only when the status
/// actually changes, so same-status updates (retry count increments,
/// `prepare_retry`) do not re-render.
pub struct Observer<R> {
    entity: gpui::WeakEntity<R>,
    config: ObserverConfig,
}

impl<R: ObservableResource + 'static> Observer<R> {
    pub fn new(entity: &Entity<R>) -> Self {
        Self {
            entity: entity.downgrade(),
            config: ObserverConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ObserverConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns `None` if the entity was already dropped; takes `&self` since
    /// the body only reads the weak handle and the `Copy` config flag.
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

pub type QueryObserver<T, E> = Observer<QueryResource<T, E>>;

pub type InfiniteQueryObserver<T, E> = Observer<InfiniteQueryResource<T, E>>;

pub type MutationObserver<V, T, E> = Observer<MutationResource<V, T, E>>;
