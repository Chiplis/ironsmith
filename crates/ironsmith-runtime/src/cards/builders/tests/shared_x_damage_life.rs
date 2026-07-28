#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn zenith_flare_compiles_to_one_shared_x_definition() {
    let definition = parse_oracle_card_definition("Zenith Flare");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Zenith Flare deals X damage to any target and you gain X life, where X is the number of cards with a cycling ability in your graveyard."
    );
}
