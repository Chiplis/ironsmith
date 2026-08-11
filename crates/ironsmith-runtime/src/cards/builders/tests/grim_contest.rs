#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn grim_contest_keeps_one_linked_target_pair_and_reciprocal_toughness_damage() {
    let name = "Grim Contest";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Grim Contest should have a spell program");
    let debug = format!("{program:#?}");
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert_eq!(
        compact.matches("TargetOnlyEffect").count(),
        2,
        "the linked pair must not acquire a third target: {debug}"
    );
    assert!(
        compact.contains("ForEachObject")
            && compact.contains("ExecuteWithSourceEffect{source:Iterated")
            && compact.contains("ToughnessOf(Iterated")
            && compact.contains("other:true")
            && !compact.contains("target:AnyOtherTarget"),
        "each chosen creature must damage only the other member using its own toughness: {debug}"
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
