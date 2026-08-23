#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn faithful_pikemaster_preserves_the_authored_as_long_as_turn_surface() {
    let name = "Faithful Pikemaster";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    assert!(definition.abilities.iter().any(|ability| {
        matches!(&ability.kind, AbilityKind::Static(ability)
        if ability.compiled_model().is_some_and(|model| model.label.starts_with(
            ironsmith_core::static_ability_model::AS_LONG_AS_ITS_YOUR_TURN_STATIC_LABEL_PREFIX
        )))
    }));

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
