use super::*;

fn render(oracle: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Draw shortfall")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .unwrap();
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn two_card_shortfall_retains_the_single_choice_and_actual_draw_surface() {
    let oracle = "Each player may draw up to two cards. For each card less than two a player draws this way, that player gains 2 life.";
    assert_eq!(render(oracle), oracle);
}

#[test]
fn shortfall_surface_uses_parsed_limit_and_gain_amount() {
    let oracle = "Each player may draw up to three cards. For each card less than three a player draws this way, that player gains 4 life.";
    assert_eq!(render(oracle), oracle);
}
