//! Null Chamber: you and an opponent each name a nonbasic card name; spells
//! and lands with either name can't be cast or played. Source-authored, UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const NULL_CHAMBER: &str = "Mana cost: {3}{W}\nType: World Enchantment\nAs this enchantment enters, you and an opponent each choose a card name other than a basic land card name.\nSpells with the chosen names can't be cast and lands with the chosen names can't be played.";

#[test]
fn null_chamber_records_two_names_and_prohibits_both() {
    for definition in support::definitions("Null Chamber", NULL_CHAMBER) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("opponent_also_chooses: true"), "{debug}");
        assert!(debug.contains("exclude_basic_land_names: true"), "{debug}");
        assert!(debug.contains("CastSpellsMatching"), "{debug}");
        assert!(debug.contains("PlayLandsMatching"), "{debug}");
        assert!(debug.contains("{chosen name}"), "{debug}");
    }
}
