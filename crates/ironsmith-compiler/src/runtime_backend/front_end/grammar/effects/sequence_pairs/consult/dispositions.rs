use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::runtime_backend::front_end::lexer::{
    LexStream, LexedClause, OwnedLexToken, split_lexed_sentences,
};
use crate::zone::Zone;

use super::super::super::{
    ends_content_sequence, finish_sequence_words, matches_complete_content_sequence,
    seek_sequence_phrase, sequence_any_phrase, sequence_phrase, starts_content_sequence,
};
use super::parse_consult_traversal_shape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsultMoveSelectionShape {
    AllMatched,
    AnyNumberOfMatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConsultMatchedMoveShape {
    pub(crate) selection: ConsultMoveSelectionShape,
    pub(crate) zone: Zone,
    pub(crate) controller_you: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsultRepeatedMoveShape {
    pub(crate) first_filter: Vec<OwnedLexToken>,
    pub(crate) repeated_filter: Vec<OwnedLexToken>,
    pub(crate) zone: Zone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConsultMiddleShape {
    MatchedMove(ConsultMatchedMoveShape),
    RepeatedMove(ConsultRepeatedMoveShape),
    Generic(Vec<Vec<OwnedLexToken>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsultRemainderDispositionShape {
    Graveyard,
    LibraryBottom(LibraryBottomOrderAst),
    ShuffleLibrary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsultDispositionSequenceShape {
    pub(crate) consult_tokens: Vec<OwnedLexToken>,
    pub(crate) middle: ConsultMiddleShape,
    pub(crate) remainder: ConsultRemainderDispositionShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevealRepeatedDispositionSequenceShape {
    pub(crate) reveal_tokens: Vec<OwnedLexToken>,
    pub(crate) repeated: ConsultRepeatedMoveShape,
    pub(crate) remainder: ConsultRemainderDispositionShape,
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn parse_matched_move_shape(tokens: &[OwnedLexToken]) -> Option<ConsultMatchedMoveShape> {
    let tokens = trimmed(tokens);
    let ((), after_put) = crate::runtime_backend::front_end::grammar::primitives::parse_prefix(
        tokens,
        |input: &mut LexStream<'_>| {
            sequence_any_phrase(&[
                &["put"],
                &["puts"],
                &["they", "put"],
                &["that", "player", "puts"],
                &["the", "player", "puts"],
            ])
            .parse_next(input)
        },
    )?;
    let mut destination_input = LexStream::new(after_put);
    let destination_at = seek_sequence_phrase(
        &mut destination_input,
        &[
            &["into", "your", "hand"],
            &["into", "hand"],
            &["onto", "the", "battlefield"],
            &["onto", "battlefield"],
        ],
    )
    .ok()?;
    let reference = trimmed(&after_put[..destination_at]);
    let selection = if starts_content_sequence(
        reference,
        &[
            &["any", "number", "of", "those"],
            &["any", "number", "of", "them"],
        ],
    ) {
        ConsultMoveSelectionShape::AnyNumberOfMatched
    } else if matches_complete_content_sequence(
        reference,
        &[&["that", "card"], &["it"], &["those", "cards"]],
    ) || ends_content_sequence(reference, &[&["revealed", "this", "way"]])
    {
        ConsultMoveSelectionShape::AllMatched
    } else {
        return None;
    };

    let zone = alt((
        sequence_any_phrase(&[&["into", "your", "hand"], &["into", "hand"]]).value(Zone::Hand),
        sequence_any_phrase(&[&["onto", "the", "battlefield"], &["onto", "battlefield"]])
            .value(Zone::Battlefield),
    ))
    .parse_next(&mut destination_input)
    .ok()?;
    let controller_you = opt(sequence_phrase(&["under", "your", "control"]))
        .parse_next(&mut destination_input)
        .ok()?
        .is_some();
    finish_sequence_words(&mut destination_input).ok()?;
    Some(ConsultMatchedMoveShape {
        selection,
        zone,
        controller_you,
    })
}

fn parse_repeated_move_shape(tokens: &[OwnedLexToken]) -> Option<ConsultRepeatedMoveShape> {
    let tokens = trimmed(tokens);
    let mut repeated_input = LexStream::new(tokens);
    let repeated_at = seek_sequence_phrase(
        &mut repeated_input,
        &[
            &["then", "do", "the", "same", "for"],
            &["do", "the", "same", "for"],
        ],
    )
    .ok()?;
    sequence_any_phrase(&[
        &["then", "do", "the", "same", "for"],
        &["do", "the", "same", "for"],
    ])
    .parse_next(&mut repeated_input)
    .ok()?;
    let repeated_filter = trimmed(&tokens[tokens.len().saturating_sub(repeated_input.len())..]);
    if repeated_filter.is_empty() {
        return None;
    }

    let first = trimmed(&tokens[..repeated_at]);
    let ((), after_put) = crate::runtime_backend::front_end::grammar::primitives::parse_prefix(
        first,
        |input: &mut LexStream<'_>| sequence_phrase(&["put", "all"]).parse_next(input),
    )?;
    let mut revealed_input = LexStream::new(after_put);
    let revealed_at =
        seek_sequence_phrase(&mut revealed_input, &[&["revealed", "this", "way"]]).ok()?;
    let first_filter = trimmed(&after_put[..revealed_at]);
    if first_filter.is_empty() {
        return None;
    }
    sequence_phrase(&["revealed", "this", "way"])
        .parse_next(&mut revealed_input)
        .ok()?;
    let zone = sequence_any_phrase(&[&["onto", "the", "battlefield"], &["onto", "battlefield"]])
        .value(Zone::Battlefield)
        .parse_next(&mut revealed_input)
        .ok()?;
    finish_sequence_words(&mut revealed_input).ok()?;

    Some(ConsultRepeatedMoveShape {
        first_filter: first_filter.to_vec(),
        repeated_filter: repeated_filter.to_vec(),
        zone,
    })
}

fn parse_remainder_shape_inner(
    input: &mut LexStream<'_>,
) -> winnow::error::ModalResult<ConsultRemainderDispositionShape> {
    let bare_shuffle = input.checkpoint();
    if (
        opt(sequence_phrase(&["then"])),
        opt(sequence_phrase(&["that", "player"])),
        alt((
            sequence_phrase(&["shuffle"]),
            sequence_phrase(&["shuffles"]),
        )),
        finish_sequence_words,
    )
        .parse_next(input)
        .is_ok()
    {
        return Ok(ConsultRemainderDispositionShape::ShuffleLibrary);
    }
    input.reset(&bare_shuffle);

    opt(sequence_phrase(&["then"])).parse_next(input)?;
    opt(sequence_phrase(&["that", "player"])).parse_next(input)?;
    let shuffles = alt((
        sequence_phrase(&["shuffle"]).value(true),
        sequence_phrase(&["shuffles"]).value(true),
        sequence_phrase(&["put"]).value(false),
        sequence_phrase(&["puts"]).value(false),
    ))
    .parse_next(input)?;
    opt(sequence_phrase(&["the"])).parse_next(input)?;
    sequence_phrase(&["rest"]).parse_next(input)?;
    opt((
        sequence_phrase(&["of"]),
        opt(sequence_phrase(&["the"])),
        opt(sequence_phrase(&["revealed", "cards"])),
    ))
    .parse_next(input)?;

    let disposition = if shuffles {
        sequence_phrase(&["into"]).parse_next(input)?;
        opt(sequence_any_phrase(&[
            &["your"],
            &["their"],
            &["that", "player's"],
            &["that", "players"],
        ]))
        .parse_next(input)?;
        sequence_phrase(&["library"]).parse_next(input)?;
        ConsultRemainderDispositionShape::ShuffleLibrary
    } else {
        alt((
            (
                sequence_any_phrase(&[&["on"], &["onto"]]),
                opt(sequence_phrase(&["the"])),
                sequence_phrase(&["bottom", "of"]),
                opt(sequence_any_phrase(&[
                    &["your"],
                    &["their"],
                    &["that", "player's"],
                    &["that", "players"],
                ])),
                sequence_phrase(&["library", "in"]),
                opt(sequence_phrase(&["a"])),
                alt((
                    sequence_phrase(&["random", "order"]).value(LibraryBottomOrderAst::Random),
                    sequence_phrase(&["any", "order"]).value(LibraryBottomOrderAst::ChooserChooses),
                )),
            )
                .map(|(_, _, _, _, _, _, order)| {
                    ConsultRemainderDispositionShape::LibraryBottom(order)
                }),
            (
                sequence_phrase(&["into"]),
                opt(sequence_any_phrase(&[
                    &["your"],
                    &["their"],
                    &["that", "player's"],
                    &["that", "players"],
                ])),
                sequence_phrase(&["graveyard"]),
            )
                .value(ConsultRemainderDispositionShape::Graveyard),
        ))
        .parse_next(input)?
    };
    finish_sequence_words(input)?;
    Ok(disposition)
}

fn parse_remainder_shape(tokens: &[OwnedLexToken]) -> Option<ConsultRemainderDispositionShape> {
    let mut input = LexStream::new(trimmed(tokens));
    parse_remainder_shape_inner.parse_next(&mut input).ok()
}

fn split_terminal_remainder(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, ConsultRemainderDispositionShape)> {
    if let Some(remainder) = parse_remainder_shape(tokens) {
        return Some((Vec::new(), remainder));
    }
    let (remainder_at, remainder, trailing) =
        crate::runtime_backend::front_end::grammar::primitives::find_prefix(tokens, || {
            parse_remainder_shape_inner
        })?;
    if !trailing.is_empty() {
        return None;
    }
    Some((trimmed(&tokens[..remainder_at]).to_vec(), remainder))
}

pub(crate) fn parse_consult_disposition_sequence_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConsultDispositionSequenceShape> {
    let sentences = split_lexed_sentences(tokens);
    let consult_tokens = sentences.first().copied()?;
    let consult = parse_consult_traversal_shape(consult_tokens)?;
    let mut middle = Vec::new();
    if !consult.trailing_effect.is_empty() {
        middle.push(consult.trailing_effect);
    }
    middle.extend(
        sentences
            .iter()
            .skip(1)
            .map(|sentence| trimmed(sentence).to_vec()),
    );
    let terminal = middle.pop()?;
    let (stem, remainder) = split_terminal_remainder(&terminal)?;
    if !stem.is_empty() {
        middle.push(stem);
    }
    if middle.is_empty() {
        return None;
    }

    let middle = if middle.len() == 1 {
        if let Some(repeated) = parse_repeated_move_shape(&middle[0]) {
            ConsultMiddleShape::RepeatedMove(repeated)
        } else if let Some(matched) = parse_matched_move_shape(&middle[0]) {
            ConsultMiddleShape::MatchedMove(matched)
        } else {
            ConsultMiddleShape::Generic(middle)
        }
    } else {
        ConsultMiddleShape::Generic(middle)
    };
    Some(ConsultDispositionSequenceShape {
        consult_tokens: consult_tokens.to_vec(),
        middle,
        remainder,
    })
}

pub(crate) fn parse_reveal_repeated_disposition_sequence_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealRepeatedDispositionSequenceShape> {
    let sentences = split_lexed_sentences(tokens);
    let [reveal, disposition] = sentences.as_slice() else {
        return None;
    };
    let mut reveal_input = LexStream::new(reveal);
    seek_sequence_phrase(
        &mut reveal_input,
        &[&[
            "reveal", "that", "many", "cards", "from", "the", "top", "of", "your", "library",
        ]],
    )
    .ok()?;
    sequence_phrase(&[
        "reveal", "that", "many", "cards", "from", "the", "top", "of", "your", "library",
    ])
    .parse_next(&mut reveal_input)
    .ok()?;
    finish_sequence_words(&mut reveal_input).ok()?;

    let (middle, remainder) = split_terminal_remainder(disposition)?;
    let repeated = parse_repeated_move_shape(&middle)?;
    Some(RevealRepeatedDispositionSequenceShape {
        reveal_tokens: trimmed(reveal).to_vec(),
        repeated,
        remainder,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parse(raw: &str) -> ConsultDispositionSequenceShape {
        let tokens = lex_line(raw, 0).unwrap();
        parse_consult_disposition_sequence_shape(&tokens).expect(raw)
    }

    #[test]
    fn consult_matched_card_to_battlefield_then_bare_shuffle_is_typed() {
        let shape = parse(
            "They reveal cards from the top of their library until they reveal a permanent card that shares a card type with the sacrificed permanent, put that card onto the battlefield, then shuffle.",
        );
        assert!(matches!(
            shape.middle,
            ConsultMiddleShape::MatchedMove(ConsultMatchedMoveShape {
                selection: ConsultMoveSelectionShape::AllMatched,
                zone: Zone::Battlefield,
                controller_you: false,
            })
        ));
        assert_eq!(
            shape.remainder,
            ConsultRemainderDispositionShape::ShuffleLibrary
        );
    }

    #[test]
    fn parses_matched_moves_and_remainder_destinations() {
        let curse = parse(
            "That player reveals cards from the top of their library until they reveal a creature card. Put that card onto the battlefield under your control. That player puts the rest of the revealed cards into their graveyard.",
        );
        assert!(matches!(
            curse.middle,
            ConsultMiddleShape::MatchedMove(ConsultMatchedMoveShape {
                zone: Zone::Battlefield,
                controller_you: true,
                ..
            })
        ));
        assert_eq!(curse.remainder, ConsultRemainderDispositionShape::Graveyard);

        let fathom = parse(
            "Reveal cards from the top of your library until you reveal three nonland cards. Put the nonland cards revealed this way into your hand, then put the rest of the revealed cards on the bottom of your library in any order.",
        );
        assert!(matches!(
            fathom.middle,
            ConsultMiddleShape::MatchedMove(ConsultMatchedMoveShape {
                zone: Zone::Hand,
                ..
            })
        ));
        assert_eq!(
            fathom.remainder,
            ConsultRemainderDispositionShape::LibraryBottom(LibraryBottomOrderAst::ChooserChooses)
        );

        let synthetic = parse(
            "Reveal cards from the top of your library until you reveal that many creature cards, put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
        );
        assert_eq!(
            synthetic.remainder,
            ConsultRemainderDispositionShape::ShuffleLibrary
        );

        let iterated = parse(
            "Its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order.",
        );
        assert!(matches!(
            iterated.middle,
            ConsultMiddleShape::MatchedMove(ConsultMatchedMoveShape {
                selection: ConsultMoveSelectionShape::AllMatched,
                zone: Zone::Battlefield,
                controller_you: false,
            })
        ));
        assert_eq!(
            iterated.remainder,
            ConsultRemainderDispositionShape::LibraryBottom(LibraryBottomOrderAst::Random)
        );
    }

    #[test]
    fn parses_any_number_repeat_and_generic_middle_shapes() {
        let vivid = parse(
            "Reveal cards from the top of your library until you reveal X permanent cards, where X is the number of colors among permanents you control. Put any number of those permanent cards onto the battlefield, then put the rest of the revealed cards on the bottom of your library in a random order.",
        );
        assert!(matches!(
            vivid.middle,
            ConsultMiddleShape::MatchedMove(ConsultMatchedMoveShape {
                selection: ConsultMoveSelectionShape::AnyNumberOfMatched,
                ..
            })
        ));

        let glimpse_tokens = lex_line(
            "Shuffle all permanents you own into your library, then reveal that many cards from the top of your library. Put all non-Aura permanent cards revealed this way onto the battlefield, then do the same for Aura cards, then put the rest on the bottom of your library in a random order.",
            0,
        )
        .unwrap();
        let glimpse = parse_reveal_repeated_disposition_sequence_shape(&glimpse_tokens).unwrap();
        assert_eq!(glimpse.repeated.zone, Zone::Battlefield);

        let thought = parse(
            "Target opponent reveals cards from the top of their library until an artifact card or X cards are revealed, whichever comes first. If an artifact card is revealed this way, put it onto the battlefield under your control and sacrifice this artifact. Put the rest of the revealed cards into that player's graveyard.",
        );
        assert!(matches!(thought.middle, ConsultMiddleShape::Generic(_)));
        assert_eq!(
            thought.remainder,
            ConsultRemainderDispositionShape::Graveyard
        );
    }
}
