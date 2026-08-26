use super::*;

fn render_card(name: &str, card_types: Vec<CardType>, oracle: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(oracle)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn reveal_until_partitions_keep_the_match_and_exact_remainder() {
    for (name, card_types, oracle) in [
        (
            "Hermit Druid",
            vec![CardType::Creature],
            "{G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.",
        ),
        (
            "Telemin Performance",
            vec![CardType::Sorcery],
            "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control.",
        ),
    ] {
        assert_eq!(render_card(name, card_types, oracle), oracle, "{name}");
    }
}
