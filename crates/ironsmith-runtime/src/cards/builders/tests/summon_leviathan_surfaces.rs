#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn summon_leviathan_keeps_its_shared_article_and_serial_subtype_list() {
    let name = "Summon: Leviathan";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("serial_or_list: true") && debug.contains("shared_indefinite_article: true"),
        "the attack filter must retain its authored union surface: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert!(
        compiled.iter().any(|line| {
            line
                == "I — Return each creature that isn't a Kraken, Leviathan, Merfolk, Octopus, or Serpent to its owner's hand."
        }),
        "the first chapter must keep its each/relative/owner surface: {compiled:#?}"
    );
    assert!(
        compiled.iter().any(|line| {
            line
                == "II, III — Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card."
        }),
        "the chapter trigger must keep the exact shared-article list: {compiled:#?}"
    );

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
