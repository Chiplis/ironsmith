use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CantBeRegeneratedSubject {
    It,
    They,
    CreatureDestroyedThisWay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeRegeneratedFollowupShape {
    pub subject: CantBeRegeneratedSubject,
    pub this_turn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageRegenerationExileGate {
    DamagedObjectIsCreature,
    ThisSpellWasKicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRegenerationExileFollowupShape {
    pub gate: DamageRegenerationExileGate,
}

fn regeneration_subject<'a>(input: &mut LexStream<'a>) -> WResult<CantBeRegeneratedSubject> {
    alt((
        primitives::kw("it").value(CantBeRegeneratedSubject::It),
        primitives::kw("they").value(CantBeRegeneratedSubject::They),
        primitives::phrase(&["those", "creatures"]).value(CantBeRegeneratedSubject::They),
        alt((
            primitives::phrase(&["creature", "destroyed", "this", "way"]),
            primitives::phrase(&["creatures", "destroyed", "this", "way"]),
            primitives::phrase(&["a", "creature", "destroyed", "this", "way"]),
        ))
        .value(CantBeRegeneratedSubject::CreatureDestroyedThisWay),
    ))
    .parse_next(input)
}

fn cant<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("cant"),
        primitives::kw("can't"),
        primitives::kw("cannot"),
    ))
    .void()
    .parse_next(input)
}

fn parse_cant_be_regenerated_followup_lexed<'a>(
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
    primitives::parse_all(
        tokens,
        parse_cant_be_regenerated_followup_lexed,
        "can't-be-regenerated followup",
    )
    .ok()
}

fn damage_regeneration_exile_gate<'a>(
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

fn damage_regeneration_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
    ))
    .parse_next(input)
}

fn parse_damage_regeneration_exile_followup_lexed<'a>(
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
    primitives::parse_all(
        tokens,
        parse_damage_regeneration_exile_followup_lexed,
        "damage regeneration/exile followup",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_pronoun_and_destroyed_this_way_regeneration_followups() {
        let they = lex_line("They can't be regenerated.", 0).unwrap();
        assert_eq!(
            parse_cant_be_regenerated_followup(&they),
            Some(CantBeRegeneratedFollowupShape {
                subject: CantBeRegeneratedSubject::They,
                this_turn: false,
            })
        );

        let those_creatures = lex_line("Those creatures can't be regenerated.", 0).unwrap();
        assert_eq!(
            parse_cant_be_regenerated_followup(&those_creatures),
            Some(CantBeRegeneratedFollowupShape {
                subject: CantBeRegeneratedSubject::They,
                this_turn: false,
            })
        );

        let this_turn = lex_line(
            "A creature destroyed this way cannot be regenerated this turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_cant_be_regenerated_followup(&this_turn),
            Some(CantBeRegeneratedFollowupShape {
                subject: CantBeRegeneratedSubject::CreatureDestroyedThisWay,
                this_turn: true,
            })
        );
    }

    #[test]
    fn parses_compound_damage_regeneration_exile_gates() {
        let creature_gate = lex_line(
            "If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_damage_regeneration_exile_followup(&creature_gate),
            Some(DamageRegenerationExileFollowupShape {
                gate: DamageRegenerationExileGate::DamagedObjectIsCreature,
            })
        );

        let kicked_gate = lex_line(
            "If this spell was kicked, that creature can't be regenerated this turn and if it would die this turn, exile it instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_damage_regeneration_exile_followup(&kicked_gate),
            Some(DamageRegenerationExileFollowupShape {
                gate: DamageRegenerationExileGate::ThisSpellWasKicked,
            })
        );
    }
}
