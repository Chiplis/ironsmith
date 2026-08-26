use super::*;

#[test]
fn enchanted_player_token_list_keeps_one_actor_and_all_three_tokens() {
    let oracle = "Enchant opponent\nWhen this Aura enters, enchanted opponent creates a Clue token, a Food token, and a Junk token.\nAt the beginning of combat on enchanted opponent's turn, that player may pay {1} for each artifact they control. If they don't, creatures can't attack this combat.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Overencumbered")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(oracle)
        .expect("shared enchanted-player token list should compile");

    let lines = crate::compiled_text::compiled_text_lines(&definition);
    assert_eq!(
        lines[1],
        "When this Aura enters, enchanted player creates a Clue token, a Food token, and a Junk token."
    );
    assert_eq!(lines[1].matches("enchanted player creates").count(), 1);
    let debug = format!("{definition:#?}");
    assert_eq!(debug.matches("CreateTokenEffect").count(), 3, "{debug}");
    assert!(debug.contains("duration: EndOfCombat"), "{debug}");
}
