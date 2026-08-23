#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use crate::runtime_display::canonical_compiled_lines;

fn assert_compiled(name: &str, expected: &str) {
    let definition = parse_oracle_card_definition(name);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        expected,
        "{definition:#?}"
    );
}

#[test]
fn combat_research_has_no_internal_tag_surface() {
    assert_compiled(
        "Combat Research",
        "Enchant creature\nEnchanted creature has \"Whenever this creature deals combat damage to a player, draw a card.\"\nAs long as enchanted creature is legendary, it gets +1/+1 and has ward {1}. (Whenever enchanted creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)",
    );
}

#[test]
fn chaos_defiler_has_no_internal_tag_surface() {
    assert_compiled(
        "Chaos Defiler",
        "Trample\nBattle Cannon — When this creature enters or dies, for each opponent, choose a nonland permanent that player controls. Destroy one of them chosen at random.",
    );
}
