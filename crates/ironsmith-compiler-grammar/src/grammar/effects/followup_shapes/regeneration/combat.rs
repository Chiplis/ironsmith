use super::*;

pub(super) fn damage_regeneration_exile_gate<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageRegenerationExileGate> {
    alt((
        (
            primitives::kw("if"),
            alt((
                primitives::kw("it's"),
                primitives::kw("it’s"),
                primitives::kw("its"),
            )),
            primitives::phrase(&["a", "creature"]),
            primitives::comma(),
        )
            .value(DamageRegenerationExileGate::DamagedObjectIsCreature),
        (
            primitives::phrase(&["if", "this", "spell", "was", "kicked"]),
            primitives::comma(),
        )
            .value(DamageRegenerationExileGate::ThisSpellWasKicked),
    ))
    .parse_next(input)
}

pub(super) fn damage_regeneration_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
    ))
    .parse_next(input)
}

pub(super) fn parse_damage_regeneration_exile_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageRegenerationExileFollowupShape> {
    let gate = damage_regeneration_exile_gate.parse_next(input)?;
    damage_regeneration_subject.parse_next(input)?;
    cant.parse_next(input)?;
    primitives::phrase(&["be", "regenerated", "this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    primitives::phrase(&["if", "it", "would", "die", "this", "turn"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["exile", "it", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DamageRegenerationExileFollowupShape { gate })
}

pub fn parse_damage_regeneration_exile_followup(
    tokens: &[OwnedLexToken],
) -> Option<DamageRegenerationExileFollowupShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_damage_regeneration_exile_followup_lexed,
        "damage regeneration/exile followup",
    )
}
