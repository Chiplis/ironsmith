#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn farmer_cotton_compacts_coordinated_x_token_counts() {
    let definition = parse_oracle_card_definition("Farmer Cotton");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "When this creature enters, create X 1/1 white Halfling creature tokens and X Food tokens."
    );
}
