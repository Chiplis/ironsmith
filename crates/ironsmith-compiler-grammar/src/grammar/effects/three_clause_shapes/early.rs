use std::ops::Range;

use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken};

use super::super::sequence_pairs::{
    contains_content_sequence, contains_sequence_phrase, contains_sequence_word,
    finish_sequence_words, matches_complete_content_sequence, matches_complete_sequence,
    seek_sequence_phrase, sequence_any_phrase, sequence_phrase, starts_content_sequence,
    starts_sequence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterThenFightShape {
    pub required_power: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentExileThenHandShape {
    pub exile_filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchThenNameShape {
    pub search: Range<usize>,
    pub name: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenNameRevealShape {
    pub view: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandOrNonlandConsultSequenceShape {
    pub remainder_order: LibraryBottomOrderAst,
}

const CHOSEN_KIND_TO_HAND: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
];
const MILLED_TO_GRAVEYARD: &[&[&str]] = &[
    &["put", "into", "graveyards", "this", "way"],
    &["put", "into", "graveyard", "this", "way"],
];
const OPPONENT_EXILES: &[&[&str]] = &[&["an", "opponent", "exiles"], &["opponent", "exiles"]];
const DOES_NOT_HAVE_NAME: &[&[&str]] = &[
    &["doesn't", "have", "that", "name"],
    &["doesnt", "have", "that", "name"],
    &["doesn", "t", "have", "that", "name"],
];
const SEARCH_RESULT_HAND: &[&[&str]] = &[
    &["put", "one", "into", "your", "hand"],
    &["put", "one", "of", "them", "into", "your", "hand"],
];
const OTHER_RESULT_GRAVEYARD: &[&[&str]] = &[
    &["other", "into", "your", "graveyard"],
    &["other", "into", "graveyard"],
];
const SHUFFLE: &[&[&str]] = &[&["then", "shuffle"], &["shuffle"]];

fn counter_power(input: &mut LexStream<'_>) -> winnow::error::ModalResult<u32> {
    sequence_phrase(&[
        "put", "a", "+1/+1", "counter", "on", "the", "creature", "you", "control", "if", "it",
        "has", "power",
    ])
    .parse_next(input)?;
    let required_power = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    sequence_phrase(&["or", "greater"]).parse_next(input)?;
    finish_sequence_words(input)?;
    Ok(required_power)
}

pub fn parse_counter_then_fight_shape(
    counter: &[OwnedLexToken],
    fight: &[OwnedLexToken],
) -> Option<CounterThenFightShape> {
    if !matches_complete_sequence(
        fight,
        &[&["then", "those", "creatures", "fight", "each", "other"]],
    ) {
        return None;
    }
    let required_power =
        crate::grammar::primitives::probe_all(counter, counter_power, "counter-then-fight-power")?;
    Some(CounterThenFightShape { required_power })
}

pub fn parse_land_or_nonland_consult_sequence_tokens(
    choice: &[OwnedLexToken],
    reveal: &[OwnedLexToken],
    move_match: &[OwnedLexToken],
) -> Option<LandOrNonlandConsultSequenceShape> {
    if !matches_complete_sequence(choice, &[&["choose", "land", "or", "nonland"]])
        || !starts_sequence(
            reveal,
            &[&[
                "reveal", "cards", "from", "the", "top", "of", "your", "library",
            ]],
        )
        || !contains_sequence_phrase(reveal, &[&["a", "card", "of", "the", "chosen", "kind"]])
        || !starts_sequence(move_match, CHOSEN_KIND_TO_HAND)
        || !contains_sequence_word(move_match, "rest")
    {
        return None;
    }
    Some(LandOrNonlandConsultSequenceShape {
        remainder_order: parse_consult_remainder_order_tokens(move_match)?,
    })
}

pub fn is_milled_creature_exile_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, &[&["exile", "up", "to", "two"]])
        && contains_sequence_phrase(tokens, &[&["creature", "cards"]])
        && contains_sequence_phrase(tokens, MILLED_TO_GRAVEYARD)
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "revealed", "this", "way"],
    &["from", "among", "them"],
];

pub fn parse_opponent_exile_then_hand_shape(
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<OpponentExileThenHandShape> {
    let mut then_input = LexStream::new(second);
    let then_at = crate::grammar::primitives::take_leaf(&mut then_input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["then"]])
    })?;
    crate::grammar::primitives::take_leaf(&mut then_input, sequence_phrase(&["then"]))?;
    let rest_start = second.len().saturating_sub(then_input.len());
    if !matches_complete_content_sequence(
        &second[rest_start..],
        &[&["you", "put", "rest", "into", "your", "hand"]],
    ) || !matches_complete_content_sequence(
        third,
        &[&[
            "that", "opponent", "may", "cast", "exiled", "card", "without", "paying", "its",
            "mana", "cost",
        ]],
    ) {
        return None;
    }

    let exile = &second[..then_at];
    let mut exile_input = LexStream::new(exile);
    crate::grammar::primitives::take_leaf(&mut exile_input, sequence_any_phrase(OPPONENT_EXILES))?;
    let filter_start = exile.len().saturating_sub(exile_input.len());
    let filter_relative_end =
        crate::grammar::primitives::take_leaf(&mut exile_input, |input: &mut _| {
            seek_sequence_phrase(input, FROM_AMONG)
        })?;
    let filter_end = filter_start + filter_relative_end;
    crate::grammar::primitives::take_leaf(&mut exile_input, sequence_any_phrase(FROM_AMONG))?;
    crate::grammar::primitives::take_leaf(&mut exile_input, finish_sequence_words)?;
    (filter_start < filter_end).then_some(OpponentExileThenHandShape {
        exile_filter: filter_start..filter_end,
    })
}

const CHOOSE_NAME: &[&[&str]] = &[
    &["that", "player", "chooses", "card", "name"],
    &["that", "player", "choose", "card", "name"],
];
const SEARCH_CARD: &[&[&str]] = &[
    &["search", "that", "player's", "library", "for", "card"],
    &["search", "that", "players", "library", "for", "card"],
];

pub fn parse_search_then_name_shape(
    first: &[OwnedLexToken],
    conditional: &[OwnedLexToken],
    shuffle: &[OwnedLexToken],
) -> Option<SearchThenNameShape> {
    let mut input = LexStream::new(first);
    let then_at = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["then"]])
    })?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(&["then"]))?;
    let name_start = first.len().saturating_sub(input.len());
    if !matches_complete_content_sequence(&first[..then_at], SEARCH_CARD)
        || !matches_complete_content_sequence(&first[name_start..], CHOOSE_NAME)
        || !contains_sequence_phrase(conditional, &[&["if", "you", "searched", "for"]])
        || !contains_sequence_phrase(conditional, &[&["creature", "card"]])
        || !contains_sequence_phrase(conditional, DOES_NOT_HAVE_NAME)
        || !contains_sequence_phrase(
            conditional,
            &[&[
                "you",
                "may",
                "put",
                "it",
                "onto",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
            ]],
        )
        || !matches_complete_sequence(
            shuffle,
            &[
                &["then", "that", "player", "shuffles"],
                &["then", "that", "player", "shuffle"],
            ],
        )
    {
        return None;
    }
    Some(SearchThenNameShape {
        search: 0..then_at,
        name: name_start..first.len(),
    })
}

#[cfg(test)]
#[path = "early/tests.rs"]
mod tests;

const CHOSEN_NAME_HAND: &[&[&str]] = &[
    &[
        "and", "put", "all", "of", "them", "with", "that", "name", "into",
    ],
    &[
        "and", "puts", "all", "of", "them", "with", "that", "name", "into",
    ],
    &[
        "and", "put", "all", "of", "those", "cards", "with", "that", "name", "into",
    ],
    &[
        "and", "puts", "all", "of", "those", "cards", "with", "that", "name", "into",
    ],
    &[
        "and", "put", "all", "of", "them", "with", "the", "chosen", "name", "into",
    ],
    &[
        "and", "puts", "all", "of", "them", "with", "the", "chosen", "name", "into",
    ],
];

pub fn parse_chosen_name_reveal_shape(
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<ChosenNameRevealShape> {
    let mut input = LexStream::new(second);
    let suffix_at = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, CHOSEN_NAME_HAND)
    })?;
    if !contains_sequence_word(&second[suffix_at..], "hand")
        || !starts_content_sequence(third, &[&["put", "rest"], &["puts", "rest"]])
        || !contains_sequence_word(third, "graveyard")
    {
        return None;
    }
    Some(ChosenNameRevealShape { view: 0..suffix_at })
}

pub fn is_search_two_disposition_then_shuffle_shape(
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> bool {
    starts_content_sequence(second, SEARCH_RESULT_HAND)
        && contains_content_sequence(second, OTHER_RESULT_GRAVEYARD)
        && matches_complete_content_sequence(third, SHUFFLE)
}

pub fn parse_consult_remainder_order_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LibraryBottomOrderAst> {
    if !contains_sequence_word(tokens, "bottom") || !contains_sequence_word(tokens, "library") {
        return None;
    }
    if contains_sequence_phrase(tokens, &[&["random", "order"]]) {
        Some(LibraryBottomOrderAst::Random)
    } else if contains_sequence_phrase(tokens, &[&["any", "order"]]) {
        Some(LibraryBottomOrderAst::ChooserChooses)
    } else {
        None
    }
}
