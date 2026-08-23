#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn temporal_graveyard_history_cards_preserve_their_authored_distinctions() {
    let cases: &[(&str, &str, &[&str])] = &[
        (
            "Reenact the Crime",
            "that was put there from anywhere this turn",
            &[
                "that was put there this turn",
                "from the battlefield this turn",
            ],
        ),
        (
            "Moira, Urborg Haunt",
            "that was put there from the battlefield this turn",
            &["that was put there this turn", "from anywhere this turn"],
        ),
        (
            "Abyssal Harvester",
            "that was put there this turn",
            &["from anywhere this turn", "from the battlefield this turn"],
        ),
    ];

    for (name, expected, rejected) in cases {
        let definition = parse_oracle_card_definition(name);
        let rendered = canonical_compiled_lines(&definition).join("\n");
        assert!(
            rendered.contains(expected),
            "{name} lost `{expected}`: {rendered}"
        );
        for phrase in *rejected {
            assert!(
                !rendered.contains(phrase),
                "{name} rendered the wrong history surface `{phrase}`: {rendered}"
            );
        }
    }
}
