use super::*;

const ORACLE: &str = "{2}{U}, {T}: Target player draws a card, then exiles a card from their hand. If a creature card is exiled this way, that player creates a token that's a copy of that card.\nWhen this creature leaves the battlefield, exile all tokens created with it at the beginning of the next end step.";

#[test]
fn arcane_artisan_keeps_one_target_and_the_exiled_card_copy_reference() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Arcane Artisan")
        .card_types(vec![CardType::Creature])
        .parse_text(ORACLE)
        .expect("Arcane Artisan should compile");

    let compiled = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(compiled, ORACLE, "{:#?}", definition.abilities);
    assert_eq!(compiled.matches("Target player").count(), 1, "{compiled}");
    assert!(!compiled.contains("Choose target player"), "{compiled}");
}
