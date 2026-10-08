//! "Enchanted creature can't attack, block, or crew Vehicles."
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const CARDS: &[(&str, &str, bool)] = &[
    ("Revoke Privileges", "Mana cost: {2}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't attack, block, or crew Vehicles.", false),
    ("Bound in Gold", "Mana cost: {2}{W}\nType: Enchantment — Aura\nEnchant permanent\nEnchanted permanent can't attack, block, or crew Vehicles, and its activated abilities can't be activated unless they're mana abilities.", true),
    ("Intercessor's Arrest", "Mana cost: {2}{W}\nType: Enchantment — Aura\nEnchant permanent\nEnchanted permanent can't attack, block, or crew Vehicles. Its activated abilities can't be activated unless they're mana abilities.", true),
];

#[test]
fn attached_prohibition_covers_attack_block_and_crew() {
    for (name, text, activation) in CARDS {
        for definition in support::definitions(name, text) {
            let debug = format!("{:?}", definition.abilities);
            assert!(debug.contains("AttackBlockOrCrew"), "{name}: {debug}");
            assert_eq!(debug.contains("ActivateNonManaAbilitiesOf"), *activation, "{name}");
        }
    }
}
