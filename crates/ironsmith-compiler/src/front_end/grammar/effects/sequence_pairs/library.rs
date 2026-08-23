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

/// Parses the common private-library split where at most one looked-at card
/// is kept on top and the exact complement is randomized onto the bottom.
///
/// The selected set is a singleton, so Magic omits an ordering clause for
/// its top placement.  Keep the ordinary partition grammar strict about
/// explicit library ordering and admit that omission only for this bounded
/// shape.  The bottom library reference may also be elided after the earlier
/// "top of your library" reference (for example, "the rest on the bottom in
/// a random order").
fn looked_card_optional_one_top_remainder_bottom(
    input: &mut LexStream<'_>,
) -> WResult<LookedCardPartitionShape> {
    primitives::kw("put").parse_next(input)?;
    let selected_count = looked_partition_count.parse_next(input)?;
    if selected_count != ChoiceCount::up_to(1) {
        return Err(primitives::backtrack_err(
            "looked-card top/bottom partition",
            "an up-to-one selected set",
        ));
    }

    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("top").parse_next(input)?;
    primitives::kw("of").parse_next(input)?;
    looked_partition_library_reference.parse_next(input)?;

    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    alt((primitives::kw("rest"), primitives::kw("other")))
        .void()
        .parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("bottom").parse_next(input)?;
    opt((primitives::kw("of"), looked_partition_library_reference)).parse_next(input)?;
    let remainder_order = looked_partition_order.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(LookedCardPartitionShape {
        selected_count,
        // A chooser order is semantically inert for a set of at most one but
        // gives lowering the typed library-placement order it requires.
        selected_destination: LookedPartitionDestination::LibraryTop(
            LibraryBottomOrderAst::ChooserChooses,
        ),
        remainder_destination: LookedPartitionDestination::LibraryBottom(remainder_order),
    })
}

/// Parses a complete two-way partition of a previously looked-at card set.
///
/// Requiring the sentence to end after both destinations prevents this rule
/// from swallowing longer looked-card procedures. Library placements retain
/// their own order modes so the selected subset and its complement can be
/// ordered independently.
pub fn parse_looked_card_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardPartitionShape> {
    alt((
        looked_card_optional_one_top_remainder_bottom,
        looked_card_partition,
    ))
    .parse(LexStream::new(tokens))
    .ok()
}

pub fn is_keyword_bundle_choice_filter(tokens: &[OwnedLexToken]) -> bool {
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

pub fn parse_looked_card_into_hand_shape(
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

const PUT_ALL: &[&[&str]] = &[&["put", "all"], &["puts", "all"]];
const CHOSEN_TYPE: &[&[&str]] = &[&["chosen", "type"], &["that", "type"]];
const REST_GRAVEYARD: &[&[&str]] = &[&["and", "rest", "into", "your"]];

pub fn parse_reveal_top_matching_followup_shape(
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
    use crate::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_looked_card_dispositions_and_from_among_filter() {
        assert_eq!(
            parse_looked_card_disposition(&lex(
                "Put one of them into your hand and the rest on the bottom of your library in any order"
            )),
            Some(LookedCardDisposition::HandAndLibraryBottom(
                LibraryBottomOrderAst::ChooserChooses
            ))
        );
        assert_eq!(
            parse_looked_card_disposition(&lex(
                "Put one of them into your hand and the rest on the bottom of your library in a random order"
            )),
            Some(LookedCardDisposition::HandAndLibraryBottom(
                LibraryBottomOrderAst::Random
            ))
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

    #[test]
    fn parses_face_down_selection_with_exact_graveyard_remainder() {
        assert!(matches!(
            parse_look_exile_face_down_shape(&lex(
                "Look at the top three cards of that player's library, exile one of them face down, then put the rest into their graveyard"
            )),
            Some(LookExileFaceDownShape::CountedGraveyardRemainder {
                count,
                ..
            }) if count == ChoiceCount::exactly(1)
        ));
    }

    #[test]
    fn parses_complete_looked_card_partitions_with_independent_orders() {
        assert_eq!(
            parse_looked_card_partition_shape(&lex(
                "Put one of them into your hand and the rest on top of your library in any order"
            )),
            Some(LookedCardPartitionShape {
                selected_count: ChoiceCount::exactly(1),
                selected_destination: LookedPartitionDestination::Hand,
                remainder_destination: LookedPartitionDestination::LibraryTop(
                    LibraryBottomOrderAst::ChooserChooses
                ),
            })
        );
        assert_eq!(
            parse_looked_card_partition_shape(&lex(
                "Put one of those cards into that player's graveyard and the rest on top of their library in any order"
            )),
            Some(LookedCardPartitionShape {
                selected_count: ChoiceCount::exactly(1),
                selected_destination: LookedPartitionDestination::Graveyard,
                remainder_destination: LookedPartitionDestination::LibraryTop(
                    LibraryBottomOrderAst::ChooserChooses
                ),
            })
        );
        assert_eq!(
            parse_looked_card_partition_shape(&lex(
                "Put any number of them on the bottom of that library in a random order and the rest on top of the library in any order"
            )),
            Some(LookedCardPartitionShape {
                selected_count: ChoiceCount::any_number(),
                selected_destination: LookedPartitionDestination::LibraryBottom(
                    LibraryBottomOrderAst::Random
                ),
                remainder_destination: LookedPartitionDestination::LibraryTop(
                    LibraryBottomOrderAst::ChooserChooses
                ),
            })
        );
        assert_eq!(
            parse_looked_card_partition_shape(&lex(
                "Put two of them into your hand and the other into your graveyard"
            )),
            Some(LookedCardPartitionShape {
                selected_count: ChoiceCount::exactly(2),
                selected_destination: LookedPartitionDestination::Hand,
                remainder_destination: LookedPartitionDestination::Graveyard,
            })
        );
        for text in [
            "Put up to one of them on top of your library and the rest on the bottom in a random order",
            "Put up to one of them on top of your library and the rest on the bottom of your library in a random order",
        ] {
            assert_eq!(
                parse_looked_card_partition_shape(&lex(text)),
                Some(LookedCardPartitionShape {
                    selected_count: ChoiceCount::up_to(1),
                    selected_destination: LookedPartitionDestination::LibraryTop(
                        LibraryBottomOrderAst::ChooserChooses
                    ),
                    remainder_destination: LookedPartitionDestination::LibraryBottom(
                        LibraryBottomOrderAst::Random
                    ),
                })
            );
        }
    }

    #[test]
    fn looked_card_partition_requires_full_consumption_and_top_remainder() {
        assert!(
            parse_looked_card_partition_shape(&lex(
                "Put one of them into your hand and the rest on top of your library in any order, then draw a card"
            ))
            .is_none()
        );
        for control in [
            "Put one of them into your hand and the rest on the bottom of your library in any order",
            "Put them back in any order",
        ] {
            assert!(parse_looked_card_partition_shape(&lex(control)).is_none());
        }
    }
}
