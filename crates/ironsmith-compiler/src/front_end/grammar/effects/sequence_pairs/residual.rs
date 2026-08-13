use std::ops::Range;

use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::front_end::lexer::{LexStream, OwnedLexToken};

use super::super::super::primitives;
use super::{
    contains_sequence_phrase, finish_sequence_words, sequence_any_phrase, sequence_phrase,
    starts_sequence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestActionShape {
    Destroy,
    Exile,
    Sacrifice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionalSequencePrefixShape {
    pub(crate) tail: Range<usize>,
}

fn rest_action(input: &mut LexStream<'_>) -> WResult<RestActionShape> {
    opt(sequence_phrase(&["then"])).parse_next(input)?;
    let action = alt((
        sequence_any_phrase(&[&["destroy", "the", "rest"], &["destroy", "rest"]])
            .value(RestActionShape::Destroy),
        sequence_any_phrase(&[&["exile", "the", "rest"], &["exile", "rest"]])
            .value(RestActionShape::Exile),
        sequence_any_phrase(&[
            &["sacrifice", "the", "rest"],
            &["sacrifice", "rest"],
            &["sacrifices", "the", "rest"],
            &["sacrifices", "rest"],
        ])
        .value(RestActionShape::Sacrifice),
    ))
    .parse_next(input)?;
    finish_sequence_words(input)?;
    Ok(action)
}

pub(crate) fn parse_rest_action_shape(tokens: &[OwnedLexToken]) -> Option<RestActionShape> {
    primitives::parse_all(tokens, rest_action, "sequence-rest-action").ok()
}

fn optional_may_prefix(input: &mut LexStream<'_>) -> WResult<()> {
    sequence_any_phrase(&[
        &["you", "may"],
        &["that", "player", "may"],
        &["they", "may"],
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    Ok(())
}

pub(crate) fn parse_optional_sequence_prefix_shape(
    tokens: &[OwnedLexToken],
) -> Option<OptionalSequencePrefixShape> {
    let (_, tail) = primitives::parse_prefix(tokens, optional_may_prefix)?;
    let start = tokens.len().saturating_sub(tail.len());
    (start < tokens.len()).then_some(OptionalSequencePrefixShape {
        tail: start..tokens.len(),
    })
}

pub(crate) fn is_consult_hand_then_exile_others_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[
            &["put", "that", "card", "into", "your", "hand"],
            &["put", "it", "into", "your", "hand"],
        ],
    ) && contains_sequence_phrase(tokens, &[&["exile"]])
        && contains_sequence_phrase(tokens, &[&["other"]])
        && contains_sequence_phrase(tokens, &[&["cards"]])
}

pub(crate) fn is_consult_battlefield_or_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[
            &[
                "put",
                "that",
                "card",
                "onto",
                "the",
                "battlefield",
                "or",
                "into",
                "your",
                "hand",
            ],
            &[
                "put",
                "it",
                "onto",
                "the",
                "battlefield",
                "or",
                "into",
                "your",
                "hand",
            ],
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_residual_pair_shapes() {
        assert_eq!(
            parse_rest_action_shape(&lex("Then sacrifice the rest.")),
            Some(RestActionShape::Sacrifice)
        );
        let optional = parse_optional_sequence_prefix_shape(&lex(
            "That player may reveal cards until they reveal a creature card.",
        ))
        .unwrap();
        assert!(!optional.tail.is_empty());
        assert!(is_consult_hand_then_exile_others_shape(&lex(
            "Put that card into your hand and exile all other cards revealed this way."
        )));
    }
}
