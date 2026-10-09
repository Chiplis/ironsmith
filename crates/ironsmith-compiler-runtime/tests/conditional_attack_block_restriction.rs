//! "This can't attack or block unless <general condition>" uses the shared
//! static condition grammar. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const BONTU: &str = "Mana cost: {2}{B}\nType: Legendary Creature — God\nPower/Toughness: 4/6\nMenace, indestructible\nBontu can't attack or block unless a creature died under your control this turn.\n{1}{B}, Sacrifice another creature: Scry 1. Each opponent loses 1 life and you gain 1 life.";

#[test]
fn bontu_attack_and_block_are_gated_by_a_typed_condition() {
    for definition in support::definitions("Bontu the Glorified", BONTU) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("AttackOrBlock"), "{debug}");
        assert!(debug.contains("Died") || debug.contains("died"), "the death-history condition survives: {debug}");
    }
}
