use super::*;
use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::leaf::LeafDurationPhrase;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReciprocalCreatureControlSequence {
    pub(crate) your_creatures: ObjectFilter,
    pub(crate) target_player_creatures: ObjectFilter,
    pub(crate) duration: Until,
    pub(crate) untap: bool,
    pub(crate) grant_haste: bool,
}

pub(crate) fn parse_reciprocal_creature_control_sequence_tokens(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<ReciprocalCreatureControlSequence>, CardTextError> {
    let Some(duration) = primitives::parse_all_or_none(
        first,
        parse_reciprocal_creature_control_head_lexed,
        "reciprocal-creature-control-head",
    )?
    else {
        return Ok(None);
    };
    if primitives::parse_all_or_none(
        second,
        parse_reciprocal_creature_control_untap_lexed,
        "reciprocal-creature-control-untap",
    )?
    .is_none()
    {
        return Ok(None);
    }
    if primitives::parse_all_or_none(
        third,
        parse_reciprocal_creature_control_haste_lexed,
        "reciprocal-creature-control-haste",
    )?
    .is_none()
    {
        return Ok(None);
    }

    let target_player = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
    Ok(Some(ReciprocalCreatureControlSequence {
        your_creatures: ObjectFilter::creature().you_control(),
        target_player_creatures: ObjectFilter::creature().controlled_by(target_player),
        duration,
        untap: true,
        grant_haste: true,
    }))
}

fn parse_reciprocal_creature_control_head_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<Until, ErrMode<ContextError>> {
    primitives::phrase(&[
        "you",
        "and",
        "target",
        "opponent",
        "each",
        "gain",
        "control",
        "of",
        "all",
        "creatures",
        "the",
        "other",
        "controls",
    ])
    .parse_next(input)?;
    let duration = super::super::leaf::parse_leaf_duration_phrase_lexed.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    match duration {
        LeafDurationPhrase::UntilEndOfTurn => Ok(Until::EndOfTurn),
        _ => Err(primitives::backtrack_err(
            "reciprocal creature control duration",
            "until end of turn",
        )),
    }
}

fn parse_reciprocal_creature_control_untap_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["untap", "those", "creatures"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_reciprocal_creature_control_haste_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["those", "creatures", "gain", "haste"]).parse_next(input)?;
    let duration = super::super::leaf::parse_leaf_duration_phrase_lexed.parse_next(input)?;
    match duration {
        LeafDurationPhrase::UntilEndOfTurn => {
            primitives::sentence_end().parse_next(input)?;
            Ok(())
        }
        _ => Err(primitives::backtrack_err(
            "reciprocal creature control haste duration",
            "until end of turn",
        )),
    }
}
