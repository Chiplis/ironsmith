use std::ops::Range;

use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::{ChoiceCount, LibraryBottomOrderAst};
use crate::grammar::leaf;
use crate::lexer::{LexStream, OwnedLexToken};

use super::super::super::primitives;
use super::{
    contains_content_sequence, contains_sequence_phrase, contains_sequence_word,
    matches_complete_content_sequence, seek_sequence_phrase, sequence_any_phrase, sequence_phrase,
    starts_content_sequence, starts_sequence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookExileFaceDownShape {
    Counted {
        look: Range<usize>,
        exile: Range<usize>,
        count: ChoiceCount,
        bottom_order: LibraryBottomOrderAst,
    },
    CountedGraveyardRemainder {
        look: Range<usize>,
        exile: Range<usize>,
        count: ChoiceCount,
    },
    Single {
        look: Range<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookedCardDisposition {
    HandAndLibraryBottom(LibraryBottomOrderAst),
    HandAndGraveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookedPartitionDestination {
    Hand,
    Graveyard,
    LibraryTop(LibraryBottomOrderAst),
    LibraryBottom(LibraryBottomOrderAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookedCardPartitionShape {
    pub selected_count: ChoiceCount,
    pub selected_destination: LookedPartitionDestination,
    pub remainder_destination: LookedPartitionDestination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedCloakPartitionShape {
    pub look: Range<usize>,
    pub selected_count: ChoiceCount,
    pub remainder_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookedCardIntoHandShape {
    pub filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevealTopRemainder {
    Graveyard,
    LibraryBottom(LibraryBottomOrderAst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealTopMatchingFollowupShape {
    pub filter: Range<usize>,
    pub chosen_type_reference: bool,
    pub remainder: RevealTopRemainder,
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
const GRAVEYARDS_REST: &[&[&str]] = &[
    &["put", "rest", "into", "graveyard"],
    &["put", "rest", "into", "your", "graveyard"],
    &["put", "rest", "into", "their", "graveyard"],
    &["put", "the", "rest", "into", "graveyard"],
    &["put", "the", "rest", "into", "your", "graveyard"],
    &["put", "the", "rest", "into", "their", "graveyard"],
];

pub fn parse_bottom_order(tokens: &[OwnedLexToken]) -> Option<LibraryBottomOrderAst> {
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

fn counted_face_down_graveyard_shape(tokens: &[OwnedLexToken]) -> Option<ChoiceCount> {
    let (count, tail) = primitives::parse_prefix(tokens, parse_counted_face_down_head)?;
    if !starts_content_sequence(tail, COUNTED_FACE_DOWN_PREFIXES)
        || !contains_content_sequence(tail, GRAVEYARDS_REST)
    {
        return None;
    }
    Some(count)
}

pub fn parse_look_exile_face_down_shape(
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
    if let Some(count) = counted_face_down_graveyard_shape(&tokens[exile.clone()]) {
        return Some(LookExileFaceDownShape::CountedGraveyardRemainder {
            look: 0..exile_at,
            exile,
            count,
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

pub fn parse_looked_card_disposition(tokens: &[OwnedLexToken]) -> Option<LookedCardDisposition> {
    if !starts_sequence(tokens, PUT_ONE_HAND) {
        return None;
    }
    if contains_content_sequence(tokens, OTHER_BOTTOM) && contains_sequence_word(tokens, "library")
    {
        return Some(LookedCardDisposition::HandAndLibraryBottom(
            parse_bottom_order(tokens).unwrap_or(LibraryBottomOrderAst::ChooserChooses),
        ));
    }
    if contains_content_sequence(tokens, OTHER_GRAVEYARD) {
        return Some(LookedCardDisposition::HandAndGraveyard);
    }
    None
}

fn looked_partition_count(input: &mut LexStream<'_>) -> WResult<ChoiceCount> {
    alt((
        alt((
            primitives::phrase(&["any", "number", "of", "them"]),
            primitives::phrase(&["any", "number", "of", "those", "cards"]),
        ))
        .value(ChoiceCount::any_number()),
        (
            leaf::parse_leaf_choice_count_prefix_lexed,
            alt((
                primitives::phrase(&["of", "them"]),
                primitives::phrase(&["of", "those", "cards"]),
            )),
        )
            .map(|(count, _)| count),
    ))
    .parse_next(input)
}

fn looked_partition_order(input: &mut LexStream<'_>) -> WResult<LibraryBottomOrderAst> {
    alt((
        primitives::phrase(&["in", "any", "order"]).value(LibraryBottomOrderAst::ChooserChooses),
        primitives::phrase(&["in", "a", "random", "order"]).value(LibraryBottomOrderAst::Random),
        primitives::phrase(&["in", "random", "order"]).value(LibraryBottomOrderAst::Random),
    ))
    .parse_next(input)
}

fn looked_partition_library_reference(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["your", "library"]),
        primitives::phrase(&["their", "library"]),
        primitives::phrase(&["that", "library"]),
        primitives::phrase(&["the", "library"]),
        primitives::phrase(&["that", "players", "library"]),
        primitives::phrase(&["that", "player's", "library"]),
    ))
    .parse_next(input)
}

fn looked_partition_library_destination(
    input: &mut LexStream<'_>,
) -> WResult<LookedPartitionDestination> {
    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let top = alt((
        primitives::kw("top").value(true),
        primitives::kw("bottom").value(false),
    ))
    .parse_next(input)?;
    primitives::kw("of").parse_next(input)?;
    looked_partition_library_reference.parse_next(input)?;
    let order = looked_partition_order.parse_next(input)?;
    Ok(if top {
        LookedPartitionDestination::LibraryTop(order)
    } else {
        LookedPartitionDestination::LibraryBottom(order)
    })
}

fn looked_cloak_partition_tail(
    input: &mut LexStream<'_>,
) -> WResult<(ChoiceCount, LibraryBottomOrderAst)> {
    primitives::kw("cloak").parse_next(input)?;
    let selected_count = leaf::parse_leaf_choice_count_prefix_lexed.parse_next(input)?;
    sequence_any_phrase(&[&["of", "them"], &["of", "those", "cards"]]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    primitives::kw("put").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("rest").parse_next(input)?;
    let LookedPartitionDestination::LibraryBottom(remainder_order) =
        looked_partition_library_destination.parse_next(input)?
    else {
        return Err(primitives::backtrack_err(
            "looked-card cloak partition",
            "the exact unselected remainder on the bottom of the library",
        ));
    };
    primitives::sentence_end().parse_next(input)?;
    Ok((selected_count, remainder_order))
}

pub fn parse_look_cloak_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCloakPartitionShape> {
    let (cloak_at, _, _) = primitives::find_prefix(tokens, || primitives::kw("cloak").void())?;
    if cloak_at == 0 {
        return None;
    }
    let mut tail = LexStream::new(&tokens[cloak_at..]);
    let (selected_count, remainder_order) =
        looked_cloak_partition_tail.parse_next(&mut tail).ok()?;
    tail.is_empty().then_some(LookedCloakPartitionShape {
        look: 0..cloak_at,
        selected_count,
        remainder_order,
    })
}

fn looked_partition_destination(input: &mut LexStream<'_>) -> WResult<LookedPartitionDestination> {
    alt((
        primitives::phrase(&["into", "your", "hand"]).value(LookedPartitionDestination::Hand),
        alt((
            primitives::phrase(&["into", "your", "graveyard"]),
            primitives::phrase(&["into", "their", "graveyard"]),
            primitives::phrase(&["into", "that", "players", "graveyard"]),
            primitives::phrase(&["into", "that", "player's", "graveyard"]),
        ))
        .value(LookedPartitionDestination::Graveyard),
        looked_partition_library_destination,
    ))
    .parse_next(input)
}

fn looked_card_partition(input: &mut LexStream<'_>) -> WResult<LookedCardPartitionShape> {
    primitives::kw("put").parse_next(input)?;
    let selected_count = looked_partition_count.parse_next(input)?;
    let selected_destination = looked_partition_destination.parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    alt((primitives::kw("rest"), primitives::kw("other")))
        .void()
        .parse_next(input)?;
    let remainder_destination = looked_partition_destination.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let selected_then_library_top = matches!(
        selected_destination,
        LookedPartitionDestination::Hand
            | LookedPartitionDestination::Graveyard
            | LookedPartitionDestination::LibraryBottom(_)
    ) && matches!(
        remainder_destination,
        LookedPartitionDestination::LibraryTop(_)
    );
    let selected_hand_then_graveyard = matches!(
        (selected_destination, remainder_destination),
        (
            LookedPartitionDestination::Hand,
            LookedPartitionDestination::Graveyard
        )
    );
    if !(selected_then_library_top || selected_hand_then_graveyard) {
        return Err(primitives::backtrack_err(
            "looked-card partition",
            "supported destinations for a selected subset and its exact remainder",
        ));
    }

    Ok(LookedCardPartitionShape {
        selected_count,
        selected_destination,
        remainder_destination,
    })
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "milled", "this", "way"],
    &["from", "among", "the", "milled", "cards"],
    &["from", "among", "them"],
];

const PUT_ALL: &[&[&str]] = &[&["put", "all"], &["puts", "all"]];
const CHOSEN_TYPE: &[&[&str]] = &[&["chosen", "type"], &["that", "type"]];
const REST_GRAVEYARD: &[&[&str]] = &[&["and", "rest", "into", "your"]];

#[cfg(test)]
#[path = "library_inline_tests.rs"]
mod tests;

#[path = "library/library.rs"]
mod library_programs;
use library_programs::looked_card_optional_one_top_remainder_bottom;
pub use library_programs::{
    parse_looked_card_into_hand_shape, parse_looked_card_partition_shape,
    parse_reveal_top_matching_followup_shape,
};
#[path = "library/choice.rs"]
mod choice_programs;
pub use choice_programs::is_keyword_bundle_choice_filter;
