#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn osseous_sticktwister_preserves_each_opponents_optional_choice_and_correlated_failure() {
    let definition = parse_oracle_card_definition("Osseous Sticktwister");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Lifelink\nDelirium — At the beginning of your end step, if there are four or more card types among cards in your graveyard, each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way."
    );

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("VillainousChoiceEffect"), "{debug}");
    assert!(debug.contains("DidNotHappen"), "{debug}");
}
