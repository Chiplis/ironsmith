//! Moonhold: two restrictions on one target, each gated on the mana spent to
//! cast the spell. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::CantEffect;

const MOONHOLD: &str = "Mana cost: {2}{R/W}\nType: Instant\nTarget player can't play lands this turn if {R} was spent to cast this spell and can't cast creature spells this turn if {W} was spent to cast this spell. (Do both if {R}{W} was spent.)";

#[test]
fn moonhold_gates_each_restriction_on_its_own_mana_color() {
    for definition in support::definitions("Moonhold", MOONHOLD) {
        let cants = support::find_all::<CantEffect>(&definition);
        assert_eq!(cants.len(), 2, "{cants:#?}");
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("PlayLandsMatching"), "{debug}");
        assert!(debug.contains("CastSpellsMatching"), "{debug}");
        assert_eq!(debug.matches("ManaSpent").count(), 2, "one gate per restriction: {debug}");
    }
}
