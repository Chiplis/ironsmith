#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn staff_of_eden_keeps_correlated_controller_and_owner_scope() {
    let oracle = "When Staff of Eden enters, put target legendary permanent card not named Staff of Eden, Vault's Key from a graveyard onto the battlefield under your control.\n{T}: Draw a card for each permanent you control but don't own.";
    let definition = parse_oracle_card_definition("Staff of Eden, Vault's Key");
    let compiled = canonical_compiled_lines(&definition).join("\n");
    let debug = format!("{definition:#?}");

    assert_eq!(compiled, oracle, "{debug}");
    assert!(
        debug.contains("controller: Some(\n")
            && debug.contains("owner: Some(\n")
            && debug.contains("NotYou"),
        "the count must retain both controller and inverse-owner scope: {debug}"
    );
}
