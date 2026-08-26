use super::*;

#[test]
fn sigil_of_myrkul_keeps_mill_as_the_reflexive_trigger_producer() {
    const ORACLE: &str = "At the beginning of combat on your turn, mill a card. When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sigil of Myrkul")
            .card_types(vec![CardType::Enchantment])
            .parse_text(ORACLE)
            .expect("the reflexive mill program should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
