#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

const TYPED_AURA_CARDS: [&str; 3] = ["Kitnap", "Redemption Arc", "Volrath's Curse"];

#[test]
fn typed_aura_attachment_is_executable_without_leaking_into_compiled_text() {
    for name in TYPED_AURA_CARDS {
        let oracle = oracle_text_by_name()
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle text for {name}"));
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("aura_attach_filter: Some")
                && debug.contains("AttachToEffect")
                && debug.contains("card_types: [\n")
                && debug.contains("Creature"),
            "{name} must retain a typed, executable creature attachment: {debug}"
        );

        let compiled = unprocessed_compiled_lines(&definition);
        assert!(
            compiled
                .iter()
                .all(|line| !line.contains("Attach this source")),
            "{name} must not render the attachment operation twice: {compiled:#?}"
        );
        match name {
            "Redemption Arc" => assert!(
                compiled.iter().any(|line| {
                    line == "Enchanted creature has indestructible and is goaded."
                }),
                "the compound attached grant must remain a static ability: {compiled:#?}"
            ),
            "Volrath's Curse" => assert!(
                compiled.iter().any(|line| {
                    line
                        == "Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn."
                }),
                "the restriction and special-action permission must render together: {compiled:#?}"
            ),
            "Kitnap" => {}
            _ => unreachable!(),
        }

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
