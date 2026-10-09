//! "You can't cast this spell unless <condition>" carries a typed condition.
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::ability::AbilityKind;
use ironsmith::static_abilities::ThisSpellCastCondition;

const PROFT: &str = "Mana cost: {2}{B}\nType: Legendary Creature — Human Rogue\nPower/Toughness: 5/5\nThreshold — You can't cast this spell unless there are seven or more cards in your graveyard.\nMenace\n{B}, Discard this card: Target creature gets -3/-1 until end of turn.";

#[test]
fn proft_is_castable_only_with_threshold() {
    for definition in support::definitions("Proft, Sinister Mastermind", PROFT) {
        let kinds: Vec<_> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => static_ability.this_spell_cast_restriction_kind(),
                _ => None,
            })
            .collect();
        let [kind] = kinds.as_slice() else { panic!("{kinds:?}") };
        assert!(kind.timing.is_none());
        assert!(matches!(kind.condition, Some(ThisSpellCastCondition::Condition(_))), "{kind:?}");
    }
}

const RAKDOS: &str = "Mana cost: {B}{B}{R}{R}\nType: Legendary Creature — Demon\nPower/Toughness: 6/6\nYou can't cast Rakdos unless an opponent lost life this turn.\nFlying, trample\nCreature spells you cast cost {1} less to cast for each 1 life your opponents have lost this turn.";

#[test]
fn rakdos_short_name_is_a_self_reference_in_its_cast_prohibition() {
    for definition in support::definitions("Rakdos, Lord of Riots", RAKDOS) {
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(static_ability)
                if matches!(static_ability.this_spell_cast_restriction_kind(),
                    Some(kind) if matches!(kind.condition, Some(ThisSpellCastCondition::Condition(_)))))));
    }
}
