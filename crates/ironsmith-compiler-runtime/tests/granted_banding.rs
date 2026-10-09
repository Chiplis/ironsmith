//! A granted or conditional banding ("Enchanted creature has banding",
//! "... have banding", "has banding as long as ...") lowers to the same static
//! keyword the printed one does (CR 702.22). Frozen complete bodies;
//! source-authored and UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const BODIES: &[(&str, &str)] = &[
    ("Cooperation", "Mana cost: {2}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has banding."),
    ("Dire Wolves", "Mana cost: {2}{G}\nType: Creature — Wolf\nPower/Toughness: 2/2\nThis creature has banding as long as you control a Plains."),
    ("Fortified Area", "Mana cost: {1}{W}{W}\nType: Enchantment\nWall creatures you control get +1/+0 and have banding."),
];

#[test]
fn granted_banding_is_the_static_banding_keyword() {
    for (name, body) in BODIES {
        for definition in support::definitions(name, body) {
            let debug = format!("{:?}", definition.abilities);
            assert!(debug.contains("Banding"), "{name}: {debug}");
            let text = support::rendered(&definition).to_ascii_lowercase();
            assert!(text.contains("banding"), "{name}: {text}");
        }
    }
}

#[test]
fn dire_wolves_banding_is_conditioned_on_a_plains() {
    for definition in support::definitions("Dire Wolves", BODIES[1].1) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("Plains"), "{debug}");
    }
}
