use crate::cards::ParseAnnotations;
use crate::cards::builders::{CardDefinition, CardDefinitionBuilder, CardTextError};

use super::document_parser;
use super::ir::RewriteSemanticDocument;
use super::lower;

pub(crate) fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    document_parser::parse_text_to_semantic_document(builder, text, allow_unsupported)
}

pub(crate) fn lower_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    lower::lower_rewrite_document(doc)
}

pub(crate) fn parse_text_with_annotations_lowered(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let (doc, _) = parse_text_to_semantic_document(builder, text, allow_unsupported)?;
    lower_semantic_document(doc)
}

pub(crate) fn parse_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    parse_text_with_annotations_lowered(builder, text, allow_unsupported)
}
