use super::*;

const KANG_DYNASTY_TEXT: &str = "I, II — For each opponent, tap up to one target creature that player controls. Goad those creatures. Until your next turn, whenever any of those creatures deals combat damage to a player, draw a card.\nIII — Target creature you control gets +1/+1 until end of turn for each card in your hand and can't be blocked this turn.";

#[test]
fn saga_chapters_keep_selected_creature_history_and_shared_pump_target() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Kang Dynasty")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(KANG_DYNASTY_TEXT)
        .expect("correlated Saga chapters should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        KANG_DYNASTY_TEXT
    );

    let debug = format!("{definition:#?}");
    assert!(debug.contains("target_tag: Some"), "{debug}");
    assert!(debug.contains("\"tapped_0\""), "{debug}");
    assert!(debug.contains("\"pumped_0\""), "{debug}");
}
