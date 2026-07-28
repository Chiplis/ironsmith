#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn past_controller_followups_keep_the_authored_conditional_for_return_and_destroy() {
    for (name, expected) in [
        (
            "Boomerang Basics",
            "Return target nonland permanent to its owner's hand. If you controlled that permanent, draw a card.",
        ),
        (
            "Geistwave",
            "Return target nonland permanent to its owner's hand. If you controlled that permanent, draw a card.",
        ),
        (
            "Kellan, Inquisitive Prodigy",
            "Whenever Kellan attacks, destroy up to one target artifact. If you controlled that permanent, draw a card.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let lines = unprocessed_compiled_lines(&definition);
        assert!(
            lines.iter().any(|line| line == expected),
            "{name} should retain the authored past-controller conditional; got {lines:#?}"
        );

        let debug = format!("{:#?}", definition.abilities);
        assert!(
            debug.contains("PlayerTaggedObjectMatches")
                && debug.contains("mode: LastKnown")
                && debug.contains("demonstrative_antecedent: Some(\n")
                && debug.contains("Permanent"),
            "{name} should retain typed LKI and demonstrative metadata; got {debug}"
        );
    }
}
