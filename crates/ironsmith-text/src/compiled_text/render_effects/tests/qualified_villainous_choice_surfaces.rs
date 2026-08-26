use super::*;

#[test]
fn life_loss_qualified_opponents_keep_typed_villainous_choice() {
    let oracle = "Each opponent who lost 3 or more life this turn faces a villainous choice — You draw a card, or that player discards a card.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("qualified villainous choice should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        oracle,
        "{debug}"
    );
    assert!(debug.contains("VillainousChoiceEffect"), "{debug}");
    assert!(debug.contains("LifeLostThisTurn(\n"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
}
