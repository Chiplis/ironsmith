//! "Can't be equipped" / "can't be enchanted by other Auras".
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const CARDS: &[(&str, &str, &str)] = &[
    ("Goblin Brawler", "Mana cost: {2}{R}\nType: Creature — Goblin Warrior\nPower/Toughness: 2/2\nFirst strike\nThis creature can't be equipped.", "Equipment"),
    ("Anti-Magic Aura", "Mana cost: {2}{U}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't be the target of spells and can't be enchanted by other Auras.", "Aura"),
    ("Consecrate Land", "Mana cost: {W}\nType: Enchantment — Aura\nEnchant land\nEnchanted land has indestructible and can't be enchanted by other Auras.", "Aura"),
];

#[test]
fn attachment_prohibition_names_its_attachment_kind() {
    for (name, text, kind) in CARDS {
        for definition in support::definitions(name, text) {
            let debug = format!("{:?}", definition.abilities);
            assert!(debug.contains("BeAttachedBy"), "{name}: {debug}");
            assert!(debug.contains(kind), "{name}");
            if *kind == "Aura" {
                assert!(debug.contains("other: true"), "{name}: the prohibiting Aura itself stays attached");
            }
        }
    }
}
