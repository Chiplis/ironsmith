//! Compatibility facade for the extracted semantic AST.

pub use ironsmith_compiler_semantic::model::*;

#[path = "card_document.rs"]
pub(crate) mod card_document;
pub(crate) use card_document::{ParsedCardAst, ParsedCleaveBranch, ParsedOverloadBranch};
