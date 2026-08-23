#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn descendants_path_names_the_declined_cast_instead_of_using_otherwise() {
    let oracle = "At the beginning of your upkeep, reveal the top card of your library. If it's a creature card that shares a creature type with a creature you control, you may cast it without paying its mana cost. If you don't cast it, put it on the bottom of your library.";
    let definition = parse_oracle_card_definition("Descendants' Path");
    let compiled = canonical_compiled_lines(&definition).join("\n");
    let debug = format!("{definition:#?}");

    assert_eq!(compiled, oracle, "{debug}");
    assert!(
        debug.contains("CastTaggedEffect")
            && debug.contains("DidNotHappen")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("kind: SharesAny"),
        "{debug}"
    );
}
