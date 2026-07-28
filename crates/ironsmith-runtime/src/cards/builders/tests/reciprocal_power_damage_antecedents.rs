#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn reciprocal_power_damage_keeps_the_shared_target_as_that_creature() {
    for (name, expected) in [
        (
            "Karplusan Yeti",
            "{T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature.",
        ),
        (
            "Tracker",
            "{G}{G}, {T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature.",
        ),
        (
            "Durkwood Tracker",
            "{1}{G}, {T}: If this creature is on the battlefield, it deals damage equal to its power to target attacking creature. That creature deals damage equal to its power to this creature.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition),
            vec![expected],
            "{definition:#?}"
        );
    }
}
