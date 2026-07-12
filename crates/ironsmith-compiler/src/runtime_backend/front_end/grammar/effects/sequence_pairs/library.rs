use std::ops::Range;

use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::{ChoiceCount, LibraryBottomOrderAst};
use crate::runtime_backend::front_end::grammar::leaf;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};

use super::super::super::primitives;
use super::{
    contains_content_sequence, contains_sequence_phrase, contains_sequence_word,
    matches_complete_content_sequence, seek_sequence_phrase, sequence_any_phrase, sequence_phrase,
    starts_content_sequence, starts_sequence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LookExileFaceDownShape {
    Counted {
        look: Range<usize>,
        exile: Range<usize>,
        count: ChoiceCount,
        bottom_order: LibraryBottomOrderAst,
    },
    Single {
        look: Range<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookedCardDisposition {
    HandAndLibraryBottom,
    HandAndGraveyard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedCardIntoHandShape {
    pub(crate) filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RevealTopRemainder {
    Graveyard,
    LibraryBottom(LibraryBottomOrderAst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevealTopMatchingFollowupShape {
    pub(crate) filter: Range<usize>,
    pub(crate) chosen_type_reference: bool,
    pub(crate) remainder: RevealTopRemainder,
}

const COUNTED_FACE_DOWN_PREFIXES: &[&[&str]] = &[
    &["of", "them", "face", "down"],
    &["of", "those", "cards", "face", "down"],
    &["them", "face", "down"],
    &["those", "cards", "face", "down"],
];
const BOTTOMS_REST: &[&[&str]] = &[
    &["put", "rest", "on", "bottom"],
    &["put", "rest", "onto", "bottom"],
    &["put", "the", "rest", "on", "bottom"],
    &["put", "the", "rest", "onto", "bottom"],
];
const SINGLE_FACE_DOWN: &[&[&str]] = &[
    &["exile", "it", "face", "down"],
    &["exile", "that", "card", "face", "down"],
];

pub(crate) fn parse_bottom_order(tokens: &[OwnedLexToken]) -> Option<LibraryBottomOrderAst> {
    if !contains_sequence_word(tokens, "bottom") || !contains_sequence_word(tokens, "library") {
        return None;
    }
    if contains_sequence_phrase(tokens, &[&["random", "order"]]) {
        return Some(LibraryBottomOrderAst::Random);
    }
    if contains_sequence_phrase(tokens, &[&["any", "order"]]) {
        return Some(LibraryBottomOrderAst::ChooserChooses);
    }
    None
}

fn parse_counted_face_down_head<'a>(input: &mut LexStream<'a>) -> WResult<ChoiceCount> {
    sequence_phrase(&["exile"]).parse_next(input)?;
    leaf::parse_leaf_choice_count_prefix_lexed.parse_next(input)
}

fn counted_face_down_shape(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, LibraryBottomOrderAst)> {
    let (count, tail) = primitives::parse_prefix(tokens, parse_counted_face_down_head)?;
    if !starts_content_sequence(tail, COUNTED_FACE_DOWN_PREFIXES)
        || !contains_sequence_word(tail, "library")
        || !contains_content_sequence(tail, BOTTOMS_REST)
    {
        return None;
    }
    Some((count, parse_bottom_order(tail)?))
}

pub(crate) fn parse_look_exile_face_down_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookExileFaceDownShape> {
    let mut exile_input = LexStream::new(tokens);
    let exile_at = seek_sequence_phrase(&mut exile_input, &[&["exile"]]).ok()?;
    let exile = exile_at..tokens.len();
    if let Some((count, bottom_order)) = counted_face_down_shape(&tokens[exile.clone()]) {
        return Some(LookExileFaceDownShape::Counted {
            look: 0..exile_at,
            exile,
            count,
            bottom_order,
        });
    }

    let mut then_input = LexStream::new(tokens);
    let then_at = seek_sequence_phrase(&mut then_input, &[&["then", "exile"]]).ok()?;
    sequence_phrase(&["then"])
        .parse_next(&mut then_input)
        .ok()?;
    let exile_at = tokens.len().saturating_sub(then_input.len());
    if !matches_complete_content_sequence(&tokens[exile_at..], SINGLE_FACE_DOWN) {
        return None;
    }
    Some(LookExileFaceDownShape::Single { look: 0..then_at })
}

const PUT_ONE_HAND: &[&[&str]] = &[
    &["put", "one", "of", "them", "into", "your", "hand"],
    &["put", "one", "of", "those", "cards", "into", "your", "hand"],
    &["put", "one", "into", "your", "hand"],
];
const OTHER_BOTTOM: &[&[&str]] = &[
    &["other", "on", "bottom"],
    &["other", "onto", "bottom"],
    &["rest", "on", "bottom"],
    &["rest", "onto", "bottom"],
];
const OTHER_GRAVEYARD: &[&[&str]] = &[
    &["other", "into", "graveyard"],
    &["other", "into", "your", "graveyard"],
    &["rest", "into", "your", "graveyard"],
    &["rest", "into", "graveyard"],
];

pub(crate) fn parse_looked_card_disposition(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardDisposition> {
    if !starts_sequence(tokens, PUT_ONE_HAND) {
        return None;
    }
    if contains_content_sequence(tokens, OTHER_BOTTOM) && contains_sequence_word(tokens, "library")
    {
        return Some(LookedCardDisposition::HandAndLibraryBottom);
    }
    if contains_content_sequence(tokens, OTHER_GRAVEYARD) {
        return Some(LookedCardDisposition::HandAndGraveyard);
    }
    None
}

pub(crate) fn is_keyword_bundle_choice_filter(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    let mut segments = 0usize;
    while let Ok(token) = super::next_word(&mut input) {
        if !matches!(token.parser_text(), "a" | "an" | "the") {
            continue;
        }
        let mut probe = input.clone();
        let Ok(card) = super::next_word(&mut probe) else {
            continue;
        };
        if !matches!(card.parser_text(), "card" | "cards") {
            continue;
        }
        let Ok(with) = super::next_word(&mut probe) else {
            continue;
        };
        if !with.is_word("with") {
            continue;
        }
        segments += 1;
        if segments >= 2 {
            return true;
        }
    }
    false
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "milled", "this", "way"],
    &["from", "among", "the", "milled", "cards"],
    &["from", "among", "them"],
];

pub(crate) fn parse_looked_card_into_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardIntoHandShape> {
    let mut input = LexStream::new(tokens);
    let filter_end = seek_sequence_phrase(&mut input, FROM_AMONG).ok()?;
    if filter_end == 0 || is_keyword_bundle_choice_filter(&tokens[..filter_end]) {
        return None;
    }
    sequence_any_phrase(FROM_AMONG)
        .parse_next(&mut input)
        .ok()?;
    let tail_start = tokens.len().saturating_sub(input.len());
    let tail = &tokens[tail_start..];
    if !starts_sequence(tail, &[&["into"]]) || !contains_sequence_word(tail, "hand") {
        return None;
    }
    Some(LookedCardIntoHandShape {
        filter: 0..filter_end,
    })
}

pub(crate) fn has_rest_on_library_bottom_surface(tokens: &[OwnedLexToken]) -> bool {
    contains_sequence_word(tokens, "rest")
        && contains_sequence_word(tokens, "bottom")
        && contains_sequence_word(tokens, "library")
}

const PUT_ALL: &[&[&str]] = &[&["put", "all"], &["puts", "all"]];
const CHOSEN_TYPE: &[&[&str]] = &[&["chosen", "type"], &["that", "type"]];
const REST_GRAVEYARD: &[&[&str]] = &[&["and", "rest", "into", "your"]];

pub(crate) fn parse_reveal_top_matching_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealTopMatchingFollowupShape> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    sequence_any_phrase(PUT_ALL).parse_next(&mut input).ok()?;
    let filter_start = initial_len.saturating_sub(input.len());
    seek_sequence_phrase(&mut input, &[&["revealed", "this", "way"]]).ok()?;
    let filter_end = initial_len.saturating_sub(input.len());
    if filter_start >= filter_end {
        return None;
    }
    let filter = &tokens[filter_start..filter_end];
    if is_keyword_bundle_choice_filter(filter) {
        return None;
    }
    sequence_phrase(&["revealed", "this", "way"])
        .parse_next(&mut input)
        .ok()?;
    let tail_start = initial_len.saturating_sub(input.len());
    let tail = &tokens[tail_start..];
    if !contains_sequence_phrase(tail, &[&["into", "your", "hand"]]) {
        return None;
    }
    let bottom_order = parse_bottom_order(tail);
    let graveyard = contains_content_sequence(tail, REST_GRAVEYARD)
        && contains_sequence_word(tail, "graveyard");
    let remainder = if let Some(order) = bottom_order {
        RevealTopRemainder::LibraryBottom(order)
    } else if graveyard {
        RevealTopRemainder::Graveyard
    } else {
        return None;
    };
    Some(RevealTopMatchingFollowupShape {
        filter: filter_start..filter_end,
        chosen_type_reference: contains_sequence_phrase(filter, CHOSEN_TYPE),
        remainder,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_looked_card_dispositions_and_from_among_filter() {
        assert_eq!(
            parse_looked_card_disposition(&lex(
                "Put one of them into your hand and the rest on the bottom of your library in any order"
            )),
            Some(LookedCardDisposition::HandAndLibraryBottom)
        );
        let shape = parse_looked_card_into_hand_shape(&lex(
            "a creature card from among those cards into your hand",
        ))
        .unwrap();
        assert!(!shape.filter.is_empty());

        assert!(matches!(
            parse_reveal_top_matching_followup_shape(&lex(
                "Put all land cards revealed this way into your hand and the rest into your graveyard"
            )),
            Some(RevealTopMatchingFollowupShape {
                remainder: RevealTopRemainder::Graveyard,
                ..
            })
        ));
    }
}
