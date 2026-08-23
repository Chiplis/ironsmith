//! Compatibility facade for the extracted compiler implementation.
//!
//! Grammar, document parsing, normalization, reference resolution, and
//! lowering compile in their owned workspace crates. Existing consumers keep
//! the historical `ironsmith_compiler` import path through this re-export.

pub use ironsmith_compiler_grammar::*;
