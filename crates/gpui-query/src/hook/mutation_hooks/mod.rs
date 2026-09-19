//! Mutation hooks and internal retry loops.
//!
//! `use_mutation_with_options` is deprecated and deliberately not re-exported:
//! the `pub use` would fire the deprecation lint on every import.

mod hooks;
mod internals;

pub use hooks::{
    mutate, mutate_arc, mutate_by_ref, mutate_with_callbacks, use_mutation, use_mutation_state,
};
