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
