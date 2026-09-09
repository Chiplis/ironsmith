use super::*;
const TEXT: &str = "When this creature enters, you may put an art sticker on a nonland permanent you own.\n{2}{R}: Another target creature with an art sticker on it gets +2/+0 and gains menace until end of turn.";
#[test]
fn sticker_target_modifier_preserves_the_complete_subject() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Proficient Pyrodancer")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}

#[test]
fn sticker_target_modifier_targets_only_other_art_sticker_creatures() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Proficient Pyrodancer")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 3)).parse_text(TEXT).unwrap();
    let AbilityKind::Activated(ability) = &definition.abilities[1].kind else { panic!("activated"); };
    for opponent_target in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let own = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let opponent = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let unstickered = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let wrong_sticker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let graveyard = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        for id in [source, own, opponent, graveyard] { game.put_sticker_on_object(id, crate::events::KeywordActionKind::ArtSticker); }
        game.put_sticker_on_object(wrong_sticker, crate::events::KeywordActionKind::NameSticker);
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game, &ability.effects, alice, Some(source), None);
        assert_eq!(requirements.len(), 1, "{requirements:#?}");
        assert_eq!(requirements[0].min_targets, 1);
        assert_eq!(requirements[0].max_targets, Some(1));
        assert_eq!(requirements[0].legal_targets.len(), 2, "{requirements:#?}");
        for id in [own, opponent] { assert!(requirements[0].legal_targets.contains(&crate::Target::Object(id))); }
        let target = if opponent_target { opponent } else { own };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.snapshot_targets(&game);
        for effect in &ability.effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        game.refresh_continuous_state();
        for id in [source, own, opponent, unstickered, wrong_sticker] {
            assert_eq!(game.calculated_power(id), Some(if id == target { 4 } else { 2 }));
            assert_eq!(game.calculated_toughness(id), Some(3));
            assert_eq!(game.current_has_static_ability_id(id, crate::static_abilities::StaticAbilityId::Menace), id == target);
        }
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(target), Some(2));
        assert!(!game.current_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::Menace));
    }
}

#[test]
fn leading_target_modifier_excludes_an_embedded_source_reference() {
    let text = "Alliance — Whenever another creature you control enters, target creature other than this creature gets +1/+1 and gains trample until end of turn.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Elegant Entourage")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4)).parse_text(text).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), text);
    let AbilityKind::Triggered(ability) = &definition.abilities[0].kind else { panic!("triggered"); };
    for opponent_target in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let own = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let opponent = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        assert_eq!(ability.choices.len(), 1);
        let legal = crate::game_loop::compute_legal_targets(&game, &ability.choices[0], alice, Some(source));
        assert_eq!(legal.len(), 2, "{legal:#?}");
        for id in [own, opponent] { assert!(legal.contains(&crate::Target::Object(id))); }
        let target = if opponent_target { opponent } else { own };
        let entry = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(own, Zone::Stack, Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(crate::snapshot::ObjectSnapshot::from_object(game.object(own).unwrap(), &game))),
            crate::provenance::ProvNodeId::default());
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_triggering_event(entry)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.snapshot_targets(&game);
        for id in [source, own, opponent] {
            assert_eq!(crate::effects::helpers::validate_target(&game, &crate::effects::ResolvedTarget::Object(id), &ability.choices[0], &ctx), id != source);
        }

        for effect in &ability.effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        game.refresh_continuous_state();
        for id in [source, own, opponent] {
            assert_eq!(game.calculated_power(id), Some(if id == target { 5 } else { 4 }));
            assert_eq!(game.calculated_toughness(id), Some(if id == target { 5 } else { 4 }));
            assert_eq!(game.current_has_static_ability_id(id, crate::static_abilities::StaticAbilityId::Trample), id == target);
        }
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(target), Some(4));
        assert_eq!(game.calculated_toughness(target), Some(4));
        assert!(!game.current_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::Trample));
    }
}
