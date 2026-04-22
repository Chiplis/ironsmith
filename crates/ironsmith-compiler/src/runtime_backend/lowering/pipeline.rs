use crate::cards::ParseAnnotations;
use crate::cards::builders::{CardDefinition, CardDefinitionBuilder, CardTextError};
use crate::parse_trace;

use super::document_parser;
use super::effect_pipeline::{NormalizedCardAst, ParsedCardAst};
use super::ir::RewriteSemanticDocument;
use super::lower;

pub(crate) fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    document_parser::parse_text_to_semantic_document(builder, text, allow_unsupported)
}

#[allow(dead_code)]
pub(crate) fn lower_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let parsed = parse_semantic_document(doc)?;
    let prepared = prepare_parsed_document(parsed)?;
    lower_prepared_document(prepared)
}

pub(crate) fn parse_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<ParsedCardAst, CardTextError> {
    lower::rewrite_document_to_parsed_card_ast(doc)
}

pub(crate) fn prepare_parsed_document(
    ast: ParsedCardAst,
) -> Result<NormalizedCardAst, CardTextError> {
    lower::prepare_parsed_card_ast_for_lowering(ast)
}

#[allow(dead_code)]
pub(crate) fn prepare_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<NormalizedCardAst, CardTextError> {
    prepare_parsed_document(parse_semantic_document(doc)?)
}

pub(crate) fn lower_prepared_document(
    ast: NormalizedCardAst,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    lower::lower_normalized_card_ast(ast)
}

pub(crate) fn parse_text_with_annotations_lowered(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let (doc, _) = parse_text_to_semantic_document(builder, text, allow_unsupported)?;
    let parsed = {
        let _scope = parse_trace::scope("semantic parse");
        parse_semantic_document(doc)?
    };
    let prepared = {
        let _scope = parse_trace::scope("prepare lowering input");
        prepare_parsed_document(parsed)?
    };
    let lowered = {
        let _scope = parse_trace::scope("lower runtime definition");
        lower_prepared_document(prepared)?
    };
    Ok(lowered)
}

pub(crate) fn parse_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    parse_text_with_annotations_lowered(builder, text, allow_unsupported)
}
