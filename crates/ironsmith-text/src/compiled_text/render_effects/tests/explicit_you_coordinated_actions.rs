use super::*;

#[test]
fn explicit_you_subject_spans_coordinated_trigger_actions() {
    let oracle = "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.\nAt the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.\nWhen this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Greed Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("coordinated actions with one explicit actor should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle,
        "{debug}"
    );
    assert_eq!(debug.matches("SequenceEffect").count(), 3, "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("SacrificePlayerEffect"), "{debug}");
}
