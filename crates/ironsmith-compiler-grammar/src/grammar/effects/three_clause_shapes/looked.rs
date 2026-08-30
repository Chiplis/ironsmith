use std::ops::Range;

use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken};
use crate::object::CounterType;

use super::super::control_copy_attach_shapes::{
    BattlefieldControllerShape, parse_battlefield_controller_prefix,
};
use super::super::sequence_pairs::{
    contains_sequence_phrase, contains_sequence_word, finish_sequence_words, seek_sequence_phrase,
    sequence_any_phrase, sequence_phrase, starts_sequence,
};
use super::parse_consult_remainder_order_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookedMoveDestinationShape {
    Hand,
    Battlefield {
        tapped: bool,
        attacking: bool,
        attacks_that_player: bool,
        controller: Option<BattlefieldControllerShape>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedMoveActionShape {
    pub count: ChoiceCount,
    pub filter: Range<usize>,
    pub destination: LookedMoveDestinationShape,
    pub all_matching: bool,
    pub entry_counter: Option<(u32, CounterType)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedHandActionShape {
    pub count: ChoiceCount,
    pub filter: Range<usize>,
    pub filter_uses_and_or: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedTopActionShape {
    pub count: ChoiceCount,
    pub filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedTopAndRemainderActionShape {
    pub count: ChoiceCount,
    pub filter: Range<usize>,
    pub remainder_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedCastActionShape {
    pub filter: Range<usize>,
    pub mentions_spell: bool,
    pub mana_value_limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookedRemainderShape {
    Graveyard,
    LibraryBottom(LibraryBottomOrderAst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnyNumberRevealedChoiceShape {
    pub count: ChoiceCount,
    pub filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealOneGainManaValueShape {
    pub view: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedRevealSelectionShape {
    pub count: ChoiceCount,
    pub filter: Range<usize>,
    pub remainder_order: LibraryBottomOrderAst,
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "revealed", "this", "way"],
    &["from", "among", "the", "cards", "milled", "this", "way"],
    &["from", "among", "the", "milled", "cards"],
    &["from", "among", "them"],
];
const INTO_HAND: &[&[&str]] = &[
    &["into", "your", "hand"],
    &["into", "hand"],
    &["to", "your", "hand"],
    &["to", "hand"],
];
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

fn parse_battlefield_entry_counter(tail: &[OwnedLexToken]) -> Option<(u32, CounterType)> {
    let (_, (), after_with) = primitives::find_prefix(tail, || primitives::kw("with").void())?;
    let (on_idx, (), after_on) =
        primitives::find_prefix(after_with, || primitives::phrase(&["on", "it"]).void())?;
    if after_on.iter().any(|token| token.as_word().is_some()) {
        return None;
    }
    let descriptor =
        super::super::zone_counter_shapes::parse_counter_descriptor_shape(&after_with[..on_idx])?;
    Some((descriptor.count, descriptor.counter_type))
}

pub fn parse_looked_move_action_shape(tokens: &[OwnedLexToken]) -> Option<LookedMoveActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let all_matching = tokens
        .get(head.clone())?
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "all");
    let (count, filter) = if all_matching {
        (
            ChoiceCount::any_number(),
            head.start.saturating_add(1)..head.end,
        )
    } else {
        counted_filter_range(tokens, head)
    };
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
        let controller = tail.iter().enumerate().find_map(|(index, token)| {
            token
                .is_word("under")
                .then(|| parse_battlefield_controller_prefix(&tail[index..]))
                .flatten()
                .map(|shape| shape.controller)
        });
        LookedMoveDestinationShape::Battlefield {
            tapped,
            attacking,
            attacks_that_player: attacking
                && contains_sequence_phrase(tail, &[&["attacking", "that", "player"]]),
            controller,
        }
    };
    Some(LookedMoveActionShape {
        count,
        filter,
        destination,
        all_matching,
        entry_counter: parse_battlefield_entry_counter(tail),
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
    &["and", "put", "them", "on", "top"],
    &["put", "them", "on", "top"],
    &["and", "put", "those", "cards", "on", "top"],
    &["put", "those", "cards", "on", "top"],
];

pub fn parse_looked_hand_action_shape(
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

pub fn parse_looked_top_action_shape(tokens: &[OwnedLexToken]) -> Option<LookedTopActionShape> {
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

/// Parses the single-sentence follow-up used after an optional look, such as
/// "reveal up to one land card from among them, then put that card on top ...
/// and the rest on the bottom ...". The selected subset and the remainder
/// stay tied to the same looked-at collection.
pub fn parse_looked_top_and_remainder_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedTopAndRemainderActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    let top_tail = primitives::parse_prefix(tail, |input: &mut LexStream<'_>| {
        sequence_phrase(&["then"]).parse_next(input)
    })
    .map(|(_, rest)| rest)
    .unwrap_or(tail);
    if filter.is_empty()
        || !starts_sequence(top_tail, REVEAL_TO_TOP)
        || !contains_sequence_word(top_tail, "rest")
        || !contains_sequence_word(top_tail, "bottom")
        || !contains_sequence_word(top_tail, "library")
    {
        return None;
    }
    Some(LookedTopAndRemainderActionShape {
        count,
        filter,
        remainder_order: parse_consult_remainder_order_tokens(top_tail)?,
    })
}

pub fn parse_looked_cast_action_shape(tokens: &[OwnedLexToken]) -> Option<LookedCastActionShape> {
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

/// Parses a reveal selection whose source and remainder both refer to the
/// preceding looked-at collection, for example "up to two creature and/or
/// land cards from among them, then put the rest on the bottom ...".
pub fn parse_looked_reveal_selection_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedRevealSelectionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty()
        || !contains_sequence_phrase(tail, &[&["put", "the", "rest"], &["put", "rest"]])
    {
        return None;
    }
    Some(LookedRevealSelectionShape {
        count,
        filter,
        remainder_order: parse_consult_remainder_order_tokens(tail)?,
    })
}

#[cfg(test)]
#[path = "looked_inline_tests.rs"]
mod tests;

#[path = "looked/library.rs"]
mod library_programs;
pub use library_programs::{
    is_explicit_revealed_cards_not_put_onto_battlefield_complement,
    is_land_nonland_split_bottom_shape, is_looked_same_name_permanent_battlefield_action,
    is_revealed_land_creature_split_shape, looked_remainder_surface,
    parse_any_number_revealed_choice_shape, parse_looked_remainder_shape,
    parse_reveal_one_gain_mana_value_shape,
};
#[path = "looked/resource.rs"]
mod resource_programs;
use resource_programs::parse_mana_value_limit;
