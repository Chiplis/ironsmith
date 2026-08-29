use crate::cards::builders::CardTextError;

use super::super::grammar::document_shapes;
use super::super::lexer::OwnedLexToken;

pub(super) fn diagnose_known_unsupported_rewrite_line(
    tokens: &[OwnedLexToken],
) -> Option<CardTextError> {
    let kind = document_shapes::parse_unsupported_rewrite_line_kind(tokens)?;
    Some(CardTextError::ParseError(kind.diagnostic().to_string()))
}
