//! Public mutation hooks: `use_mutation`, `mutate`, `mutate_with_callbacks`,
//! `mutate_by_ref`, `mutate_arc`, and `use_mutation_state`.

use std::sync::Arc;

use gpui::{AppContext as _, BorrowAppContext as _, Context, Entity, Subscription};

use crate::client::{MutationObserver, QueryClient};
use crate::core::MutationResource;

use super::internals::{
    run_mutation_loop, run_mutation_loop_by_ref, run_mutation_loop_by_ref_with_callbacks,
    run_mutation_loop_with_callbacks,
};
use super::super::options::MutationCallbacks;
use super::super::MutationOptions;

/// Hook for executing mutations (create, update, delete operations).
///
/// Creates a [`MutationResource`] entity. Returns the entity and a subscription
/// for state observation during render. Use the [`mutate`] helper to trigger the
/// mutation from event handlers.
///
/// Accepts `impl Into<MutationOptions>` so both `use_mutation((), cx)` (using
/// `Default` via `From<()>`) and `use_mutation(MutationOptions { .. }, cx)`
/// work.
///
/// Audit fix #1/#11: Uses `MutationObserver` with status-deduplication instead
/// of a raw `cx.observe`. The observer only calls `cx.notify()` when the
/// mutation's `MutationStatus` actually changes (Idle -> Loading, Loading ->
/// Success, Loading -> Failure). Intermediate updates like `increment_retry()`
/// and `prepare_retry()` do not change status (stays Loading), so they no
/// longer trigger re-renders.
///
/// Audit fix #17: Registers the mutation entity with the global [`QueryClient`]
/// so that `use_mutation_state` returns it, GC is triggered, and
/// `MutationOptions::gc_time_ms` is respected.
///
/// Audit fix #29: Replaces the production `.expect()` on
/// `MutationObserver::observe` with a `debug_assert!` + safe fallback so
/// production builds never panic on a GPUI internal regression.
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
    let entity = cx.new(|_| MutationResource::new(opts.retry_policy.clone()));

    // Audit fix #1/#11: Use MutationObserver with status-deduplication instead
    // of raw cx.observe. The observer only calls cx.notify() when MutationStatus
    // actually changes, preventing excessive re-renders from increment_retry()
    // and prepare_retry() calls that don't change status (stays Loading).
    let mut m_observer = MutationObserver::new(&entity);
    // Audit fix #29: debug_assert + safe fallback instead of .expect() so a
    // GPUI internal regression does not panic production builds.
    let subscription = match m_observer.observe(cx) {
        Some(sub) => sub,
        None => {
            debug_assert!(
                false,
                "MutationObserver::observe failed: entity was just created and \
                 cannot be dropped. This indicates a GPUI internal regression."
            );
            // Return a no-op subscription so the caller can continue.
            Subscription::new(|| {})
        }
    };

    // Audit fix #17: Register the mutation entity with the global QueryClient so
    // that use_mutation_state returns it, GC is triggered, and gc_time_ms
    // is respected.
    if cx.has_global::<QueryClient>() {
        let _ = cx.update_global::<QueryClient, _>(|client, cx| {
            client.register_mutation(&entity, cx);
        });
    }

    (entity, subscription)
}

/// Hook for executing mutations with a custom retry policy.
///
/// Audit fix #68 / CL7 (#111): This deprecated entrypoint now delegates to
/// [`use_mutation`] so it also registers the entity with the global
/// [`QueryClient`] (the previous implementation skipped registration, leaving
/// the mutation invisible to `use_mutation_state` and GC). Keeping the
/// `#[deprecated]` attribute preserves the source-compat migration path.
#[deprecated(
    since = "0.2.0",
    note = "Use `use_mutation(options, cx)` instead — it now accepts MutationOptions via Into"
)]
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

/// Hook to observe all mutation state across the application for a given
/// `(V, T, E)` type triple.
///
/// Returns a snapshot of all [`MutationResource`] entities of the specified
/// types registered in the global [`QueryClient`]. Returns an empty vec
/// if no mutations exist for this type or if no `QueryClient` is set up.
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
/// This is the primary way to execute mutations. It:
/// 1. Transitions the entity to Loading with the given variables
/// 2. Spawns an async task calling the mutator
/// 3. On success, completes with the result data
/// 4. On failure, retries according to the entity's retry policy
///
/// Audit fix #8/#7: Guards against concurrent calls by checking whether the
/// mutation is already in Loading state *inside the same `entity.update` that
/// calls `begin`*, so the check+begin is atomic and a racing caller cannot
/// slip a `begin` in between. If already Loading, returns without starting a
/// new mutation.
///
/// Audit fix #3: Variables are wrapped in `Arc<V>` internally so that the
/// retry loop only performs an `Arc::clone` (cheap reference count increment)
/// per attempt, rather than cloning the full variables payload. For the
/// no-`V::clone`-per-attempt path, prefer [`mutate_by_ref`] or
/// [`mutate_arc`].
///
/// Audit fix #6: The spawned task is stored on the resource via
/// `set_current_task` so a replacement call (or entity drop) aborts the prior
/// in-flight task. Previously the task was `.detach()`ed and kept running
/// after unmount/replacement.
///
/// Audit fix #67: The shared guard/begin/spawn logic lives in
/// [`begin_and_spawn`] and is shared with [`mutate_with_callbacks`].
///
/// Audit fix #119: The unused `+ Clone` bound on `F` has been dropped — the
/// mutator is only ever borrowed, never cloned.
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
    begin_and_spawn(entity, variables, mutator, cx, None);
}

/// Like [`mutate`] but with lifecycle callbacks.
///
/// Callbacks fire on the final outcome (after all retries exhausted or
/// on first success), not on intermediate retry attempts.
///
/// **Important**: Callbacks receive cloned data/error and run *outside* any
/// entity borrow, so they may safely call `entity.update()` or other GPUI
/// mutations without risk of deadlock or panic.
///
/// Audit fix #8/#7: Guards against concurrent calls atomically (see [`mutate`]).
/// Audit fix #9: If the entity is dropped during the mutation, `on_error` and
/// `on_settled` are still invoked so callers always get a terminal callback.
/// Audit fix #3: Variables are wrapped in `Arc<V>` for cheap retries.
/// Audit fix #67: Delegates to the shared [`begin_and_spawn`] helper.
/// Audit fix #119: The unused `+ Clone` bound on `F` has been dropped.
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
    begin_and_spawn(entity, variables, mutator, cx, Some(callbacks));
}

/// Audit fix #3: Like [`mutate`] but the mutator receives `&V` instead of
/// `V`, so the retry loop borrows the variables from the stored `Arc<V>` and
/// performs **no `V::clone` per attempt**. The caller is responsible for
/// cloning `V` inside the mutator only if the fetcher needs an owned value
/// across an `.await` (otherwise no clone is needed at all).
///
/// `V` is still required to be `Clone` because the initial `begin` call
/// stores an owned copy on the resource. Only the retry path is clone-free.
///
/// Audit fix #119: No `+ Clone` bound on `F`.
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
    begin_and_spawn_by_ref(entity, Arc::new(variables), mutator, cx, None);
}

/// Audit fix #3: Like [`mutate_by_ref`] but accepts `Arc<V>` directly, letting
/// the caller share the variables buffer across multiple mutation invocations
/// (or with other readers) without an extra `Arc::new`.
///
/// Audit fix #119: No `+ Clone` bound on `F`.
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
    begin_and_spawn_by_ref(entity, variables, mutator, cx, None);
}

// ── Shared helpers ───────────────────────────────────────────────────────

/// Audit fix #67: Shared guard/begin/spawn for [`mutate`] and
/// [`mutate_with_callbacks`] (legacy `Fn(V) -> Fut` mutator).
///
/// Audit fix #7: The `is_loading` guard and the `begin` transition happen
/// inside the *same* `entity.update` closure, so the check+begin is atomic
/// — a racing caller cannot slip a `begin` in between the check and our own
/// `begin`.
///
/// Audit fix #6: The spawned task is stored on the resource via
/// `set_current_task` so a replacement / unmount aborts the prior task.
fn begin_and_spawn<V, T, E, C, F, Fut>(
    entity: &Entity<MutationResource<V, T, E>>,
    variables: V,
    mutator: F,
    cx: &mut Context<C>,
    callbacks: Option<MutationCallbacks<T, E>>,
) where
    V: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
    C: 'static,
    F: Fn(V) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let variables_arc = Arc::new(variables);

    let began = entity.update(cx, |resource, cx| {
        // Audit fix #7: atomic check+begin.
        if resource.is_loading() {
            return false;
        }
        resource.begin((*variables_arc).clone());
        cx.notify();
        true
    });
    if !began {
        return;
    }

    let retry_policy = entity.read_with(cx, |r, _| r.retry_policy().clone());
    let weak = entity.downgrade();

    let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
        // `mutator` is moved by value into exactly one arm (run_mutation_loop*
        // now own the closure). A `match` keeps the two moves mutually exclusive.
        match callbacks {
            Some(callbacks) => {
                run_mutation_loop_with_callbacks(
                    &weak,
                    variables_arc,
                    mutator,
                    &retry_policy,
                    callbacks,
                    cx,
                )
                .await;
            }
            None => {
                run_mutation_loop(&weak, variables_arc, mutator, &retry_policy, cx).await;
            }
        }
    });
    // Audit fix #6: store the task so it is aborted on replacement / drop.
    let _ = entity.update(cx, |r, cx| {
        r.set_current_task(task);
        cx.notify();
    });
}

/// Audit fix #3/#67: Shared guard/begin/spawn for [`mutate_by_ref`] and
/// [`mutate_arc`] (new `Fn(&V) -> Fut` mutator). The retry loop borrows the
/// variables from the `Arc<V>` and performs no `V::clone` per attempt.
///
/// Preserves the #7 atomic-check fix and the #6 task-storage fix.
fn begin_and_spawn_by_ref<V, T, E, C, F, Fut>(
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
        // Audit fix #7: atomic check+begin.
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

    let task: gpui::Task<()> = cx.spawn(async move |_this, cx| {
        match callbacks {
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
        }
    });
    // Audit fix #6: store the task so it is aborted on replacement / drop.
    let _ = entity.update(cx, |r, cx| {
        r.set_current_task(task);
        cx.notify();
    });
}
