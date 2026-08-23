#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn indomitable_creativity_renders_the_exact_staged_collection_program() {
    let oracle = "Destroy X target artifacts and/or creatures. For each permanent destroyed this way, its controller reveals cards from the top of their library until an artifact or creature card is revealed and exiles that card. Those players put the exiled cards onto the battlefield, then shuffle.";
    let definition = parse_oracle_card_definition("Indomitable Creativity");

    assert_eq!(canonical_compiled_lines(&definition).join(" "), oracle);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("ForEachTaggedEffect")
            && debug.contains("ForEachControllerOfTaggedEffect")
            && debug.contains("__exiled_collection")
            && debug.contains("ConsultTopOfLibraryEffect")
            && debug.contains("zone: Exile")
            && debug.contains("zone: Battlefield"),
        "the staged collection must reveal/exile per destroyed permanent, then move and shuffle once: {debug}"
    );
}
