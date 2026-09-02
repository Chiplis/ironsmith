use ironsmith_core::card::CardBuilder;

use crate::cards::builders::CardTextError;
use crate::model::ParsedCardAst;
use crate::parse_context::ParseContext;
use crate::parse_trace;

/// The canonical public-to-the-compiler parse entry. It stops at the
/// compiler-owned card AST and cannot allocate runtime abilities.
pub fn parse_card_ast_with_context(
    context: &mut ParseContext,
    card: CardBuilder,
    text: String,
) -> Result<ParsedCardAst, CardTextError> {
    let (document, _) =
        super::document_parser::parse_text_to_semantic_document_with_context(context, card, text)?;
    let _scope = parse_trace::scope("semantic parse");
    super::semantic_document::parse_semantic_document(document)
}
