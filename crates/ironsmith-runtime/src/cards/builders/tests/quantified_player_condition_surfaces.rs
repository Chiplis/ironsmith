#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn qualified_each_player_procedures_render_exactly() {
    for (name, oracle) in [
        (
            "Scholarship Sponsor",
            "When this creature enters, each player who controls fewer lands than the player who controls the most lands searches their library for a number of basic land cards less than or equal to the difference, puts those cards onto the battlefield tapped, then shuffles.",
        ),
        (
            "Natural Balance",
            "Each player who controls six or more lands chooses five lands they control and sacrifices the rest. Each player who controls four or fewer lands may search their library for up to X basic land cards and put them onto the battlefield, where X is five minus the number of lands they control. Then each player who searched their library this way shuffles.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        let debug = format!("{definition:#?}");

        assert_eq!(compiled, oracle, "{name}: {debug}");
        assert!(
            debug.contains("ForPlayersEffect")
                && debug.contains("ConditionalEffect")
                && debug.contains("IteratedPlayer"),
            "{name}: {debug}"
        );
    }
}
