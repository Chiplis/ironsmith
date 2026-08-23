#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn compleat_devotion_keeps_the_toxic_subject_as_that_creature() {
    let definition = parse_oracle_card_definition("Compleat Devotion");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Target creature you control gets +2/+2 until end of turn. If that creature has toxic, draw a card.",
        ],
        "{definition:#?}"
    );
}

#[test]
fn driftgloom_coyote_keeps_the_last_known_power_subject_as_that_creature() {
    let definition = parse_oracle_card_definition("Driftgloom Coyote");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "When this creature enters, exile target creature an opponent controls until this creature leaves the battlefield. If that creature had power 2 or less, put a +1/+1 counter on this creature.",
        ],
        "{definition:#?}"
    );
}
