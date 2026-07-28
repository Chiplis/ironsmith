#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn target_payment_followed_by_linked_life_exchange_keeps_leading_unless() {
    let definition = parse_oracle_card_definition("Rhystic Syphon");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Unless target player pays {3}, that player loses 5 life and you gain 5 life."
    );
}
