use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn filtered_characteristic_relations_survive_into_compiled_card_text() {
    let descendants = compiled_card_text("Descendants' Path");
    assert!(
        descendants.contains(
            "If it's a creature card that shares a creature type with a creature you control, you may cast it without paying its mana cost"
        ),
        "{descendants}"
    );

    let ringsight = compiled_card_text("Ringsight");
    assert!(
        ringsight.contains(
            "Search your library for a card that shares a color with a legendary creature you control, reveal it, put it into your hand, then shuffle"
        ),
        "{ringsight}"
    );

    let hiveheart = compiled_card_text("Hiveheart Shaman");
    assert!(
        hiveheart.contains(
            "search your library for a basic land card that doesn't share a land type with a land you control"
        ),
        "{hiveheart}"
    );
}

#[test]
fn tagged_characteristic_comparisons_survive_into_compiled_card_text() {
    let konda = compiled_card_text("Konda's Banner");
    assert!(
        konda.contains("can be attached only to a legendary creature"),
        "{konda}"
    );
    assert!(
        konda.contains("Creatures that share a color with equipped creature get +1/+1"),
        "{konda}"
    );
    assert!(
        konda.contains("Creatures that share a creature type with equipped creature get +1/+1"),
        "{konda}"
    );

    let resplendent = compiled_card_text("Resplendent Marshal");
    assert!(
        resplendent.contains(
            "each other creature you control that shares a creature type with the exiled card"
        ),
        "{resplendent}"
    );

    let thought_prison = compiled_card_text("Thought Prison");
    assert!(
        thought_prison.contains(
            "Whenever a player casts a spell that shares a color or mana value with the exiled card"
        ),
        "{thought_prison}"
    );
}

#[test]
fn convoked_creature_comparison_remains_scoped_to_the_creature_exception() {
    let everything = compiled_card_text("Everything Comes to Dust");
    assert!(
        everything.contains(
            "Exile all creatures except those that share a creature type with a creature that convoked this spell, all artifacts, and all enchantments"
        ),
        "{everything}"
    );
}
