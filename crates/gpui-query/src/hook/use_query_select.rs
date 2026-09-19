//! The `use_query_select` hook: combines `use_query` with a
//! [`SelectTransform`].
//!
//! Mirrors TanStack Query's `select` option: cached data is projected into a
//! derived shape, re-running only when the data changes. Rust needs `U` (the
//! transform output) at compile time and `QueryOptions` is not generic, so the
//! transform is a separate parameter and the hook returns a
//! `MappedQueryResource<T, U, E>` entity.
//!
//! # Usage
//!
//! ```no_run
//! use gpui_query::hook::{use_query_select, QueryOptions};
//! use gpui_query::core::SelectTransform;
//! # #[derive(Clone, PartialEq)]
//! # struct User;
//! # #[derive(Clone, Debug)]
//! # struct MyError;
//!
//! struct UserCountView {
//!     mapped: gpui::Entity<gpui_query::core::MappedQueryResource<Vec<User>, usize, MyError>>,
//!     _subs: (gpui::Subscription, gpui::Subscription),
//! }
//!
//! impl UserCountView {
//!     fn new(cx: &mut gpui::Context<Self>) -> Self {
//!         let count_transform = SelectTransform::new(|users: &Vec<User>| users.len());
//!         let (mapped, query_entity, subs) = use_query_select(
//!             QueryOptions::new("users"),
//!             count_transform,
//!             |signal| async move {
//!                 // Your async fetcher here
//!                 Ok(vec![])
//!             },
//!             cx,
//!         );
//!         Self { mapped, _subs: subs }
//!     }
//! }
//! ```

use std::sync::Arc;

use gpui::{AppContext as _, Context, Entity, Subscription};

use crate::core::{MappedQueryResource, QueryResource, SelectTransform};

use super::{QueryOptions, use_query};

/// The result of [`use_query_select`]: the projected view entity, the
/// underlying query entity, and the subscriptions that keep both observations
/// alive.
pub type QuerySelectResult<T, U, E> = (
    Entity<MappedQueryResource<T, U, E>>,
    Entity<QueryResource<T, E>>,
    (Subscription, Subscription),
);

/// Subscribe to a query and project its data through a [`SelectTransform`].
///
/// Creates the underlying query via [`use_query`], seeds a
/// `MappedQueryResource<T, U, E>` with the current data, and observes the
/// source entity so the mapped resource refreshes whenever the data actually
/// changes. The transform itself runs lazily on every `mapped.data()` call
/// (no output cache), so reuse the result if it is expensive:
///
/// ```no_run
/// use gpui_query::core::{MappedQueryResource, SelectTransform};
/// # fn _doc(mapped: &gpui::Entity<MappedQueryResource<Vec<String>, usize, ()>>, cx: &gpui::App) {
/// let count = mapped.read(cx).data(); // transform runs once; reuse `count`
/// # }
/// ```
pub fn use_query_select<T, U, E, C, F, Fut>(
    options: impl Into<QueryOptions>,
    transform: SelectTransform<T, U>,
    fetcher: F,
    cx: &mut Context<C>,
) -> QuerySelectResult<T, U, E>
where
    T: Clone + PartialEq + Send + Sync + 'static,
    U: 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(crate::core::QuerySignal) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let (query_entity, query_subscription) = use_query(options, fetcher, cx);

    // Seed the mapped resource with whatever data the query has now. The
    // source owns `T` by value and only lends `&T`, so this is the one
    // unavoidable clone.
    let initial_data: Option<Arc<T>> =
        query_entity.read_with(cx, |r, _| r.data().map(|d| Arc::new(d.clone())));
    let mapped = MappedQueryResource::new(initial_data, transform);
    let mapped_entity = cx.new(|_| mapped);

    // Keep the mapped resource in sync. On each notification, bump the cached
    // `Arc<T>` out (cheap), compare `&T` vs `&T` without cloning, and only
    // clone + update + notify when the content actually changed. The borrow on
    // `mapped` ends before the source read, so nothing nests.
    let mapped_weak = mapped_entity.downgrade();
    let mapped_subscription = cx.observe(&query_entity, move |_, entity, cx| {
        if let Some(mapped) = mapped_weak.upgrade() {
            let cached: Option<Arc<T>> = mapped.read_with(cx, |m, _| m.source_arc());

            let changed = entity.read_with(cx, |r, _| match (&cached, r.data()) {
                (Some(c), Some(fresh)) => c.as_ref() != fresh,
                (None, None) => false,
                _ => true,
            });

            if changed {
                let fresh: Option<Arc<T>> = entity.read(cx).data().map(|d| Arc::new(d.clone()));
                mapped.update(cx, |m, cx2| {
                    m.update_source(fresh);
                    cx2.notify();
                });
            }
        }
    });

    (
        mapped_entity,
        query_entity,
        (query_subscription, mapped_subscription),
    )
}
