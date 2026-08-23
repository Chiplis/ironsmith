use super::*;

#[test]
fn draw_step_player_searches_keep_the_shared_player_surface() {
    for (name, card_type, oracle) in [
        (
            "Maralen of the Mornsong",
            CardType::Creature,
            "Players can't draw cards.\nAt the beginning of each player's draw step, that player loses 3 life, searches their library for a card, puts it into their hand, then shuffles.",
        ),
        (
            "Mornsong Aria",
            CardType::Enchantment,
            "Players can't draw cards or gain life.\nAt the beginning of each player's draw step, that player loses 3 life, searches their library for a card, puts it into their hand, then shuffles.",
        ),
    ] {
        let definition =
            crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
                .card_types(vec![card_type])
                .parse_text(oracle)
                .expect("the draw-step player search should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            oracle,
            "{name}"
        );
    }
}
