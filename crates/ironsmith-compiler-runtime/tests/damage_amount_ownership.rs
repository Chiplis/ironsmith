//! Damage amounts and recipients owned by their complete readings: a
//! relative aggregate ("the total mana value of other spells you've cast
//! this turn") over a fragmentary cost-modifier reading; "each creature and
//! each planeswalker" as one union recipient set; and a "twice that much ...
//! instead" replacement inside a reflexive trigger's body (CR 614.1a).
//! Source-authored, UNRUN.
use ironsmith::effects::ReflexiveTriggerEffect;

#[path = "cf8_p08/support.rs"]
mod support;

const CALL_FORTH: &str = "Mana cost: {5}{R}{R}{R}\nType: Sorcery\nCascade, cascade\nCall Forth the Tempest deals damage to each creature your opponents control equal to the total mana value of other spells you've cast this turn.";
const CORPSE_EXPLOSION: &str = "Mana cost: {1}{B}{R}\nType: Sorcery\nAs an additional cost to cast this spell, exile a creature card from your graveyard.\nCorpse Explosion deals damage equal to the exiled card's power to each creature and each planeswalker.";
const SURTLAND_FLINGER: &str = "Mana cost: {3}{R}{R}\nType: Creature — Giant Berserker\nPower/Toughness: 4/6\nWhenever this creature attacks, you may sacrifice another creature. When you do, this creature deals damage equal to the sacrificed creature's power to any target. If the sacrificed creature was a Giant, this creature deals twice that much damage instead.";

#[test]
fn total_mana_value_of_spells_cast_this_turn_is_the_amount() {
    for definition in support::definitions("Call Forth the Tempest", CALL_FORTH) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("TotalManaValueOfSpellsCastThisTurn"), "{debug}");
    }
}

#[test]
fn creatures_and_planeswalkers_are_one_recipient_union() {
    for definition in support::definitions("Corpse Explosion", CORPSE_EXPLOSION) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Creature") && debug.contains("Planeswalker"), "{debug}");
        assert!(debug.contains("any_of: ["), "{debug}");
    }
}

#[test]
fn giant_doubling_lives_inside_the_reflexive_body() {
    for definition in support::definitions("Surtland Flinger", SURTLAND_FLINGER) {
        let reflexive = support::find_all::<ReflexiveTriggerEffect>(&definition);
        assert_eq!(reflexive.len(), 1);
        let inner = format!("{:?}", reflexive[0]);
        assert!(inner.contains("Scaled("), "{inner}");
        assert!(inner.contains("Giant"), "{inner}");
    }
}
