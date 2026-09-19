//! Mutation hooks and internals: `use_mutation`, `mutate`, `mutate_with_callbacks`,
//! `mutate_by_ref`, `mutate_arc`, and the internal retry loops.
//!
//! `use_mutation_with_options` is deprecated and deliberately not re-exported
//! from the public surface; the `pub use` list would otherwise fire the
//! deprecated lint on every import of this module.

mod hooks;
mod internals;

pub use hooks::{
    mutate, mutate_arc, mutate_by_ref, mutate_with_callbacks, use_mutation, use_mutation_state,
};
