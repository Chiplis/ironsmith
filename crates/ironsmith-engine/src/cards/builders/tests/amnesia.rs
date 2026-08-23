#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn reveal_hand_then_filtered_complete_discard_keeps_the_filter() {
    let definition = parse_oracle_card_definition("Amnesia");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Target player reveals their hand and discards all nonland cards."
    );
}
