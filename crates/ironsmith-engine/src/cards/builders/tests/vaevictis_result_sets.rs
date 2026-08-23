#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn vaevictis_renders_and_executes_correlated_player_result_sets() {
    let oracle = "Flying\nWhenever Vaevictis Asmadi attacks, for each player, choose target permanent that player controls. Those players sacrifice those permanents. Each player who sacrificed a permanent this way reveals the top card of their library, then puts it onto the battlefield if it's a permanent card.";
    let definition = parse_oracle_card_definition("Vaevictis Asmadi, the Dire");
    let debug = format!("{definition:#?}");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle,
        "the correlated result-set program must render exactly: {debug}"
    );

    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("TargetOnlyEffect")
            && debug.contains("SacrificePlayerEffect")
            && debug.contains("TaggedEffect")
            && debug.contains("__sentence_helper_sacrificed_")
            && debug.contains("RevealTopEffect")
            && debug.contains("zone: Battlefield"),
        "the compiled trigger must preserve each chosen permanent, the actual sacrifice result, and each qualifying player's reveal/put: {debug}"
    );
}
