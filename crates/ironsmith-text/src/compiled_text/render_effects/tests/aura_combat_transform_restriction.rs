use super::*;
const TEXT: &str = "Enchant creature\nEnchanted creature can't attack, block, or transform.\nSacrifice another permanent: Attach this Aura to target creature. Activate only as a sorcery and only once each turn.";

#[test]
fn aura_combat_transform_restriction_tracks_the_enchanted_creature() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bound by Moonsilver")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let first = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for enchanted in [first, second] {
        game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(enchanted));
        game.refresh_continuous_state();
        for creature in [first, second] {
            assert_eq!(game.can_attack(creature), creature != enchanted);
            assert_eq!(game.can_block(creature), creature != enchanted);
            assert_eq!(game.can_transform(creature), creature != enchanted);
        }
    }
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    game.refresh_continuous_state();
    for creature in [first, second] {
        assert!(game.can_attack(creature));
        assert!(game.can_block(creature));
        assert!(game.can_transform(creature));
    }
    assert_only_attachment_spell(&definition);
}

#[test]
fn aura_combat_transform_restriction_does_not_add_unlisted_actions() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Restricting Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text("Enchant creature\nEnchanted creature can't transform or attack.")
            .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(creature));
    game.refresh_continuous_state();
    assert!(!game.can_transform(creature));
    assert!(!game.can_attack(creature));
    assert!(game.can_block(creature));
    assert_only_attachment_spell(&definition);
}

fn assert_only_attachment_spell(definition: &crate::CardDefinition) {
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Aura attachment spell")
        .flattened_default_effects();
    assert_eq!(effects.len(), 1);
    let effect =
        crate::compiled_text::render_effects::effect_lists::structural_unwrap_render_wrappers(
            &effects[0],
        );
    assert!(
        effect
            .downcast_ref::<crate::effects::AttachToEffect>()
            .is_some(),
        "the only resolving instruction must attach the Aura"
    );
}
