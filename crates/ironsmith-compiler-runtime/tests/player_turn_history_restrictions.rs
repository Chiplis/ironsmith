//! Angelic Arbiter: restrictions on each opponent by this turn's history
//! ("who cast a spell this turn" / "who attacked with a creature this turn").
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const ANGELIC_ARBITER: &str = "Mana cost: {5}{W}{W}\nType: Creature — Angel\nPower/Toughness: 5/6\nFlying\nEach opponent who cast a spell this turn can't attack with creatures.\nEach opponent who attacked with a creature this turn can't cast spells.";

#[test]
fn angelic_arbiter_restricts_opponents_by_turn_history() {
    for definition in support::definitions("Angelic Arbiter", ANGELIC_ARBITER) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("TurnHistory(CastSpell)"), "{debug}");
        assert!(debug.contains("TurnHistory(AttackedWithCreature)"), "{debug}");
        assert!(debug.contains("Attack("), "{debug}");
        assert!(debug.contains("CastSpellsMatching"), "{debug}");
    }
}
