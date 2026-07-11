use std::ops::Range;

use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

use super::super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookedCardDestinationShape {
    Hand,
    Graveyard,
    LibraryTop,
    LibraryBottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThreeWayLookedCardDispositionShape {
    HandTopBottom,
    HandGraveyardBottom,
}

impl ThreeWayLookedCardDispositionShape {
    pub(crate) const fn destinations(self) -> [LookedCardDestinationShape; 3] {
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
pub(crate) enum RevealedCardChooserShape {
    You,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevealedCardChoiceShape {
    pub(crate) chooser: RevealedCardChooserShape,
    pub(crate) destination: Option<LookedCardDestinationShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChosenCardMoveFollowupShape {
    pub(crate) destination: LookedCardDestinationShape,
    pub(crate) followup: Range<usize>,
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

pub(crate) fn parse_three_way_looked_card_disposition_shape(
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

pub(crate) fn parse_revealed_card_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealedCardChoiceShape> {
    revealed_card_choice
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .ok()
}

pub(crate) fn parse_chosen_card_move_followup_shape(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

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
}
