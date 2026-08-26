use super::*;

#[test]
fn perish_the_thought_keeps_the_revealed_hand_choice_and_shuffle_linked() {
    let oracle = "Target opponent reveals their hand. You choose a card from it. That player shuffles that card into their library.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Perish the Thought")
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("Perish the Thought should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}
