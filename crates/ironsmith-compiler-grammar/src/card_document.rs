//! The front end's card-level result.
//!
//! Recognition produces it and lowering consumes it, so the type itself is
//! owned by the semantic crate that both phases can see.

pub use ironsmith_compiler_semantic::card_document::*;
