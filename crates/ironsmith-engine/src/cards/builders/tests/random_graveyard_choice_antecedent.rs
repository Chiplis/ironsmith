#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn tariel_keeps_the_randomly_chosen_card_as_that_card() {
    let definition = parse_oracle_card_definition("Tariel, Reckoner of Souls");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Flying, vigilance",
            "{T}: Choose a creature card at random from target opponent's graveyard. Put that card onto the battlefield under your control.",
        ],
        "{definition:#?}"
    );
}
