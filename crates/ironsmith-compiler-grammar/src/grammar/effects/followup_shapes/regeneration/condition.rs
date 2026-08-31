use super::*;

pub(super) fn parse_cant_be_regenerated_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantBeRegeneratedFollowupShape> {
    let subject = regeneration_subject.parse_next(input)?;
    cant.parse_next(input)?;
    primitives::phrase(&["be", "regenerated"]).parse_next(input)?;
    let this_turn = opt(primitives::phrase(&["this", "turn"]))
        .parse_next(input)?
        .is_some();
    primitives::sentence_end().parse_next(input)?;
    Ok(CantBeRegeneratedFollowupShape { subject, this_turn })
}

pub fn parse_cant_be_regenerated_followup(
    tokens: &[OwnedLexToken],
) -> Option<CantBeRegeneratedFollowupShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_cant_be_regenerated_followup_lexed,
        "can't-be-regenerated followup",
    )
}
