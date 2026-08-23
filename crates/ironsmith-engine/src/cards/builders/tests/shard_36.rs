use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

#[test]
pub(super) fn intervening_if_search_cards_keep_the_complete_search_effect() {
    for (name, gate, search, destination) in [
        (
            "Gigantiform",
            "if it was kicked",
            "you may search your library for a card named",
            "put it onto the battlefield, then shuffle",
        ),
        (
            "Lost Auramancers",
            "if it had no time counters on it",
            "you may search your library for an enchantment card",
            "put it onto the battlefield, then shuffle",
        ),
        (
            "Sprouting Goblin",
            "if it was kicked",
            "search your library for a land card with a basic land type",
            "put it into your hand, then shuffle",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();

        assert!(compiled.contains(gate), "{name}: {compiled}");
        assert!(compiled.contains(search), "{name}: {compiled}");
        assert!(compiled.contains(destination), "{name}: {compiled}");
        assert!(
            !compiled.contains("return it to the battlefield")
                && !compiled.contains("return it to your hand"),
            "{name}'s search action must not be misread as a zone-return predicate: {compiled}"
        );
    }
}

#[test]
pub(super) fn cryptic_gateway_requires_and_renders_each_tapped_creatures_subtype() {
    assert_oracle_card_parses_strict("Cryptic Gateway");
    let definition = parse_oracle_card_definition("Cryptic Gateway");
    let debug = format!("{definition:#?}");
    let compiled = compiled_text_lines(&definition)
        .join("\n")
        .to_ascii_lowercase();

    assert!(debug.contains("tap_cost_0"), "{debug}");
    assert!(debug.contains("SharesSubtypeWithEachTagged"), "{debug}");
    assert!(
        compiled.contains("shares a creature type with each creature tapped this way"),
        "{compiled}"
    );
    assert!(
        compiled.contains("creature card") && compiled.contains("from your hand"),
        "{compiled}"
    );
    assert!(
        !compiled.contains("tapped creature card from your hand"),
        "the cost-reference adjective must not make the hand card tapped: {compiled}"
    );
}
