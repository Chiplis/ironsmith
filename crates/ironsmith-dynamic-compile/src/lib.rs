//! Legacy in-process source compilation adapter.
//!
//! Browser release builds use the independent compiler wasm module. Native
//! callers and compatibility builds may enable this adapter explicitly.

pub use ironsmith_compiler::CardDefinitionBuilder as CompilerCardDefinitionBuilder;
pub use ironsmith_registry::{
    compile_builder_to_runtime_definition, compile_to_runtime_definition,
};
