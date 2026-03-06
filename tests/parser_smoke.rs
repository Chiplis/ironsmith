use ironsmith::{
    ability::AbilityKind,
    cards::CardDefinitionBuilder,
    effects::HauntExileEffect,
    ids::CardId,
    types::CardType,
};

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

#[test]
fn parser_feature_smoke_haunt_linkage_stitches_into_haunt_ability() {
    let text = "Haunt\nWhen this creature enters or the creature it haunts dies, draw a card.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Haunt Smoke")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("haunt smoke should parse");

    let haunt_ability = def
        .abilities
        .iter()
        .find(|ability| ability.text.as_deref() == Some("Haunt"))
        .expect("haunt keyword ability should be present");

    let AbilityKind::Triggered(triggered) = &haunt_ability.kind else {
        panic!("haunt keyword should lower to a triggered ability");
    };
    assert_eq!(triggered.effects.len(), 1);
    assert!(
        triggered.effects[0]
            .downcast_ref::<HauntExileEffect>()
            .is_some(),
        "haunt keyword should carry the delayed haunt exile effect"
    );
}
