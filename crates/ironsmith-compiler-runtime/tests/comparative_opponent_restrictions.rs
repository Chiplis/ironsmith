//! Ward of Bones: each opponent who controls more permanents of a type than
//! you can't cast spells of that type (lands: can't play lands). Source-
//! authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const WARD_OF_BONES: &str = "Mana cost: {6}\nType: Artifact\nEach opponent who controls more creatures than you can't cast creature spells. The same is true for artifacts and enchantments.\nEach opponent who controls more lands than you can't play lands.";

#[test]
fn ward_of_bones_lowers_four_comparative_restrictions() {
    for definition in support::definitions("Ward of Bones", WARD_OF_BONES) {
        let debug = format!("{:?}", definition.abilities);
        assert_eq!(debug.matches("OpponentWithMoreControlledObjectsThan").count(), 4, "{debug}");
        assert_eq!(debug.matches("CastSpellsMatching").count(), 3, "{debug}");
        assert!(debug.contains("PlayLandsMatching"), "{debug}");
        for card_type in ["Creature", "Artifact", "Enchantment", "Land"] {
            assert!(debug.contains(card_type), "{card_type}: {debug}");
        }
    }
}
