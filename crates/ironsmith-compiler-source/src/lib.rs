//! Lossless source normalization and compiler CST containers.

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub use ironsmith_compiler_syntax::lexer;

pub mod model {
    pub use ironsmith_compiler_ast::provenance;
}

#[path = "../../ironsmith-compiler/src/front_end/cst_primitives.rs"]
pub mod cst_primitives;
#[path = "../../ironsmith-compiler/src/front_end/preprocess.rs"]
pub mod preprocess;
#[path = "../../ironsmith-compiler/src/front_end/source_model.rs"]
pub mod source_model;

pub mod front_end {
    pub use crate::cst_primitives::*;
    pub use crate::preprocess::*;
    pub use crate::source_model::*;
    pub use ironsmith_compiler_syntax::*;
}

#[path = "../../ironsmith-compiler/src/front_end/document_cst.rs"]
pub mod document_cst;
#[path = "../../ironsmith-compiler/src/front_end/document_structure.rs"]
pub mod document_structure;

pub use cst_primitives::*;
pub use document_cst::*;
pub use document_structure::*;
pub use preprocess::*;
pub use source_model::*;
