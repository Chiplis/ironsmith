//! Lossless source normalization and compiler CST containers.

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub use ironsmith_compiler_syntax::lexer;

pub use ironsmith_compiler_syntax::{slice_primitives, string_primitives};

pub mod provenance;

pub mod model {
    pub use crate::provenance;
}

pub mod preprocess;
pub mod source_model;

pub mod front_end {
    pub use crate::document_cst::*;
    pub use crate::preprocess::*;
    pub use crate::source_model::*;
}

pub mod document_cst;

pub use document_cst::*;
pub use preprocess::*;
pub use provenance::*;
pub use source_model::*;
