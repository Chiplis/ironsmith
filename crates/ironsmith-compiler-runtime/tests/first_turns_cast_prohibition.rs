//! cf8 p10: "You can't cast this spell during your first, second, or third
//! turns of the game." Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::ability::AbilityKind;
use ironsmith::static_abilities::ThisSpellCastRestrictionKind;

const SERRA_AVENGER: &str = "Mana cost: {W}{W}\nType: Creature — Angel\nPower/Toughness: 3/3\nYou can't cast Serra Avenger during your first, second, or third turns of the game.\nFlying, vigilance";
const JACE_REAWAKENED: &str = "Mana cost: {U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 3\nYou can't cast Jace Reawakened during your first, second, or third turns of the game.\n+1: Draw a card, then discard a card.\n+1: You may exile a nonland card with mana value 3 or less from your hand. If you do, it becomes plotted.\n−6: Until end of turn, whenever you cast a spell, copy it. You may choose new targets for the copy.";
const SPIDER_MAN_2099: &str = "Mana cost: {U}{R}\nType: Legendary Creature — Spider Human Hero\nPower/Toughness: 2/3\nFrom the Future — You can't cast Spider-Man 2099 during your first, second, or third turns of the game.\nDouble strike, vigilance\nAt the beginning of your end step, if you've played a land or cast a spell this turn from anywhere other than your hand, Spider-Man 2099 deals damage equal to his power to any target.";

fn restriction_kinds(name: &str, text: &str) -> Vec<ThisSpellCastRestrictionKind> {
    support::definitions(name, text)
        .iter()
        .flat_map(|definition| {
            definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => static_ability.this_spell_cast_restriction_kind(),
                _ => None,
            })
        })
        .collect()
}

#[test]
fn first_three_turns_prohibition_is_a_typed_cast_condition() {
    for (name, text) in [
        ("Serra Avenger", SERRA_AVENGER),
        ("Jace Reawakened", JACE_REAWAKENED),
        ("Spider-Man 2099", SPIDER_MAN_2099),
    ] {
        let kinds = restriction_kinds(name, text);
        assert_eq!(kinds.len(), 2, "{name}: one restriction per route");
        for kind in kinds {
            assert_eq!(
                kind,
                ThisSpellCastRestrictionKind::timing(
                    ironsmith_core::ThisSpellCastTiming::NotDuringYourFirstTurns(3)
                ),
                "{name}"
            );
        }
    }
}
