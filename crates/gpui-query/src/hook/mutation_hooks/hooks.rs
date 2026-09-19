//! Public mutation hooks: `use_mutation`, `mutate`, `mutate_with_callbacks`,
//! `mutate_by_ref`, `mutate_arc`, and `use_mutation_state`.

use std::sync::Arc;

use gpui::{AppContext as _, BorrowAppContext as _, Context, Entity, Subscription};

use crate::client::{MutationObserver, QueryClient};
use crate::core::MutationResource;

use super::super::MutationOptions;
use super::super::options::MutationCallbacks;
use super::internals::{run_mutation_loop_by_ref, run_mutation_loop_by_ref_with_callbacks};

/// Hook for executing mutations (create, update, delete operations).
///
/// Creates a [`MutationResource`] entity and returns it with a subscription
/// for state observation during render. Trigger it with [`mutate`] from event
/// handlers. Accepts `impl Into<MutationOptions>`, so both `use_mutation((), cx)`
/// and `use_mutation(MutationOptions::default(), cx)` work.
///
/// The observer dedupes on `MutationStatus`: intermediate updates like
/// `increment_retry()` stay in Loading and do not trigger re-renders. The
/// entity is registered with the global [`QueryClient`] so `use_mutation_state`
/// finds it and GC respects `gc_time_ms`.
///
/// # Example
///
/// ```no_run
/// use gpui::{Entity, Subscription, Context};
/// use gpui_query::hook::{use_mutation, mutate};
/// use gpui_query::MutationResource;
/// # #[derive(Clone)]
/// # struct NewUser { name: String }
/// # #[derive(Clone)]
/// # struct User;
/// # #[derive(Clone, Debug)]
/// # struct MyError;
///
/// struct MyView {
///     create_user: Entity<MutationResource<NewUser, User, MyError>>,
///     _mutation_sub: Subscription,
/// }
///
/// impl MyView {
///     fn new(cx: &mut Context<Self>) -> Self {
///         let (entity, sub) = use_mutation((), cx);
///         Self { create_user: entity, _mutation_sub: sub }
///     }
///
///     fn handle_submit(&mut self, name: String, cx: &mut Context<Self>) {
///         mutate(&self.create_user, NewUser { name }, |vars| async move {
///             Ok(User)
///         }, cx);
///     }
/// }
/// ```
pub fn use_mutation<V, T, E, C>(
    options: impl Into<MutationOptions>,
    cx: &mut Context<C>,
) -> (Entity<MutationResource<V, T, E>>, Subscription)
where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    let opts = options.into();
    let entity = cx.new(|_| MutationResource::new(opts.retry_policy));

    let observer = MutationObserver::new(&entity);
    let subscription = match observer.observe(cx) {
        Some(sub) => sub,
        None => {
            // The entity was just created, so this only fires on a GPUI
            // internal regression. Do not panic production builds.
            debug_assert!(
                false,
                "MutationObserver::observe failed: entity was just created and \
                 cannot be dropped. This indicates a GPUI internal regression."
            );
            Subscription::new(|| {})
        }
    };

    if cx.has_global::<QueryClient>() {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_mutation(&entity, cx);
        });
    }

    (entity, subscription)
}

/// Hook for executing mutations with a custom retry policy. Deprecated alias
/// of [`use_mutation`], which now accepts `MutationOptions` directly.
#[deprecated(
    since = "0.2.0",
    note = "Use `use_mutation(options, cx)` instead — it now accepts MutationOptions via Into"
)]
// Retained for the deprecated source-compat path and exercised by
// `test_deprecated_use_mutation_with_options_still_works`.
#[allow(dead_code)]
pub fn use_mutation_with_options<V, T, E, C>(
    options: &MutationOptions,
    cx: &mut Context<C>,
) -> (Entity<MutationResource<V, T, E>>, Subscription)
where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    use_mutation(options.clone(), cx)
}

/// Observe all mutation state across the application for a given
/// `(V, T, E)` type triple. Returns an empty vec if no mutations of this type
/// exist or no [`QueryClient`] is set up.
///
/// # Example
///
/// ```no_run
/// use gpui_query::hook::use_mutation_state;
/// use gpui_query::MutationResource;
/// # #[derive(Clone)]
/// # struct NewUser;
/// # #[derive(Clone)]
/// # struct User;
/// # #[derive(Clone, Debug)]
/// # struct QueryError;
/// # fn _doc<C: 'static>(cx: &mut gpui::Context<C>) {
///
/// let mutations = use_mutation_state::<NewUser, User, QueryError, _>(cx);
/// for entity in &mutations {
///     let status = entity.read(cx).status();
///     // ...
/// }
/// # }
/// ```
pub fn use_mutation_state<V, T, E, C>(cx: &mut Context<C>) -> Vec<Entity<MutationResource<V, T, E>>>
where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    if cx.has_global::<QueryClient>() {
        cx.read_global::<QueryClient, _>(|client, _| client.all_mutations::<V, T, E>())
    } else {
        Vec::new()
    }
}

/// Trigger a mutation on an existing mutation entity.
///
/// Transitions the entity to Loading with the given variables, spawns the
/// mutator, and retries per the entity's policy. Variables are wrapped in an
/// `Arc<V>` so each retry only clones once; prefer [`mutate_by_ref`] or
/// [`mutate_arc`] to skip the per-attempt `V::clone` entirely.
///
/// A call while the mutation is already Loading is a no-op: the check and the
/// `begin` transition happen inside one `entity.update`, so racing callers
/// cannot both start. The spawned task is stored on the resource, so a
/// replacement call or entity drop aborts a prior in-flight task.
///
/// # Example
///
/// ```no_run
/// use gpui_query::hook::mutate;
/// # #[derive(Clone)]
/// # struct Vars;
/// # #[derive(Clone)]
/// # struct Data;
/// # #[derive(Clone, Debug)]
/// # struct Err;
/// # fn _doc(entity: &gpui::Entity<gpui_query::MutationResource<Vars, Data, Err>>, cx: &mut gpui::Context<()>) {
///
/// mutate(entity, Vars, |v| async move { Ok(Data) }, cx);
/// # }
/// ```
pub fn mutate<V, T, E, C, F, Fut>(
    entity: &Entity<MutationResource<V, T, E>>,
    variables: V,
    mutator: F,
    cx: &mut Context<C>,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    // One V::clone per attempt, matching the Fn(V) mutator contract.
    begin_and_spawn(
        entity,
        Arc::new(variables),
        move |v: &V| mutator(v.clone()),
        cx,
        None,
    );
}

/// Like [`mutate`] but with lifecycle callbacks.
///
/// Callbacks fire on the final outcome (first success or retries exhausted),
/// never on intermediate attempts. They receive cloned data/error and run
/// outside any entity borrow, so they may safely call `entity.update()`. If
/// the entity is dropped mid-mutation, `on_error` and `on_settled` still fire
/// so callers always get a terminal callback.
pub fn mutate_with_callbacks<V, T, E, C, F, Fut>(
    entity: &Entity<MutationResource<V, T, E>>,
    variables: V,
    mutator: F,
    callbacks: MutationCallbacks<T, E>,
    cx: &mut Context<C>,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    begin_and_spawn(
        entity,
        Arc::new(variables),
        move |v: &V| mutator(v.clone()),
        cx,
        Some(callbacks),
    );
}

/// Like [`mutate`] but the mutator receives `&V`, so the retry loop borrows
/// the variables from the stored `Arc<V>` and performs no `V::clone` per
/// attempt. Clone inside the mutator only if it needs an owned value across an
/// `.await`.
///
/// `V` is still `Clone` because `begin` stores an owned copy on the resource.
pub fn mutate_by_ref<V, T, E, C, F, Fut>(
    entity: &Entity<MutationResource<V, T, E>>,
    variables: V,
    mutator: F,
    cx: &mut Context<C>,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(&V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    begin_and_spawn(entity, Arc::new(variables), mutator, cx, None);
}

/// Like [`mutate_by_ref`] but accepts `Arc<V>` directly, letting the caller
/// share the variables buffer across invocations without an extra `Arc::new`.
pub fn mutate_arc<V, T, E, C, F, Fut>(
    entity: &Entity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    cx: &mut Context<C>,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(&V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    begin_and_spawn(entity, variables, mutator, cx, None);
}

/// Shared guard/begin/spawn for every `mutate*` entrypoint.
///
/// The `is_loading` guard and the `begin` transition happen inside one
/// `entity.update` so racing callers cannot both begin. The spawned task is
/// stored via `set_current_task` so replacement or drop aborts it.
fn begin_and_spawn<V, T, E, C, F, Fut>(
    entity: &Entity<MutationResource<V, T, E>>,
    variables: Arc<V>,
    mutator: F,
    cx: &mut Context<C>,
    callbacks: Option<MutationCallbacks<T, E>>,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(&V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let began = entity.update(cx, |resource, cx| {
        if resource.is_loading() {
            return false;
        }
        resource.begin((*variables).clone());
        cx.notify();
        true
    });
    if !began {
        return;
    }

    let retry_policy = entity.read_with(cx, |r, _| r.retry_policy().clone());
    let weak = entity.downgrade();

    let task: gpui::Task<()> = cx.spawn(async move |_this, cx| match callbacks {
        Some(callbacks) => {
            run_mutation_loop_by_ref_with_callbacks(
                &weak,
                variables,
                mutator,
                &retry_policy,
                callbacks,
                cx,
            )
            .await;
        }
        None => {
            run_mutation_loop_by_ref(&weak, variables, mutator, &retry_policy, cx).await;
        }
    });
    // No notify: set_current_task does not change status (already Loading
    // from begin, which notified).
    entity.update(cx, |r, _| {
        r.set_current_task(task);
    });
}
