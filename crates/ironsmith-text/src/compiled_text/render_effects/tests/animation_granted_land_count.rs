use super::*;
const TEXT: &str = "{5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\"";

#[test]
fn animation_granted_land_count_keeps_types_and_dynamic_size() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Beorn's Hospitality")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Test Land")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..3 {
        game.create_object_from_card(&land, alice, Zone::Battlefield);
    }
    for _ in 0..5 {
        game.create_object_from_card(&land, bob, Zone::Battlefield);
    }
    let crate::ability::AbilityKind::Activated(ability) = &definition.abilities[0].kind else {
        panic!("activated");
    };
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for segment in &ability.effects.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert!(game.current_is_creature(source));
    assert!(game.current_has_card_type(source, CardType::Enchantment));
    assert!(game.current_has_subtype(source, Subtype::Bear));
    assert_eq!(
        (game.current_power(source), game.current_toughness(source)),
        (Some(3), Some(3))
    );
    let added = game.create_object_from_card(&land, alice, Zone::Battlefield);
    assert_eq!(
        (game.current_power(source), game.current_toughness(source)),
        (Some(4), Some(4))
    );
    game.move_object_by_effect(added, Zone::Graveyard).unwrap();
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert!(game.current_is_creature(source));
    assert_eq!(
        (game.current_power(source), game.current_toughness(source)),
        (Some(3), Some(3))
    );
    let remove = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ability Removal")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature loses all abilities until end of turn.")
        .unwrap();
    let spell = game.create_object_from_definition(&remove, alice, Zone::Stack);
    let program = game.object(spell).unwrap().spell_effect_owned().unwrap();
    let mut ctx = crate::effects::EffectContext::new_default(spell, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(source)]);
    ctx.snapshot_targets(&game);
    for segment in &program.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert!(!game.current_has_static_ability_id(
        source,
        crate::static_abilities::StaticAbilityId::CharacteristicDefiningPT
    ));
    assert_ne!(
        game.current_power(source),
        Some(3),
        "removed grant cannot keep setting size"
    );
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(
        (game.current_power(source), game.current_toughness(source)),
        (Some(3), Some(3))
    );
    // A later setting effect wins over the older granted size. Granting the
    // same ability instance to another permanent must not retimestamp it.
    let setting = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Size Setting")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature has base power and toughness 1/1 until end of turn.")
        .unwrap();
    let spell = game.create_object_from_definition(&setting, alice, Zone::Stack);
    let program = game.object(spell).unwrap().spell_effect_owned().unwrap();
    let mut ctx = crate::effects::EffectContext::new_default(spell, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(source)]);
    ctx.snapshot_targets(&game);
    for segment in &program.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert_eq!(
        game.current_power(source),
        Some(1),
        "setting before second grant"
    );
    let second = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = crate::effects::EffectContext::new_default(second, alice);
    for segment in &ability.effects.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert_eq!(
        (game.current_power(source), game.current_toughness(source)),
        (Some(1), Some(1))
    );
    assert_eq!(
        (game.current_power(second), game.current_toughness(second)),
        (Some(3), Some(3))
    );
}

#[test]
fn animation_granted_land_count_renders_both_actions() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Beorn's Hospitality")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
