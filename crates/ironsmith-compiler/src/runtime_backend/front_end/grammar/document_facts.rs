use super::super::ir::{DocumentSemanticFacts, OverloadRewritePayload};
use super::super::lexer::OwnedLexToken;
use super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverloadKeywordLine;

fn parse_overload_keyword_tokens(tokens: &[OwnedLexToken]) -> Option<OverloadKeywordLine> {
    primitives::parse_prefix(tokens, primitives::kw("overload"))?;
    Some(OverloadKeywordLine)
}

/// Builds document-wide facts while the front end still owns the lexed Oracle
/// text. The overload payload is consumed before preparation and lowering.
pub(crate) fn parse_document_semantic_facts<'a>(
    lines: impl IntoIterator<Item = (usize, &'a [OwnedLexToken])>,
) -> DocumentSemanticFacts {
    let mut overload_keyword_line_index = None;
    let mut overload_target_spans = Vec::new();

    for (index, tokens) in lines {
        if parse_overload_keyword_tokens(tokens).is_some() {
            overload_keyword_line_index.get_or_insert(index);
        } else {
            overload_target_spans.extend(
                tokens
                    .iter()
                    .filter(|token| token.is_word("target"))
                    .map(|token| token.span),
            );
        }
    }

    DocumentSemanticFacts {
        overload_rewrite: overload_keyword_line_index.map(|keyword_line_index| {
            OverloadRewritePayload {
                keyword_line_index,
                target_spans: overload_target_spans,
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn builds_typed_overload_rewrite_payload() {
        let lines = "Return target creature to its owner's hand.\nOverload {1}{U}"
            .lines()
            .enumerate()
            .map(|(index, line)| lex_line(line.trim(), index).expect("document fact fixture"))
            .collect::<Vec<_>>();
        let facts = parse_document_semantic_facts(
            lines
                .iter()
                .enumerate()
                .map(|(index, tokens)| (index, tokens.as_slice())),
        );
        let payload = facts
            .overload_rewrite
            .expect("overload should request a rewrite");
        assert_eq!(payload.keyword_line_index, 1);
        assert_eq!(payload.target_spans.len(), 1);
        assert_eq!(payload.target_spans[0].line, 0);
    }
}
