use super::*;

#[test]
fn copied_end_step_exile_is_intrinsic_and_survives_another_copy() {
    let oracle = "Create a token that's a copy of target creature, except it has haste and \"At the beginning of the end step, exile this token.\"";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "End Step Copy")
        .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    let plain_copy = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Plain Copy")
        .card_types(vec![CardType::Sorcery]).parse_text("Create a token that's a copy of target creature.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Copy Original")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(3, 3)).build();
    let original = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let mut previous = original;
    let mut tokens = Vec::new();
    for spell in [&definition, &plain_copy] {
        let source = game.create_object_from_definition(spell, alice, Zone::Stack);
        let before = game.battlefield.clone();
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(previous)]);
        ctx.snapshot_targets(&game);
        for segment in &spell.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        let created = *game.battlefield.iter().find(|id| !before.contains(id)).unwrap();
        assert!(game.current_has_static_ability_id(created, crate::static_abilities::StaticAbilityId::Haste));
        tokens.push(created);
        previous = created;
    }
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfEndStepEvent::new(bob), crate::provenance::ProvNodeId::default());
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(triggers.len(), 2, "both copies must own their end-step ability: {triggers:#?}");
    for entry in triggers {
        assert!(tokens.contains(&entry.source), "the token is the ability source");
        let mut ctx = crate::effects::EffectContext::new_default(entry.source, entry.controller);
        ctx.triggering_event = Some(entry.triggering_event);
        for segment in &entry.ability.effects.segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
    }
    assert_eq!(game.battlefield, vec![original], "each copied trigger exiles its own token");
    assert_eq!(rendered, oracle, "{definition:#?}");
}

#[test]
fn copied_entry_ability_is_installed_before_each_token_enters() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Entry Copy")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create X tokens that are copies of target creature you control, except they have \"When this token enters, it fights up to one target creature you don't control.\"").unwrap();
    for count in [0, 2] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Copy Original")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(3, 3)).build();
        let defender = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Fight Recipient")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1, 10)).build();
        let original = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let opponent = game.create_object_from_card(&defender, bob, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_x(count)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(original)]);
        ctx.snapshot_targets(&game);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        let mut queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        assert_eq!(queue.entries.len(), count as usize, "one entry trigger per token");
        for entry in queue.entries {
            assert_ne!(entry.source, original);
            let token = entry.source;
            let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
                &game, &entry.ability.effects, alice, Some(token), None);
            assert_eq!(requirements.len(), 1);
            assert!(requirements[0].legal_targets.contains(&crate::game_state::Target::Object(opponent)));
            let mut ctx = crate::effects::EffectContext::new_default(token, alice)
                .with_targets(vec![crate::effects::ResolvedTarget::Object(opponent)])
                .with_target_assignments(vec![crate::game_state::TargetAssignment {
                    spec: requirements[0].spec.clone(), range: 0..1,
                }]);
            ctx.triggering_event = Some(entry.triggering_event);
            ctx.snapshot_targets(&game);
            for segment in &entry.ability.effects.segments {
                for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            }
            assert_eq!(game.damage_on(token), 1);
        }
        assert_eq!(game.damage_on(opponent), count * 3);
        assert_eq!(game.damage_on(original), 0);
    }
}
