//! Converts the typed rewrite/CST output into the semantic card AST.
//!
//! This is the final front-end stage. Preparation and runtime lowering consume
//! the returned [`ParsedCardAst`] and never inspect Oracle tokens themselves.

use crate::cards::builders::{
    CardTextError, LineAst, ParsedCardItem, ParsedLevelAbilityAst, ParsedLineAst,
    ParsedRestrictions, TriggerSpec,
};
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::model::{ParsedCardAst, ParsedCleaveBranch, ParsedOverloadBranch};

use super::ir::{RewriteSemanticDocument, RewriteSemanticItem};
use super::semantic_line_parsing::rewrite_modal_to_parsed_item;

fn unsupported_line_ast(raw_line: &str, reason: impl Into<String>) -> LineAst {
    LineAst::StaticAbility(StaticAbility::unsupported_parser_line(raw_line, reason).into())
}

#[path = "semantic_document/core.rs"]
mod core_programs;
pub use core_programs::parse_semantic_document;
use core_programs::{parse_rewrite_item, parse_rewrite_items};
