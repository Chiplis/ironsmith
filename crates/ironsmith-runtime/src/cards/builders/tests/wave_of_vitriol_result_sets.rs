#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const WAVE_ORACLE: &str = "Each player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player who searched their library this way shuffles.";

fn parse_sorcery(name: &str, oracle: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("result-set search program should parse")
}

#[test]
fn wave_of_vitriol_renders_and_executes_exact_correlated_result_sets() {
    let definition = parse_oracle_card_definition("Wave of Vitriol");
    let debug = format!("{definition:#?}");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        WAVE_ORACLE,
        "the typed sacrifice/search procedure must render exactly: {debug}"
    );
    assert!(
        debug.contains("SacrificePlayerEffect")
            && debug.contains("ForEachTaggedEffect")
            && debug.contains("TaggedObjectMatchedLastKnown")
            && debug.contains("MayEffect")
            && debug.contains("decider: Some")
            && debug.contains("PutOntoBattlefieldEffect")
            && debug.contains("chooser: ControllerOf")
            && debug.contains("owner: Some(\n")
            && debug.contains("AliasedControllerOf")
            && debug.contains("ShuffleLibraryEffect"),
        "the runtime program must retain the sacrificed-land snapshot, its controller, and the searched-player gate: {debug}"
    );
}

#[test]
fn wave_renderer_rejects_a_union_that_also_sacrifices_basic_lands() {
    let near_miss = parse_sorcery(
        "Wave near miss",
        "Each player sacrifices all artifacts, enchantments, and lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player who searched their library this way shuffles.",
    );
    let rendered = canonical_compiled_lines(&near_miss).join("\n");

    assert_ne!(rendered, WAVE_ORACLE);
    assert!(
        !rendered.contains("nonbasic lands they control"),
        "the structural renderer must not invent the missing nonbasic restriction: {rendered}"
    );
}

#[test]
fn wave_renderer_rejects_an_unconditional_each_player_shuffle() {
    let near_miss = parse_sorcery(
        "Wave shuffle near miss",
        "Each player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player shuffles.",
    );
    let rendered = canonical_compiled_lines(&near_miss).join("\n");

    assert_ne!(rendered, WAVE_ORACLE);
    assert!(
        !rendered.contains("each player who searched their library this way shuffles"),
        "the structural renderer must require the searched-player result id: {rendered}"
    );
}
