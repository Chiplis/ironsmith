//! Look/reveal the top N, cast a spell from among them with a dynamic
//! mana-value cap without paying its mana cost, put the rest on the bottom in
//! a random order (CR 601.2, 608.2c). Covers the trailing cap word order
//! ("from among them with mana value less than or equal to <value>") and the
//! "cards revealed this way" collection reference. Source-authored, UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const COSMIC_CUBE: &str = "Mana cost: {7}\nType: Legendary Artifact\nWard {2}\nWhenever you attack, look at the top six cards of your library. You may cast a spell from among them with mana value less than or equal to the greatest power among attacking creatures you control without paying its mana cost. Put the rest on the bottom of your library in a random order.";
const SUNBIRDS_INVOCATION: &str = "Mana cost: {5}{R}\nType: Enchantment\nWhenever you cast a spell from your hand, reveal the top X cards of your library, where X is that spell's mana value. You may cast a spell with mana value X or less from among cards revealed this way without paying its mana cost. Put the rest on the bottom of your library in a random order.";

#[test]
fn cosmic_cube_caps_the_free_cast_by_the_greatest_attacking_power() {
    for definition in support::definitions("Cosmic Cube", COSMIC_CUBE) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("GreatestPower"), "{debug}");
        assert!(debug.contains("LessThanOrEqualExpr"), "{debug}");
        assert!(debug.contains("attacking: true"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        assert!(debug.contains("Bottom"), "the rest goes to the bottom: {debug}");
        assert!(debug.contains("Ward"), "{debug}");
    }
}

#[test]
fn sunbirds_invocation_reveals_x_and_caps_the_free_cast_at_x() {
    for definition in support::definitions("Sunbird's Invocation", SUNBIRDS_INVOCATION) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        assert!(debug.contains("Reveal"), "the top X are revealed: {debug}");
        assert!(debug.contains("mana_value: Some("), "the cast is capped: {debug}");
        assert!(debug.contains("Bottom"), "{debug}");
    }
}
