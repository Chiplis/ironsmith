use super::*;

fn render_sorcery(text: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice test")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .unwrap();
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn chosen_hand_cards_are_kept_when_the_complement_is_discarded() {
    assert_eq!(
        render_sorcery("Draw four cards, then choose X cards in your hand and discard the rest."),
        "Draw four cards, then choose X cards in your hand and discard the rest."
    );
}

#[test]
fn discarding_a_chosen_hand_card_does_not_render_as_discarding_the_rest() {
    let rendered = render_sorcery("Choose a card in your hand, then discard that card.");
    assert!(!rendered.contains("discard the rest"), "{rendered}");
}

#[test]
fn per_color_hand_choices_keep_the_cards_selected_before_discarding_nonlands() {
    assert_eq!(
        render_sorcery(
            "Each player reveals their hand, chooses one card of each color from it, then discards all other nonland cards."
        ),
        "Each player reveals their hand, chooses one card of each color from it, then discards all other nonland cards."
    );
}
