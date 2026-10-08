//! "Any number of target opponents each discard their hands, then draw seven
//! cards." (Wheel and Deal): the plural possessive names each targeted
//! player's own hand. Source-authored, deliberately unrun.

#[path = "p02_line_families/compile.rs"]
mod compile;

const WHEEL_AND_DEAL: &str = "Mana cost: {3}{U}\nType: Instant\nAny number of target opponents each discard their hands, then draw seven cards.\nDraw a card.";

#[test]
fn wheel_and_deal_wheels_each_targeted_opponent_then_cantrips() {
    for definition in compile::compile_both("Wheel and Deal", WHEEL_AND_DEAL) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Discard"), "{debug}");
        assert!(debug.contains("Opponent"), "{debug}");
        assert!(debug.contains("min: 0, max: None"), "any number of targets: {debug}");
        assert!(debug.matches("DrawCardsEffect").count() >= 2, "{debug}");
    }
}
