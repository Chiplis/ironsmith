//! Absorb N (CR 702.64a) granted by Lymph Sliver: "If a source would deal
//! damage to this creature, prevent N of that damage." on each Sliver.
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const LYMPH_SLIVER: &str = "Mana cost: {4}{W}\nType: Creature — Sliver\nPower/Toughness: 3/3\nAll Sliver creatures have absorb 1. (If a source would deal damage to a Sliver, prevent 1 of that damage.)";

#[test]
fn lymph_sliver_grants_absorb_one_to_slivers() {
    for definition in compile::compile_both("Lymph Sliver", LYMPH_SLIVER) {
        let statics: Vec<String> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect();
        assert_eq!(statics.len(), 1, "{statics:?}");
        assert!(statics[0].contains("Sliver"), "{}", statics[0]);
        assert!(statics[0].contains("PreventMatchingDamage"), "{}", statics[0]);
        assert!(statics[0].contains("Amount(Fixed(1))"), "{}", statics[0]);
    }
}
