//! Mutation hooks and internals — `use_mutation`, `mutate`, `mutate_with_callbacks`,
//! `mutate_by_ref`, `mutate_arc`, and the internal retry loops.
//!
//! Audit fix #22: `use_mutation_with_options` is intentionally NOT re-exported
//! from the module public surface. The function itself remains defined (it is
//! `#[deprecated]` and delegates to `use_mutation`) so existing call sites
//! that import it via the full path keep compiling with a deprecation warning,
//! but the `pub use` re-export no longer fires the deprecated-lint-on-re-export
//! warning under `clippy::style`.

mod hooks;
mod internals;

pub use hooks::{
    mutate, mutate_arc, mutate_by_ref, mutate_with_callbacks, use_mutation, use_mutation_state,
};
