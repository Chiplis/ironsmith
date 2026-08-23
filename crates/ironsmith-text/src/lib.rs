//! Canonical card and effect rendering, deliberately outside gameplay wasm.

pub use ::engine::*;

pub mod compiled_text;
pub(crate) mod effect_text_shared;
pub(crate) mod perf;
pub(crate) mod text_cleanup;

pub use compiled_text::{
    ability_surface_text, canonical_compiled_lines, compile_effect_list, compiled_text_lines,
    debug_compiled_lines, unprocessed_compiled_lines,
};
