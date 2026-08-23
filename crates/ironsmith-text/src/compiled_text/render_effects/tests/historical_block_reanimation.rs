use super::*;

const HISTORICAL_BLOCK_REANIMATION: &str = "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control";

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn historical_block_reanimation_round_trips_exact_controller_provenance() {
    let definition = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Historical block reanimation",
    )
    .card_types(vec![CardType::Instant])
    .parse_text(format!("{HISTORICAL_BLOCK_REANIMATION}."))
    .expect("historical block reanimation should parse");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        format!("{HISTORICAL_BLOCK_REANIMATION}.")
    );
}
