//! Already-on-main proof for two conditional self cost reductions (merged
//! source from the Oct 8 repair PRs). Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

use ironsmith::ability::AbilityKind;

#[test]
fn two_conditional_reductions_are_two_static_abilities() {
    for name in ["Assassin's Ink", "Geistlight Snare"] {
        for definition in support::definitions(name) {
            let reductions = definition
                .abilities
                .iter()
                .filter(|ability| match &ability.kind {
                    AbilityKind::Static(ability) => format!("{ability:?}").contains("ThisSpellCostReduction"),
                    _ => false,
                })
                .count();
            assert_eq!(reductions, 2, "{name}");
        }
    }
}
