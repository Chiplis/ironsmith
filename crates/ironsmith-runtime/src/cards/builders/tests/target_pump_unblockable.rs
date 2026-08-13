#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn target_pump_keeps_same_target_unblockable_tail() {
    for (name, oracle) in [
        (
            "Taigam's Strike",
            "Target creature gets +2/+0 until end of turn and can't be blocked this turn.\nRebound.",
        ),
        (
            "Distortion Strike",
            "Target creature gets +1/+0 until end of turn and can't be blocked this turn.\nRebound.",
        ),
        (
            "Teleportal",
            "Target creature you control gets +1/+0 until end of turn and can't be blocked this turn.\nOverload {3}{U}{R}.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert_eq!(compiled, oracle, "{name}: {definition:#?}");
    }
}
