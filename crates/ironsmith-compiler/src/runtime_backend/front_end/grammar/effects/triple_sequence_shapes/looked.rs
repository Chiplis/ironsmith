use std::ops::Range;

use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};
use crate::runtime_backend::grammar::{leaf, primitives};

use super::super::sequence_pairs::{
    contains_sequence_phrase, contains_sequence_word, finish_sequence_words, seek_sequence_phrase,
    sequence_any_phrase, sequence_phrase, starts_sequence,
};
use super::parse_consult_remainder_order_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookedMoveDestinationShape {
    Hand,
    Battlefield {
        tapped: bool,
        attacking: bool,
        attacks_that_player: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedMoveActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
    pub(crate) destination: LookedMoveDestinationShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedHandActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
    pub(crate) filter_uses_and_or: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedTopActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedCastActionShape {
    pub(crate) filter: Range<usize>,
    pub(crate) mentions_spell: bool,
    pub(crate) mana_value_limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookedRemainderShape {
    Graveyard,
    LibraryBottom(LibraryBottomOrderAst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnyNumberRevealedChoiceShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevealOneGainManaValueShape {
    pub(crate) view: Range<usize>,
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "revealed", "this", "way"],
    &["from", "among", "the", "cards", "milled", "this", "way"],
    &["from", "among", "the", "milled", "cards"],
    &["from", "among", "them"],
];
const INTO_HAND: &[&[&str]] = &[&["into", "your", "hand"], &["into", "hand"]];
const BATTLEFIELD_TAPPED: &[&[&str]] = &[
    &["onto", "the", "battlefield", "tapped"],
    &["onto", "battlefield", "tapped"],
];
const BATTLEFIELD: &[&[&str]] = &[&["onto", "the", "battlefield"], &["onto", "battlefield"]];
const PUT_ONE_INTO_HAND: &[&[&str]] = &[
    &["put", "one", "of", "them", "into", "your", "hand"],
    &["put", "one", "of", "those", "cards", "into", "your", "hand"],
    &["put", "one", "into", "your", "hand"],
];

fn split_from_among(tokens: &[OwnedLexToken]) -> Option<(Range<usize>, &[OwnedLexToken])> {
    let mut input = LexStream::new(tokens);
    let head_end = seek_sequence_phrase(&mut input, FROM_AMONG).ok()?;
    sequence_any_phrase(FROM_AMONG)
        .parse_next(&mut input)
        .ok()?;
    let tail_start = tokens.len().saturating_sub(input.len());
    (head_end > 0).then_some((0..head_end, &tokens[tail_start..]))
}

fn counted_filter_range(
    tokens: &[OwnedLexToken],
    head: Range<usize>,
) -> (ChoiceCount, Range<usize>) {
    let head_tokens = &tokens[head.clone()];
    if let Some((count, rest)) =
        primitives::parse_prefix(head_tokens, leaf::parse_leaf_choice_count_prefix_lexed)
    {
        let start = head.end.saturating_sub(rest.len());
        (count, start..head.end)
    } else {
        (ChoiceCount::up_to(1), head)
    }
}

pub(crate) fn parse_looked_move_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedMoveActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty() {
        return None;
    }
    let destination = if starts_sequence(tail, INTO_HAND) {
        LookedMoveDestinationShape::Hand
    } else {
        let tapped = starts_sequence(tail, BATTLEFIELD_TAPPED);
        if !tapped && !starts_sequence(tail, BATTLEFIELD) {
            return None;
        }
        let attacking = contains_sequence_word(tail, "attacking");
        LookedMoveDestinationShape::Battlefield {
            tapped,
            attacking,
            attacks_that_player: attacking
                && contains_sequence_phrase(tail, &[&["attacking", "that", "player"]]),
        }
    };
    Some(LookedMoveActionShape {
        count,
        filter,
        destination,
    })
}

const REVEAL_TO_HAND: &[&[&str]] = &[
    &["and", "put", "it", "into"],
    &["put", "it", "into"],
    &["and", "put", "them", "into"],
    &["put", "them", "into"],
    &["and", "put", "that", "card", "into"],
    &["put", "that", "card", "into"],
    &["and", "put", "the", "revealed"],
    &["and", "put", "those", "cards"],
    &["and", "put", "them"],
];
const REVEAL_TO_TOP: &[&[&str]] = &[
    &["and", "put", "it", "on", "top"],
    &["put", "it", "on", "top"],
    &["and", "put", "that", "card", "on", "top"],
    &["put", "that", "card", "on", "top"],
];

pub(crate) fn parse_looked_hand_action_shape(
    tokens: &[OwnedLexToken],
    reveal_chosen: bool,
) -> Option<LookedHandActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty() {
        return None;
    }
    let valid_tail = if reveal_chosen {
        starts_sequence(tail, REVEAL_TO_HAND) && contains_sequence_word(tail, "hand")
    } else {
        starts_sequence(tail, &[&["into"]]) && contains_sequence_word(tail, "hand")
    };
    valid_tail.then_some(LookedHandActionShape {
        count,
        filter: filter.clone(),
        filter_uses_and_or: contains_sequence_word(&tokens[filter], "and/or"),
    })
}

pub(crate) fn parse_looked_top_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedTopActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty()
        || !starts_sequence(tail, REVEAL_TO_TOP)
        || !contains_sequence_word(tail, "library")
    {
        return None;
    }
    Some(LookedTopActionShape { count, filter })
}

pub(crate) fn parse_looked_cast_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCastActionShape> {
    let (filter, tail) = split_from_among(tokens)?;
    if !starts_sequence(tail, &[&["without", "paying", "its", "mana", "cost"]]) {
        return None;
    }
    let filter_tokens = &tokens[filter.clone()];
    let mentions_spell = contains_sequence_word(filter_tokens, "spell")
        || contains_sequence_word(filter_tokens, "spells");
    let mana_value_limit = parse_mana_value_limit(filter_tokens);
    Some(LookedCastActionShape {
        filter,
        mentions_spell,
        mana_value_limit,
    })
}

fn parse_mana_value_limit(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut input = LexStream::new(tokens);
    seek_sequence_phrase(&mut input, &[&["mana", "value"]]).ok()?;
    sequence_phrase(&["mana", "value"])
        .parse_next(&mut input)
        .ok()?;
    let value = leaf::parse_leaf_number_prefix_lexed
        .parse_next(&mut input)
        .ok()?;
    sequence_phrase(&["or", "less"])
        .parse_next(&mut input)
        .ok()?;
    Some(value)
}

pub(crate) fn parse_looked_remainder_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedRemainderShape> {
    let tail = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        sequence_phrase(&["then"]).parse_next(input)
    })
    .map(|(_, tail)| tail)
    .unwrap_or(tokens);
    if !starts_sequence(tail, &[&["put"], &["puts"]]) || !contains_sequence_word(tail, "rest") {
        return None;
    }
    if contains_sequence_word(tail, "bottom") && contains_sequence_word(tail, "library") {
        return parse_consult_remainder_order_tokens(tail).map(LookedRemainderShape::LibraryBottom);
    }
    contains_sequence_word(tail, "graveyard").then_some(LookedRemainderShape::Graveyard)
}

pub(crate) fn parse_any_number_revealed_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<AnyNumberRevealedChoiceShape> {
    let (_, tail) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        sequence_phrase(&["choose"]).parse_next(input)
    })?;
    let (count, after_count) =
        primitives::parse_prefix(tail, leaf::parse_leaf_choice_count_prefix_lexed)?;
    if count != ChoiceCount::any_number() {
        return None;
    }
    let mut input = LexStream::new(after_count);
    let filter_end = seek_sequence_phrase(&mut input, &[&["revealed", "this", "way"]]).ok()?;
    sequence_phrase(&["revealed", "this", "way"])
        .parse_next(&mut input)
        .ok()?;
    finish_sequence_words(&mut input).ok()?;
    let filter_start =
        tokens.len().saturating_sub(tail.len()) + tail.len().saturating_sub(after_count.len());
    (filter_end > 0).then_some(AnyNumberRevealedChoiceShape {
        count,
        filter: filter_start..filter_start + filter_end,
    })
}

pub(crate) fn is_land_nonland_split_bottom_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, &[&["put"]])
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "nonland",
                "cards",
                "chosen",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
            ]],
        )
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "land",
                "cards",
                "chosen",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
                "tapped",
            ]],
        )
        && parse_looked_remainder_shape(tokens)
            == Some(LookedRemainderShape::LibraryBottom(
                LibraryBottomOrderAst::Random,
            ))
}

pub(crate) fn parse_reveal_one_gain_mana_value_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<RevealOneGainManaValueShape> {
    let mut input = LexStream::new(first);
    let view_end = seek_sequence_phrase(&mut input, &[&["and", "put"]]).ok()?;
    sequence_phrase(&["and"]).parse_next(&mut input).ok()?;
    let tail_start = first.len().saturating_sub(input.len());
    if !starts_sequence(&first[tail_start..], PUT_ONE_INTO_HAND)
        || !starts_sequence(second, &[&["you", "gain", "life"]])
        || !contains_sequence_phrase(second, &[&["mana", "value"]])
        || (!contains_sequence_word(second, "card")
            && !contains_sequence_word(second, "cards")
            && !contains_sequence_word(second, "card's"))
        || !starts_sequence(third, &[&["put"], &["puts"]])
        || !contains_sequence_word(third, "other")
        || !contains_sequence_word(third, "revealed")
        || !contains_sequence_word(third, "graveyard")
    {
        return None;
    }
    Some(RevealOneGainManaValueShape { view: 0..view_end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, split_lexed_sentences};

    #[test]
    fn parses_reveal_one_gain_mana_value_shape() {
        let tokens = lex_line(
            "Reveal the top three cards of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(
            parse_reveal_one_gain_mana_value_shape(sentences[0], sentences[1], sentences[2])
                .is_some()
        );
    }
}
