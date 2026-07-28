#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

const COPY_CAST_CARDS: [&str; 6] = [
    "Reenact the Crime",
    "Roving Actuator",
    "Arcane Proxy",
    "Flawless Forgery",
    "Soundwave, Sonic Spy",
    "Shiko, Paragon of the Way",
];

#[test]
fn copy_then_cast_copy_cards_keep_the_copy_action_and_clear_the_floor() {
    for name in COPY_CAST_CARDS {
        let oracle = oracle_text_by_name()
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle text for {name}"));
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("CastTaggedEffect") && debug.contains("as_copy: true"),
            "{name} must retain a typed copy-cast permission: {debug}"
        );

        let compiled = unprocessed_compiled_lines(&definition);
        let rendered = compiled.join("\n");
        assert!(
            rendered.contains("Copy it") || rendered.contains("Copy that card"),
            "{name} must explicitly render the AST-proven copy action: {rendered}"
        );
        assert!(
            rendered.contains("You may cast the copy without paying its mana cost"),
            "{name} must render the free-cast permission for that copy: {rendered}"
        );

        let (_, _, similarity, _, mismatch) =
            crate::semantic_compare::compare_card_semantics_scored(
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
}
