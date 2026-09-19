//! `use_query` and `use_mutation` hooks: query and mutation subscriptions for
//! GPUI components.
//!
//! The primary API is options-first. The fetcher always receives a
//! [`QuerySignal`](crate::core::QuerySignal) for cooperative cancellation.
//! `use_query_unsignalled` remains for callers that want a signal-free fetcher.
//!
//! # Query usage
//!
//! ```no_run
//! use gpui_query::hook::use_query;
//! use gpui_query::{QueryOptions, CachePolicy, RequestPolicy};
//! # #[derive(Clone)]
//! # struct User;
//! # #[derive(Clone, Debug)]
//! # struct MyError;
//!
//! struct MyView {
//!     users: gpui::Entity<gpui_query::QueryResource<Vec<User>, MyError>>,
//!     _subscription: gpui::Subscription,
//! }
//!
//! impl MyView {
//!     fn new(cx: &mut gpui::Context<Self>) -> Self {
//!         let (users, _subscription) = use_query(
//!             QueryOptions::new("users")
//!                 .cache_policy(CachePolicy::Ttl { ttl_ms: 60_000 })
//!                 .request_policy(RequestPolicy::LatestWins),
//!             |signal| async move {
//!                 // Your async fetcher here
//!                 Ok(vec![])
//!             },
//!             cx,
//!         );
//!         Self { users, _subscription }
//!     }
//! }
//! ```
//!
//! # Mutation usage
//!
//! ```no_run
//! use gpui_query::hook::{use_mutation, mutate};
//! # #[derive(Clone)]
//! # struct NewUser { name: String }
//! # #[derive(Clone)]
//! # struct User;
//! # #[derive(Clone, Debug)]
//! # struct MyError;
//!
//! struct MyView {
//!     create_user: gpui::Entity<gpui_query::MutationResource<NewUser, User, MyError>>,
//!     _subscription: gpui::Subscription,
//! }
//!
//! impl MyView {
//!     fn new(cx: &mut gpui::Context<Self>) -> Self {
//!         let (entity, sub) = use_mutation((), cx);
//!         Self { create_user: entity, _subscription: sub }
//!     }
//!
//!     fn handle_submit(&mut self, name: String, cx: &mut gpui::Context<Self>) {
//!         mutate(&self.create_user, NewUser { name }, |vars| async move {
//!             Ok(User)
//!         }, cx);
//!     }
//! }
//! ```
//!
//! # Discard behavior
//!
//! Async tasks here access the owning entity through
//! [`gpui::WeakEntity::upgrade()`]. If the component unmounts while a fetch is
//! in-flight, `upgrade()` returns `None` and the result is silently discarded:
//! no callback or notification fires. Stale writes are prevented by the
//! two-phase `accept_current_request` + complete protocol rather than by
//! aborting tasks.

mod fetch_retry;
mod gpui_compat;
mod mutation_hooks;
mod options;
mod query_hooks;
mod use_infinite_query;
mod use_query_select;

// Source-compat shim: read entities regardless of whether `read_with` returns
// `R` (older gpui / git) or `Result<R>` (gpui 0.2.2 / crates.io).
pub(crate) use gpui_compat::read_entity;

pub use options::{InfiniteQueryOptions, MutationCallbacks, MutationOptions, QueryOptions};

pub use query_hooks::{
    fetch_query, fetch_query_with_policy, fetch_query_with_signal, use_query, use_query_manual,
    use_query_manual_opts, use_query_unsignalled, use_query_unsignalled_opts,
    use_query_with_policy,
};

pub use use_infinite_query::{
    fetch_next_page_infinite, fetch_previous_page_infinite, use_infinite_query,
};

pub use use_query_select::use_query_select;

// `use_mutation_with_options` is deprecated and deliberately not re-exported;
// callers that reach it via the full path still get the deprecation warning.
pub use mutation_hooks::{
    mutate, mutate_arc, mutate_by_ref, mutate_with_callbacks, use_mutation, use_mutation_state,
};

/// Current time as milliseconds since the UNIX epoch.
///
/// A clock that reports a time before the epoch clamps to `0`, which callers
/// treat as "ancient" (stale). No panic is propagated.
#[inline]
pub fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lets `use_mutation((), cx)` use the default options.
impl From<()> for MutationOptions {
    fn from((): ()) -> Self {
        Self::default()
    }
}
