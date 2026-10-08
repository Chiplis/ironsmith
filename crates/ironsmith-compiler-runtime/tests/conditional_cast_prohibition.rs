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
