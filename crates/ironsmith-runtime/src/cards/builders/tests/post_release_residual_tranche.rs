#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_exact(name: &str, oracle: &str) {
    let definition = parse_oracle_card_definition(name);
    let compiled = canonical_compiled_lines(&definition).join("\n");
    assert_eq!(compiled, oracle, "{name}: {definition:#?}");
}

#[test]
fn kamahls_summons_renders_exactly() {
    assert_exact(
        "Kamahl's Summons",
        "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way.",
    );
}

#[test]
fn part_in_friendship_renders_exactly() {
    assert_exact(
        "Part in Friendship",
        "Whenever a nontoken creature you control dies, reveal cards from the top of your library until you reveal a creature card. If its mana value is less than or equal to the number of lands you control, put it onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your library in a random order. This ability triggers only once each turn.",
    );
}

#[test]
fn veiling_oddity_renders_exactly() {
    assert_exact(
        "Veiling Oddity",
        "Suspend 4—{1}{U}\nWhen the last time counter is removed from this card while it's exiled, creatures can't be blocked this turn.",
    );
}

#[test]
fn descendants_path_renders_exactly() {
    assert_exact(
        "Descendants' Path",
        "At the beginning of your upkeep, reveal the top card of your library. If it's a creature card that shares a creature type with a creature you control, you may cast it without paying its mana cost. If you don't cast it, put it on the bottom of your library.",
    );
}
