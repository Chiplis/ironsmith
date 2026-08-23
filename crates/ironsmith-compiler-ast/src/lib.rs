//! Compiler provenance, symbols, and grammar-neutral leaf AST vocabulary.

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub mod effect {
    pub use ironsmith_core::Value;
}

pub mod target {
    pub use ironsmith_core::{ObjectFilter, PlayerFilter};
}

pub mod types {
    pub use ironsmith_core::{CardType, Subtype, Supertype};
}

#[path = "../../ironsmith-compiler/src/model/provenance.rs"]
pub mod provenance;

pub mod model {
    pub use crate::provenance;
    pub use crate::symbols;
}

#[path = "../../ironsmith-compiler/src/parse_context.rs"]
pub mod parse_context;
#[path = "../../ironsmith-compiler/src/model/parse_types.rs"]
pub mod parse_types;
#[path = "../../ironsmith-compiler/src/model/restrictions.rs"]
pub mod restrictions;
#[path = "../../ironsmith-compiler/src/model/symbols.rs"]
pub mod symbols;

pub use parse_context::*;
pub use parse_types::*;
pub use provenance::*;
pub use restrictions::*;
pub use symbols::*;
