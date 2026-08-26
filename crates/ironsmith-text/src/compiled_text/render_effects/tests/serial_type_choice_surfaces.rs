use super::*;

#[test]
fn nimble_larcenist_keeps_serial_card_types_inside_the_revealed_hand_choice() {
    let oracle = "Flying\nWhen this creature enters, target opponent reveals their hand. You choose an artifact, instant, or sorcery card from it and exile that card.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nimble Larcenist")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .expect("Nimble Larcenist should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}
