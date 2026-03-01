use ironsmith::{cards::CardDefinitionBuilder, ids::CardId, types::CardType};

#[test]
fn parser_feature_smoke_spell_line_parses() {
    let text = "Destroy target creature.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Parser Smoke")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("parser smoke spell should parse");
    assert!(def.spell_effect.is_some());
}

#[test]
fn parser_feature_smoke_trigger_line_parses() {
    let text = "Whenever this creature deals combat damage to a player, draw a card.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Parser Trigger Smoke")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("parser smoke trigger should parse");
    assert!(!def.abilities.is_empty());
}
