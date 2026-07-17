use std::ops::Range;

use winnow::combinator::opt;
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, TokenKind};
use crate::zone::Zone;

use super::library::parse_bottom_order;
use super::{
    contains_sequence_phrase, contains_sequence_word, finish_sequence_words,
    matches_complete_content_sequence, seek_sequence_phrase, sequence_any_phrase, sequence_phrase,
    starts_sequence,
};

#[path = "consult/cast.rs"]
mod cast;
pub(crate) use cast::*;
#[path = "consult/remainder.rs"]
mod remainder;
pub(crate) use remainder::*;
#[path = "consult/traversal.rs"]
mod traversal;
pub(crate) use traversal::*;
#[path = "consult/values.rs"]
mod values;
pub(crate) use values::*;
#[path = "consult/dispositions.rs"]
mod dispositions;
pub(crate) use dispositions::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsultMoveBottomShape {
    MatchedToBattlefieldAndShuffle,
    MoveMatchAndBottom {
        zone: Zone,
        battlefield_tapped: bool,
        order: LibraryBottomOrderAst,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionalConsultShape {
    pub(crate) predicate: Range<usize>,
    pub(crate) effect: Range<usize>,
    pub(crate) if_result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsultBattlefieldGraveyardShape {
    Combined,
    RemainderThenMatch { controller_you: bool },
}

const HAND_PREFIXES: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
];
const BATTLEFIELD_TAPPED_PREFIXES: &[&[&str]] = &[
    &[
        "put",
        "that",
        "card",
        "onto",
        "the",
        "battlefield",
        "tapped",
    ],
    &["put", "it", "onto", "the", "battlefield", "tapped"],
    &["put", "that", "card", "onto", "battlefield", "tapped"],
    &["put", "it", "onto", "battlefield", "tapped"],
    &[
        "put",
        "those",
        "land",
        "cards",
        "onto",
        "the",
        "battlefield",
        "tapped",
    ],
    &[
        "put",
        "those",
        "lands",
        "onto",
        "the",
        "battlefield",
        "tapped",
    ],
];
const BATTLEFIELD_PREFIXES: &[&[&str]] = &[
    &["put", "that", "card", "onto", "the", "battlefield"],
    &["put", "it", "onto", "the", "battlefield"],
    &["put", "that", "card", "onto", "battlefield"],
    &["put", "it", "onto", "battlefield"],
    &[
        "put",
        "those",
        "land",
        "cards",
        "onto",
        "the",
        "battlefield",
    ],
    &["put", "those", "lands", "onto", "the", "battlefield"],
];

fn put_those_cards_then_shuffle_revealed_remainder(
    input: &mut LexStream<'_>,
) -> winnow::error::ModalResult<()> {
    sequence_any_phrase(&[
        &["put", "those", "cards", "onto", "the", "battlefield"],
        &["put", "those", "cards", "onto", "battlefield"],
    ])
    .parse_next(input)?;
    opt(crate::runtime_backend::grammar::primitives::comma()).parse_next(input)?;
    sequence_any_phrase(&[
        &[
            "then", "shuffle", "the", "rest", "of", "the", "revealed", "cards", "into", "your",
            "library",
        ],
        &[
            "then", "shuffle", "rest", "of", "revealed", "cards", "into", "your", "library",
        ],
    ])
    .parse_next(input)?;
    finish_sequence_words(input)
}

fn is_put_those_cards_then_shuffle_revealed_remainder(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    put_those_cards_then_shuffle_revealed_remainder
        .parse_next(&mut input)
        .is_ok()
}

pub(crate) fn parse_consult_move_bottom_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConsultMoveBottomShape> {
    let special = is_put_those_cards_then_shuffle_revealed_remainder(tokens)
        || (starts_sequence(tokens, &[&["put", "all"]])
            && contains_sequence_phrase(tokens, &[&["cards", "revealed", "this", "way"]])
            && (contains_sequence_phrase(tokens, &[&["onto", "the", "battlefield"]])
                || contains_sequence_phrase(tokens, &[&["onto", "battlefield"]]))
            && contains_sequence_word(tokens, "shuffle")
            && contains_sequence_word(tokens, "rest")
            && contains_sequence_word(tokens, "library"));
    if special {
        return Some(ConsultMoveBottomShape::MatchedToBattlefieldAndShuffle);
    }

    let (zone, battlefield_tapped) = if starts_sequence(tokens, HAND_PREFIXES) {
        (Zone::Hand, false)
    } else if starts_sequence(tokens, BATTLEFIELD_TAPPED_PREFIXES) {
        (Zone::Battlefield, true)
    } else if starts_sequence(tokens, BATTLEFIELD_PREFIXES) {
        (Zone::Battlefield, false)
    } else {
        return None;
    };
    if !contains_sequence_word(tokens, "rest") && !contains_sequence_word(tokens, "other") {
        return None;
    }
    Some(ConsultMoveBottomShape::MoveMatchAndBottom {
        zone,
        battlefield_tapped,
        order: parse_bottom_order(tokens)?,
    })
}

pub(crate) fn parse_conditional_consult_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalConsultShape> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    if sequence_phrase(&["then", "if"])
        .parse_next(&mut input)
        .is_err()
    {
        input = LexStream::new(tokens);
        sequence_phrase(&["if"]).parse_next(&mut input).ok()?;
    }
    let predicate_start = initial_len.saturating_sub(input.len());
    let mut comma_at = None;
    while !input.is_empty() {
        let offset = initial_len.saturating_sub(input.len());
        let parsed: winnow::error::ModalResult<&OwnedLexToken> =
            winnow::token::any.parse_next(&mut input);
        let token = parsed.ok()?;
        if token.kind == TokenKind::Comma {
            comma_at = Some(offset);
            break;
        }
    }
    let comma_at = comma_at?;
    let effect_start = initial_len.saturating_sub(input.len());
    if predicate_start >= comma_at || effect_start >= tokens.len() {
        return None;
    }
    let predicate = predicate_start..comma_at;
    Some(ConditionalConsultShape {
        if_result: matches_complete_content_sequence(&tokens[predicate.clone()], &[&["you", "do"]]),
        predicate,
        effect: effect_start..tokens.len(),
    })
}

pub(crate) fn is_consult_move_all_to_graveyard_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[
            &["put", "all"],
            &["puts", "all"],
            &["that", "player", "puts", "all"],
        ],
    ) && contains_sequence_phrase(tokens, &[&["revealed", "this", "way"]])
        && contains_sequence_word(tokens, "graveyard")
}

pub(crate) fn is_consult_hand_others_graveyard_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, HAND_PREFIXES)
        && (contains_sequence_phrase(tokens, &[&["other", "cards"]])
            || contains_sequence_phrase(tokens, &[&["all", "other"]])
            || contains_sequence_word(tokens, "rest"))
        && contains_sequence_word(tokens, "graveyard")
}

const MATCH_BATTLEFIELD_PREFIXES: &[&[&str]] = &[
    &["put", "that", "card", "onto", "the", "battlefield"],
    &["put", "it", "onto", "the", "battlefield"],
    &[
        "you",
        "put",
        "the",
        "creature",
        "card",
        "onto",
        "the",
        "battlefield",
    ],
    &[
        "the",
        "player",
        "puts",
        "that",
        "card",
        "onto",
        "the",
        "battlefield",
    ],
    &[
        "that",
        "player",
        "puts",
        "that",
        "card",
        "onto",
        "the",
        "battlefield",
    ],
];

fn remainder_to_graveyard(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[
            &["put", "all"],
            &["puts", "all"],
            &["that", "player", "puts", "all"],
        ],
    ) && (contains_sequence_phrase(
        tokens,
        &[
            &["noncreature", "cards", "revealed", "this", "way"],
            &["all", "noncreature", "cards", "revealed", "this", "way"],
        ],
    )) && contains_sequence_word(tokens, "graveyard")
}

fn matched_to_battlefield(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, MATCH_BATTLEFIELD_PREFIXES)
}

pub(crate) fn parse_consult_battlefield_graveyard_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConsultBattlefieldGraveyardShape> {
    let mut input = LexStream::new(tokens);
    if let Ok(then_at) = seek_sequence_phrase(&mut input, &[&["then"]]) {
        sequence_any_phrase(&[&["then"]])
            .parse_next(&mut input)
            .ok()?;
        let after_then = tokens.len().saturating_sub(input.len());
        let remainder = &tokens[..then_at];
        let matched = &tokens[after_then..];
        if remainder_to_graveyard(remainder) && matched_to_battlefield(matched) {
            return Some(ConsultBattlefieldGraveyardShape::RemainderThenMatch {
                controller_you: starts_sequence(matched, &[&["you", "put"]])
                    || contains_sequence_phrase(matched, &[&["under", "your", "control"]]),
            });
        }
    }
    if matched_to_battlefield(tokens)
        && (contains_sequence_phrase(tokens, &[&["other", "cards"]])
            || contains_sequence_phrase(tokens, &[&["all", "other"]]))
        && contains_sequence_word(tokens, "graveyard")
    {
        Some(ConsultBattlefieldGraveyardShape::Combined)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn classifies_consult_move_and_conditional_surfaces() {
        assert!(matches!(
            parse_consult_move_bottom_shape(&lex(
                "Put that card into your hand and the rest on the bottom of your library in any order"
            )),
            Some(ConsultMoveBottomShape::MoveMatchAndBottom {
                zone: Zone::Hand,
                ..
            })
        ));
        let conditional =
            parse_conditional_consult_shape(&lex("Then if you do, put that card into your hand"))
                .unwrap();
        assert!(conditional.if_result);

        assert_eq!(
            parse_consult_move_bottom_shape(&lex(
                "Put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library"
            )),
            Some(ConsultMoveBottomShape::MatchedToBattlefieldAndShuffle)
        );
    }
}
