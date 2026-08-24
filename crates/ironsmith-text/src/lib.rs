//! Canonical card and effect rendering, deliberately outside gameplay wasm.

// Renderer matchers mirror authored clause structure, assemble filters incrementally, and use
// explicit guards to keep rejection points reviewable. These local lint allowances document that
// surface-preservation style without relaxing warnings for the rest of the workspace.
#![allow(
    clippy::enum_variant_names,
    clippy::field_reassign_with_default,
    clippy::if_same_then_else,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::question_mark,
    clippy::too_many_arguments,
    clippy::wrong_self_convention
)]

pub use ::engine::*;

pub mod compiled_text;
pub(crate) mod effect_text_shared;
pub(crate) mod perf;
pub(crate) mod text_cleanup;

#[cfg(test)]
pub(crate) mod compiler_test_support;

#[cfg(test)]
pub use compiler_test_support::CardDefinitionBuilder;

pub use compiled_text::{
    ability_surface_text, ability_surface_texts, canonical_compiled_lines, compile_effect_list,
    compiled_text_lines, debug_compiled_lines, unprocessed_compiled_lines,
};
