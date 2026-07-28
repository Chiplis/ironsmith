#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn food_sacrifice_outcomes_keep_the_shared_target_as_that_creature() {
    for (name, accepted, fallback) in [
        ("Insatiable Appetite", "+5/+5", "+3/+3"),
        ("Pippin's Bravery", "+4/+4", "+2/+2"),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition),
            vec![format!(
                "You may sacrifice a Food. If you do, target creature gets {accepted} until end of turn. Otherwise, that creature gets {fallback} until end of turn."
            )],
            "{definition:#?}"
        );
    }
}
