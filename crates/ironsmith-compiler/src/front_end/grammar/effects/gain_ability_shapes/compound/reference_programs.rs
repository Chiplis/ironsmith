use super::*;

pub(super) fn parse_attached_and_related_subject(
    input: &mut LexStream<'_>,
) -> WResult<AttachedReferenceSubject> {
    let subject = alt((
        primitives::phrase(&["enchanted", "creature"])
            .value(AttachedReferenceSubject::EnchantedCreature),
        primitives::phrase(&["equipped", "creature"])
            .value(AttachedReferenceSubject::EquippedCreature),
    ))
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(alt((primitives::kw("each"), primitives::kw("all")))).parse_next(input)?;
    primitives::phrase(&["other", "creatures"]).parse_next(input)?;
    opt(primitives::kw("that")).parse_next(input)?;
    alt((primitives::kw("share"), primitives::kw("shares"))).parse_next(input)?;
    primitives::phrase(&["a", "creature", "type", "with", "it"]).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(subject)
}
