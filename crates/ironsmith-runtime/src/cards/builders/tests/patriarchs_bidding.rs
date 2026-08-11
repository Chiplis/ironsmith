#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn patriarchs_bidding_returns_cards_of_every_type_chosen_this_way() {
    let name = "Patriarch's Bidding";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Patriarch's Bidding should have a spell program");
    let debug = format!("{program:#?}");

    assert!(
        debug.contains("ChooseCreatureTypeEffect")
            && debug.contains("ReturnAllToBattlefieldEffect")
            && debug.contains("chosen_creature_type: true")
            && debug.contains("chosen_type_this_way: true"),
        "the return filter must consume the full set of player-chosen creature types: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert_eq!(compiled.join("\n"), oracle.as_str());
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}
