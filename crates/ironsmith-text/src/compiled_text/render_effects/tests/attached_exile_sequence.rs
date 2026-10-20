use super::*;

fn render(text: &str) -> String {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Attachment test")
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .unwrap();
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn exile_target_and_all_equipment_attached_to_it_as_one_instruction() {
    assert_eq!(
        render("Exile target attacking creature and all Equipment attached to it."),
        "Exile target attacking creature and all Equipment attached to it."
    );
}
