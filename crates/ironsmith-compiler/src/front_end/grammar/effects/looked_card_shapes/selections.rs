use std::ops::Range;

use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

use super::super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookedCardDestinationShape {
    Hand,
    Graveyard,
    Battlefield,
    LibraryTop,
    LibraryBottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreeWayLookedCardDispositionShape {
    HandTopBottom,
    HandGraveyardBottom,
}

impl ThreeWayLookedCardDispositionShape {
    pub const fn destinations(self) -> [LookedCardDestinationShape; 3] {
        match self {
            Self::HandTopBottom => [
                LookedCardDestinationShape::Hand,
                LookedCardDestinationShape::LibraryTop,
                LookedCardDestinationShape::LibraryBottom,
            ],
            Self::HandGraveyardBottom => [
                LookedCardDestinationShape::Hand,
                LookedCardDestinationShape::Graveyard,
                LookedCardDestinationShape::LibraryBottom,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealedCardChooserShape {
    You,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealedCardChoiceShape {
    pub chooser: RevealedCardChooserShape,
    pub destination: Option<LookedCardDestinationShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenCardMoveFollowupShape {
    pub destination: LookedCardDestinationShape,
    pub followup: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentRevealedCardSelectionShape {
    pub filter: Option<Range<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChosenCardPartitionShape {
    pub selected_destination: LookedCardDestinationShape,
    pub remainder_destination: LookedCardDestinationShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountedLookedHandRemainderShape {
    pub count: ChoiceCount,
    pub remainder_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionalLookedTopRemainderShape {
    pub count: ChoiceCount,
    pub remainder_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactLookedCardMoveShape {
    pub destination: LookedCardDestinationShape,
}

fn counted_looked_reference(input: &mut LexStream<'_>) -> WResult<ChoiceCount> {
    let count = leaf::parse_leaf_choice_count_prefix_lexed.parse_next(input)?;
    alt((
        primitives::phrase(&["of", "those", "cards"]),
        primitives::phrase(&["of", "them"]),
    ))
    .void()
    .parse_next(input)?;
    Ok(count)
}

fn library_bottom_order(input: &mut LexStream<'_>) -> WResult<LibraryBottomOrderAst> {
    alt((
        primitives::phrase(&["in", "a", "random", "order"]).value(LibraryBottomOrderAst::Random),
        primitives::phrase(&["in", "random", "order"]).value(LibraryBottomOrderAst::Random),
        primitives::phrase(&["in", "any", "order"]).value(LibraryBottomOrderAst::ChooserChooses),
    ))
    .parse_next(input)
}

fn library_position(input: &mut LexStream<'_>, position: &str) -> WResult<()> {
    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    if position == "top" {
        primitives::kw("top").parse_next(input)?;
    } else {
        primitives::kw("bottom").parse_next(input)?;
    }
    primitives::phrase(&["of", "your", "library"])
        .void()
        .parse_next(input)
}

fn standalone_library_bottom_remainder(
    input: &mut LexStream<'_>,
) -> WResult<LibraryBottomOrderAst> {
    primitives::kw("put").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("rest").parse_next(input)?;
    library_position(input, "bottom")?;
    let order = library_bottom_order.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(order)
}

fn counted_looked_hand_remainder(
    input: &mut LexStream<'_>,
) -> WResult<CountedLookedHandRemainderShape> {
    primitives::kw("put").parse_next(input)?;
    let count = counted_looked_reference.parse_next(input)?;
    primitives::phrase(&["into", "your", "hand"])
        .void()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("rest").parse_next(input)?;
    library_position(input, "bottom")?;
    let remainder_order = library_bottom_order.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CountedLookedHandRemainderShape {
        count,
        remainder_order,
    })
}

pub fn parse_counted_looked_hand_remainder_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedLookedHandRemainderShape> {
    counted_looked_hand_remainder
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .ok()
}

fn optional_looked_top_selection(input: &mut LexStream<'_>) -> WResult<ChoiceCount> {
    primitives::phrase(&["you", "may", "put"])
        .void()
        .parse_next(input)?;
    let count = counted_looked_reference.parse_next(input)?;
    library_position(input, "top")?;
    primitives::sentence_end().parse_next(input)?;
    if count == ChoiceCount::exactly(1) || count == ChoiceCount::up_to(1) {
        // The surrounding `you may` carries the optionality.  Preserve the
        // selected branch as an exact singleton so lowering can represent the
        // choice as `May { choose one, move it }` instead of losing the printed
        // "may" in a flat `up to one` selection.
        Ok(ChoiceCount::exactly(1))
    } else {
        Err(primitives::backtrack_err(
            "optional looked-card top selection",
            "one or up to one looked card",
        ))
    }
}

pub fn parse_optional_looked_top_remainder_shape(
    selection_tokens: &[OwnedLexToken],
    remainder_tokens: &[OwnedLexToken],
) -> Option<OptionalLookedTopRemainderShape> {
    let count = optional_looked_top_selection
        .parse(LexStream::new(trim_lexed_commas(selection_tokens)))
        .ok()?;
    let remainder_order = standalone_library_bottom_remainder
        .parse(LexStream::new(trim_lexed_commas(remainder_tokens)))
        .ok()?;
    Some(OptionalLookedTopRemainderShape {
        count,
        remainder_order,
    })
}

fn looked_one_reference(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("one").parse_next(input)?;
    alt((
        primitives::phrase(&["of", "those", "cards"]),
        primitives::phrase(&["of", "them"]),
    ))
    .void()
    .parse_next(input)
}

fn tagged_card_reference(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "card"]),
        primitives::phrase(&["the", "chosen", "card"]),
    ))
    .void()
    .parse_next(input)
}

fn looked_card_destination(input: &mut LexStream<'_>) -> WResult<LookedCardDestinationShape> {
    alt((
        primitives::phrase(&["into", "your", "hand"]).value(LookedCardDestinationShape::Hand),
        alt((
            primitives::phrase(&["into", "your", "graveyard"]),
            primitives::phrase(&["into", "their", "graveyard"]),
        ))
        .value(LookedCardDestinationShape::Graveyard),
        primitives::phrase(&["onto", "the", "battlefield"])
            .value(LookedCardDestinationShape::Battlefield),
        alt((
            primitives::phrase(&["on", "top", "of", "your", "library"]),
            primitives::phrase(&["on", "the", "top", "of", "your", "library"]),
        ))
        .value(LookedCardDestinationShape::LibraryTop),
        alt((
            primitives::phrase(&["on", "bottom", "of", "your", "library"]),
            primitives::phrase(&["on", "the", "bottom", "of", "your", "library"]),
        ))
        .value(LookedCardDestinationShape::LibraryBottom),
    ))
    .parse_next(input)
}

fn bare_one_destination(input: &mut LexStream<'_>) -> WResult<LookedCardDestinationShape> {
    primitives::kw("one").parse_next(input)?;
    looked_card_destination.parse_next(input)
}

fn exact_looked_card_move(input: &mut LexStream<'_>) -> WResult<ExactLookedCardMoveShape> {
    primitives::kw("put").parse_next(input)?;
    looked_one_reference.parse_next(input)?;
    let destination = looked_card_destination.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ExactLookedCardMoveShape { destination })
}

pub fn parse_exact_looked_card_move_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExactLookedCardMoveShape> {
    exact_looked_card_move
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .ok()
}

fn three_way_disposition(input: &mut LexStream<'_>) -> WResult<ThreeWayLookedCardDispositionShape> {
    primitives::kw("put").parse_next(input)?;
    looked_one_reference.parse_next(input)?;
    let first = looked_card_destination.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let second = bare_one_destination.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let third = bare_one_destination.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    match [first, second, third] {
        [
            LookedCardDestinationShape::Hand,
            LookedCardDestinationShape::LibraryTop,
            LookedCardDestinationShape::LibraryBottom,
        ] => Ok(ThreeWayLookedCardDispositionShape::HandTopBottom),
        [
            LookedCardDestinationShape::Hand,
            LookedCardDestinationShape::Graveyard,
            LookedCardDestinationShape::LibraryBottom,
        ] => Ok(ThreeWayLookedCardDispositionShape::HandGraveyardBottom),
        _ => Err(primitives::backtrack_err(
            "three-way looked-card disposition",
            "supported distinct destinations",
        )),
    }
}

pub fn parse_three_way_looked_card_disposition_shape(
    tokens: &[OwnedLexToken],
) -> Option<ThreeWayLookedCardDispositionShape> {
    three_way_disposition
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .ok()
}

fn revealed_card_chooser(input: &mut LexStream<'_>) -> WResult<RevealedCardChooserShape> {
    alt((
        primitives::phrase(&["you", "choose"]).value(RevealedCardChooserShape::You),
        primitives::phrase(&["target", "opponent", "chooses"])
            .value(RevealedCardChooserShape::TargetOpponent),
    ))
    .parse_next(input)
}

fn revealed_card_choice(input: &mut LexStream<'_>) -> WResult<RevealedCardChoiceShape> {
    let chooser = revealed_card_chooser.parse_next(input)?;
    looked_one_reference.parse_next(input)?;
    let destination = opt((
        primitives::kw("and"),
        primitives::kw("put"),
        tagged_card_reference,
        looked_card_destination,
    ))
    .parse_next(input)?
    .map(|(_, _, _, destination)| destination);
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(RevealedCardChoiceShape {
        chooser,
        destination,
    })
}

pub fn parse_revealed_card_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealedCardChoiceShape> {
    revealed_card_choice
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .ok()
}

pub fn is_one_looked_card_into_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(trim_lexed_commas(tokens));
    if looked_one_reference.parse_next(&mut input).is_err() {
        return false;
    }
    if looked_card_destination.parse_next(&mut input).ok() != Some(LookedCardDestinationShape::Hand)
    {
        return false;
    }
    opt(primitives::period()).parse_next(&mut input).is_ok() && input.is_empty()
}

pub fn parse_chosen_card_move_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChosenCardMoveFollowupShape> {
    let tokens = trim_lexed_commas(tokens);
    let mut input = LexStream::new(tokens);
    primitives::kw("put").parse_next(&mut input).ok()?;
    tagged_card_reference.parse_next(&mut input).ok()?;
    let destination = looked_card_destination.parse_next(&mut input).ok()?;
    opt(primitives::comma()).parse_next(&mut input).ok()?;
    primitives::kw("then").parse_next(&mut input).ok()?;
    let followup_start = tokens.len().saturating_sub(input.len());
    (followup_start < tokens.len()).then_some(ChosenCardMoveFollowupShape {
        destination,
        followup: followup_start..tokens.len(),
    })
}

pub fn parse_opponent_revealed_card_selection_shape(
    tokens: &[OwnedLexToken],
) -> Option<OpponentRevealedCardSelectionShape> {
    let tokens = trim_lexed_commas(tokens);
    if !tokens.first()?.is_word("an")
        || !tokens.get(1)?.is_word("opponent")
        || !tokens.get(2)?.is_word("chooses")
    {
        return None;
    }
    let from_among = (3..tokens.len().saturating_sub(2)).find(|index| {
        tokens[*index].is_word("from")
            && tokens[*index + 1].is_word("among")
            && tokens[*index + 2].is_word("them")
    });
    let filter = if let Some(from_among) = from_among {
        if from_among == 3
            || tokens[from_among + 3..]
                .iter()
                .any(|token| token.as_word().is_some())
        {
            return None;
        }
        Some(3..from_among)
    } else {
        let words = tokens[3..]
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        if words.as_slice() != ["one", "of", "them"]
            && words.as_slice() != ["one", "of", "those", "cards"]
        {
            return None;
        }
        None
    };
    Some(OpponentRevealedCardSelectionShape { filter })
}

fn chosen_card_partition(input: &mut LexStream<'_>) -> WResult<ChosenCardPartitionShape> {
    primitives::kw("put").parse_next(input)?;
    tagged_card_reference.parse_next(input)?;
    let selected_destination = looked_card_destination.parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("rest").parse_next(input)?;
    let remainder_destination = looked_card_destination.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ChosenCardPartitionShape {
        selected_destination,
        remainder_destination,
    })
}

pub fn parse_chosen_card_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChosenCardPartitionShape> {
    chosen_card_partition
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_distinct_three_way_looked_card_dispositions() {
        let telling = parse_three_way_looked_card_disposition_shape(&lex(
            "Put one of those cards into your hand, one on top of your library, and one on the bottom of your library",
        ))
        .unwrap();
        assert_eq!(telling, ThreeWayLookedCardDispositionShape::HandTopBottom);

        let moment = parse_three_way_looked_card_disposition_shape(&lex(
            "Put one of those cards into your hand, one into your graveyard, and one on the bottom of your library",
        ))
        .unwrap();
        assert_eq!(
            moment,
            ThreeWayLookedCardDispositionShape::HandGraveyardBottom
        );
    }

    #[test]
    fn parses_revealed_candidate_choices_and_chosen_followup() {
        assert_eq!(
            parse_revealed_card_choice_shape(&lex(
                "You choose one of those cards and put it into their graveyard"
            )),
            Some(RevealedCardChoiceShape {
                chooser: RevealedCardChooserShape::You,
                destination: Some(LookedCardDestinationShape::Graveyard),
            })
        );
        assert_eq!(
            parse_revealed_card_choice_shape(&lex("Target opponent chooses one of those cards")),
            Some(RevealedCardChoiceShape {
                chooser: RevealedCardChooserShape::TargetOpponent,
                destination: None,
            })
        );

        let tokens = lex("Put that card into your graveyard, then draw two cards");
        let followup = parse_chosen_card_move_followup_shape(&tokens).unwrap();
        assert_eq!(followup.destination, LookedCardDestinationShape::Graveyard);
        assert_eq!(
            tokens[followup.followup]
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect::<Vec<_>>(),
            ["draw", "two", "cards"]
        );
    }

    #[test]
    fn parses_opponent_filtered_choice_and_exact_complement_disposition() {
        let creature_tokens = lex("An opponent chooses a creature card from among them");
        let creature = parse_opponent_revealed_card_selection_shape(&creature_tokens).unwrap();
        assert_eq!(
            creature_tokens[creature.filter.unwrap()]
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect::<Vec<_>>(),
            ["a", "creature", "card"]
        );
        assert_eq!(
            parse_opponent_revealed_card_selection_shape(&lex("An opponent chooses one of them")),
            Some(OpponentRevealedCardSelectionShape { filter: None })
        );
        assert_eq!(
            parse_chosen_card_partition_shape(&lex(
                "Put that card onto the battlefield and the rest into your graveyard"
            )),
            Some(ChosenCardPartitionShape {
                selected_destination: LookedCardDestinationShape::Battlefield,
                remainder_destination: LookedCardDestinationShape::Graveyard,
            })
        );
    }

    #[test]
    fn parses_exact_singleton_looked_card_moves() {
        for (text, destination) in [
            (
                "Put one of them into your graveyard",
                LookedCardDestinationShape::Graveyard,
            ),
            (
                "Put one of those cards into their graveyard",
                LookedCardDestinationShape::Graveyard,
            ),
        ] {
            assert_eq!(
                parse_exact_looked_card_move_shape(&lex(text)),
                Some(ExactLookedCardMoveShape { destination })
            );
        }

        assert!(
            parse_exact_looked_card_move_shape(&lex("Put the rest into your graveyard")).is_none()
        );
    }

    #[test]
    fn parses_direct_counted_hand_and_random_remainder_partition() {
        for (text, count) in [
            (
                "Put two of those cards into your hand and the rest on the bottom of your library in a random order",
                ChoiceCount::exactly(2),
            ),
            (
                "Put three of them into your hand and the rest on the bottom of your library in a random order",
                ChoiceCount::exactly(3),
            ),
        ] {
            assert_eq!(
                parse_counted_looked_hand_remainder_shape(&lex(text)),
                Some(CountedLookedHandRemainderShape {
                    count,
                    remainder_order: LibraryBottomOrderAst::Random,
                })
            );
        }
    }

    #[test]
    fn parses_separate_optional_top_selection_and_exact_remainder() {
        assert_eq!(
            parse_optional_looked_top_remainder_shape(
                &lex("You may put one of those cards on top of your library"),
                &lex("Put the rest on the bottom of your library in a random order"),
            ),
            Some(OptionalLookedTopRemainderShape {
                count: ChoiceCount::exactly(1),
                remainder_order: LibraryBottomOrderAst::Random,
            })
        );
    }
}
