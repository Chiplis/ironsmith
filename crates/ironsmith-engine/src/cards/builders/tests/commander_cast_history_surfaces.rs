#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_exact(name: &str, oracle: &str) {
    let definition = parse_oracle_card_definition(name);
    let compiled = canonical_compiled_lines(&definition).join("\n");
    assert_eq!(compiled, oracle, "{name}: {definition:#?}");
}

#[test]
fn commander_cast_history_consumers_keep_command_zone_provenance() {
    assert_exact(
        "Font of Magic",
        "Instant and sorcery spells you cast cost {1} less to cast for each time you've cast a commander from the command zone this game.",
    );
    assert_exact(
        "Skull Storm",
        "When you cast this spell, copy it for each time you've cast your commander from the command zone this game.\nEach opponent sacrifices a creature of their choice. Each opponent who can't loses half their life, rounded up.",
    );
    assert_exact(
        "Captain Vargus Wrath",
        "Whenever Captain Vargus Wrath attacks, Pirates you control get +1/+1 until end of turn for each time you've cast a commander from the command zone this game.",
    );
    assert_exact(
        "Jyoti, Moag Ancient",
        "When Jyoti enters, create a 1/1 green Forest Dryad land creature token for each time you've cast your commander from the command zone this game.\nAt the beginning of each combat, land creatures you control get +X/+X until end of turn, where X is Jyoti's power.",
    );
}
