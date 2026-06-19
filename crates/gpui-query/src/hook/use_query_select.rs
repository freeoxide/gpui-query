//! The `use_query_select` hook — combines `use_query` with a [`SelectTransform`].
//!
//! TanStack Query's `select` option transforms cached data into a derived shape,
//! re-running only when data changes. This module provides the same pattern for
//! gpui-query: it wraps a [`QueryResource`] with a [`MappedQueryResource`]
//! entity that applies the transform on each observer notification.
//!
//! # Why a separate hook?
//!
//! Rust's type system requires knowing `T` (source) and `U` (output) at compile
//! time. Since `QueryOptions` is not generic over a transform output type, the
//! `select` field cannot live on `QueryOptions` without making the entire options
//! struct generic. Instead, [`use_query_select`] is a standalone hook that accepts
//! the transform as a separate parameter and returns a
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

use super::{use_query, QueryOptions};

/// The result of [`use_query_select`]: the projected view entity, the
/// underlying query entity, and the pair of subscriptions that keep both
/// observations alive.
///
/// Introduced as a type alias (audit #96) to satisfy `clippy::type_complexity`
/// on the public hook signature and to give callers a name to reference.
pub type QuerySelectResult<T, U, E> = (
    Entity<MappedQueryResource<T, U, E>>,
    Entity<QueryResource<T, E>>,
    (Subscription, Subscription),
);

/// Subscribe to a query and project its data through a [`SelectTransform`].
///
/// This is the "select" integration point for the hook layer (audit #3, HIGH
/// finding). It:
///
/// 1. Calls [`use_query`] to create/subscribe to the underlying `QueryResource`.
/// 2. Creates a `MappedQueryResource<T, U, E>` entity seeded with the current
///    source data.
/// 3. Observes the source entity so that every time it changes, the mapped
///    resource's source data is updated from the fresh `QueryResource::data()`.
///    The transform itself is applied lazily when
///    [`MappedQueryResource::data()`] is called.
///
/// # Returns
///
/// A tuple of:
/// - `Entity<MappedQueryResource<T, U, E>>` — the projected view entity
/// - `Entity<QueryResource<T, E>>` — the underlying query entity (for status,
///   error, refetch, etc.)
/// - `(Subscription, Subscription)` — the query subscription and the mapped
///   observer subscription. Store both to keep observations alive.
///
/// # Transform cost
///
/// The transform closure runs every time `mapped.data()` is called (no output
/// cache). For expensive transforms, cache the result:
///
/// ```no_run
/// use gpui_query::core::{MappedQueryResource, SelectTransform};
/// # fn _doc(mapped: &gpui::Entity<MappedQueryResource<Vec<String>, usize, ()>>, cx: &gpui::App) {
///
/// let count = mapped.read(cx).data(); // transform runs once
/// // reuse `count` below
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
    // Step 1: Create the underlying query entity and start the fetch.
    let (query_entity, query_subscription) = use_query(options, fetcher, cx);

    // Step 2: Seed the mapped resource with whatever data the query has now.
    // The source `QueryResource` owns `T` by value and only lends `&T`, so the
    // initial seed clones `T` once into an `Arc<T>` (audit #20). Subsequent
    // updates (Step 3) only re-clone `T` when the content has actually changed.
    let initial_data: Option<Arc<T>> =
        query_entity.read_with(cx, |r, _| r.data().map(|d| Arc::new(d.clone())));
    let mapped = MappedQueryResource::new(initial_data, transform);
    let mapped_entity = cx.new(|_| mapped);

    // Step 3: Observe the query entity so the mapped resource stays in sync.
    // Every time the query entity is updated (fetch completes, refetch, cache
    // invalidation, etc.), we read the fresh source data once, compare it
    // against the cached source, and only notify + re-store when it changed.
    //
    // Audit H1: the previous version cloned `T` into a fresh `Arc<T>` on
    // EVERY notification (even the common case where data was unchanged) just
    // to drive the `PartialEq` comparison. This version avoids that O(|T|)
    // clone on unchanged notifications:
    //   1. Clone the cached `Arc<T>` out of the mapped resource first via
    //      `source_arc()` — a cheap refcount bump, no `T` clone. The mapped
    //      borrow ends with that call (audit #115 preserved: no nested borrow).
    //   2. Read the fresh `&T` straight from the source entity and compare
    //      `&T` vs `&T` without cloning `T`.
    //   3. Only when the content actually changed do we clone `T` into an
    //      `Arc<T>` to hand to `update_source`, exactly as before.
    // Net: unchanged notifications (the common case) pay one cheap `Arc::clone`
    // instead of a full `T` clone + allocation; changed notifications behave
    // identically (same `update_source` + `notify`).
    //
    // Audit fix #4 / #20: source data is still stored as `Option<Arc<T>>`, so
    // `MappedQueryResource` clones (derived views, entity cloning) remain cheap
    // `Arc::clone`s and storage stays shared.
    let mapped_weak = mapped_entity.downgrade();
    let mapped_subscription = cx.observe(&query_entity, move |_, entity, cx| {
        if let Some(mapped) = mapped_weak.upgrade() {
            // Audit fix #115 / H1 step 1: Read the cached source `Arc<T>` out
            // of the mapped resource FIRST, as an owned value (cheap refcount
            // bump via `source_arc`). The mapped borrow ends here, so the
            // `entity.read(cx)` below does NOT nest inside it.
            let cached: Option<Arc<T>> = mapped.read_with(cx, |m, _| m.source_arc());

            // H1 step 2: Compare cached `&T` vs fresh `&T` WITHOUT cloning T.
            // `cached` is owned, so no nested borrow is taken on `mapped`.
            let changed =
                entity.read_with(cx, |r, _| match (&cached, r.data()) {
                    (Some(c), Some(fresh)) => c.as_ref() != fresh,
                    (None, None) => false,
                    _ => true,
                });

            if changed {
                // H1 step 3: Only now clone `T` into an `Arc<T>` for the
                // update (the source `QueryResource` owns `T` by value and only
                // lends `&T`, so this single clone is unavoidable on change).
                let fresh: Option<Arc<T>> = entity
                    .read(cx)
                    .data()
                    .map(|d| Arc::new(d.clone()));
                // Audit fix #116: Notify after updating the mapped source so
                // third-party observers of the mapped entity (not just the
                // primary caller, which already re-renders via the query
                // subscription) see the derived change. Safe and correct.
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
