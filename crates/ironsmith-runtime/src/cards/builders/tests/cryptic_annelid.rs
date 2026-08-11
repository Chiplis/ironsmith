#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn cryptic_annelid_keeps_all_three_ordered_scry_actions() {
    let name = "Cryptic Annelid";
    let definition = parse_oracle_card_definition(name);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cryptic Annelid should have its enters trigger");
    let [effect] = triggered.effects.flattened_default_effects() else {
        panic!("expected one ordered runtime sequence: {triggered:#?}");
    };
    let sequence = effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the authored comma-then chain should remain typed");
    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    let counts = sequence
        .effects
        .iter()
        .map(|effect| {
            effect
                .downcast_ref::<crate::effects::ScryEffect>()
                .unwrap_or_else(|| panic!("expected a scry child, got {effect:#?}"))
                .count
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        counts,
        vec![
            crate::effect::Value::Fixed(1),
            crate::effect::Value::Fixed(2),
            crate::effect::Value::Fixed(3),
        ]
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert!(
        compiled
            .iter()
            .any(|line| line == "When this creature enters, scry 1, then scry 2, then scry 3."),
        "the complete ordered chain should render without dropping its final action: {compiled:?}"
    );

    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "Cryptic Annelid must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}
