#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn second_sunrise_preserves_historical_graveyard_return_exactly() {
    let definition = parse_oracle_card_definition("Second Sunrise");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Each player returns to the battlefield all artifact, creature, enchantment, and land cards in their graveyard that were put there from the battlefield this turn.".to_string(),
        ]
    );
}
