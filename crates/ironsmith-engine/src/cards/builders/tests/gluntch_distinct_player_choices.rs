#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn gluntch_renders_the_exact_distinct_player_choice_sequence() {
    let oracle = "Flying\nAt the beginning of your end step, choose a player. They put two +1/+1 counters on a creature they control. Choose a second player to draw a card. Then choose a third player to create two Treasure tokens.";
    let definition = parse_oracle_card_definition("Gluntch, the Bestower");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("chosen_player_0")
            && debug.contains("chosen_player_1")
            && debug.contains("chosen_player_2")
            && debug.contains("excluded_tags")
            && debug.contains("DrawCardsEffect")
            && debug.contains("CreateTokenEffect"),
        "the compiled trigger must preserve three distinct tagged players and both linked actions: {debug}"
    );
}
