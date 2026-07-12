use crate::runtime_backend::lexer::{OwnedLexToken, split_lexed_sentences};
use crate::runtime_backend::token_primitives::{
    lexed_tokens_contain_non_prefix_instead, remove_copy_exception_type_removal_lexed,
    rewrite_followup_intro_to_if_lexed,
};
use crate::runtime_backend::util::join_sentences_with_period;

use super::document_shapes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatementGroupBoundaryKind {
    NonPrefixInstead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatementGroupBoundary {
    pub(crate) sentence_index: usize,
    pub(crate) kind: StatementGroupBoundaryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatementSentencesShape {
    pub(crate) sentences: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatementGroupingShape {
    pub(crate) sentences: Vec<Vec<OwnedLexToken>>,
    pub(crate) groups: Vec<Vec<OwnedLexToken>>,
    pub(crate) boundary: Option<StatementGroupBoundary>,
}

pub(crate) fn parse_statement_sentences_tokens(
    tokens: &[OwnedLexToken],
) -> StatementSentencesShape {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence_tokens| !sentence_tokens.is_empty())
        .map(|sentence_tokens| {
            document_shapes::parse_statement_label_strip_tokens(sentence_tokens).body_tokens
        })
        .map(rewrite_followup_intro_to_if_lexed)
        .map(|sentence| remove_copy_exception_type_removal_lexed(&sentence))
        .filter(|sentence| !sentence.is_empty())
        .collect();
    StatementSentencesShape { sentences }
}

pub(crate) fn parse_statement_group_boundary(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<StatementGroupBoundary> {
    sentences
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(sentence_index, sentence)| {
            lexed_tokens_contain_non_prefix_instead(sentence).then_some(StatementGroupBoundary {
                sentence_index,
                kind: StatementGroupBoundaryKind::NonPrefixInstead,
            })
        })
}

pub(crate) fn parse_statement_grouping_tokens(tokens: &[OwnedLexToken]) -> StatementGroupingShape {
    let StatementSentencesShape { sentences } = parse_statement_sentences_tokens(tokens);
    let boundary = parse_statement_group_boundary(&sentences);
    let groups = match (sentences.as_slice(), boundary) {
        ([], _) => {
            let fallback = document_shapes::parse_statement_label_strip_tokens(tokens).body_tokens;
            let fallback = rewrite_followup_intro_to_if_lexed(fallback);
            let fallback = remove_copy_exception_type_removal_lexed(&fallback);
            (!fallback.is_empty())
                .then_some(fallback)
                .into_iter()
                .collect()
        }
        ([only], _) => vec![only.clone()],
        (_, Some(boundary)) => {
            let mut groups = Vec::with_capacity(2);
            if boundary.sentence_index > 0 {
                groups.push(join_sentences_with_period(
                    &sentences[..boundary.sentence_index],
                ));
            }
            if boundary.sentence_index < sentences.len() {
                groups.push(join_sentences_with_period(
                    &sentences[boundary.sentence_index..],
                ));
            }
            groups
        }
        _ => vec![join_sentences_with_period(&sentences)],
    };
    StatementGroupingShape {
        sentences,
        groups,
        boundary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::{lex_line, render_token_slice};

    #[test]
    fn grouping_returns_typed_label_and_instead_boundary_facts() {
        let tokens = lex_line(
            "Ability word — Draw a card. Destroy target creature instead if you paid life.",
            0,
        )
        .unwrap();
        let parsed = parse_statement_grouping_tokens(&tokens);
        assert_eq!(parsed.sentences.len(), 2);
        assert_eq!(
            parsed.boundary,
            Some(StatementGroupBoundary {
                sentence_index: 1,
                kind: StatementGroupBoundaryKind::NonPrefixInstead,
            })
        );
        assert_eq!(parsed.groups.len(), 2);
        assert!(render_token_slice(&parsed.sentences[0]).starts_with("Draw a card"));
    }

    #[test]
    fn grouping_does_not_strip_numeric_result_table_heads() {
        let tokens = lex_line("1—4 | Draw a card. 5—6 | Draw two cards.", 0).unwrap();
        let parsed = parse_statement_sentences_tokens(&tokens);
        assert!(render_token_slice(&parsed.sentences[0]).starts_with("1—4"));
    }
}
