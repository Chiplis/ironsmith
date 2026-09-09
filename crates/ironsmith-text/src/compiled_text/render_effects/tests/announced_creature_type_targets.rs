use super::*;

#[test]
fn announced_creature_type_limits_compiled_return_targets() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Typed return fixture")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return X target creatures of the creature type of your choice to their owner's hand.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let mut creatures = Vec::new();
    for subtype in [Subtype::Goblin, Subtype::Goblin, Subtype::Elf] {
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Type target")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype]).build();
        creatures.push(game.create_object_from_card(&card, bob, Zone::Battlefield));
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(source).unwrap().x_value = Some(2);
    game.set_chosen_subtype(source, Subtype::Goblin);
    let program = definition.spell_effect.as_ref().unwrap();
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game, program, alice, Some(source), None);
    assert_eq!(requirements.len(), 1, "{requirements:#?}");
    assert_eq!(requirements[0].min_targets, 2);
    assert_eq!(requirements[0].max_targets, Some(2));
    assert_eq!(requirements[0].legal_targets.len(), 2, "{requirements:#?}");
    for id in &creatures[..2] {
        assert!(requirements[0].legal_targets.contains(&crate::Target::Object(*id)));
    }
    let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_x(2)
        .with_targets(creatures[..2].iter().copied().map(crate::effects::ResolvedTarget::Object).collect());
    ctx.snapshot_targets(&game);
    for segment in &program.segments {
        for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    assert!(game.object(creatures[0]).is_none());
    assert!(game.object(creatures[1]).is_none());
    assert_eq!(game.object(creatures[2]).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.player(bob).unwrap().hand.len(), 2);
}

#[test]
fn announced_creature_type_return_renders_the_casting_choice() {
    let oracle = "Return X target creatures of the creature type of your choice to their owner's hand.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selective Snare")
        .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
    let effect = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects[0];
    let effect = super::super::structural_unwrap_render_wrappers(effect);
    let returned = effect.downcast_ref::<crate::effects::ReturnToHandEffect>().unwrap();
    let ChooseSpec::Object(filter) = returned.spec.base() else { panic!("object filter"); };
    assert!(filter.chosen_creature_type);
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), ["Return X target creatures of the chosen type to their owners' hands."],
        "filter={} effect={}", filter.description(), crate::compiled_text::describe_effect(effect));
}

#[test]
fn announced_creature_type_restricts_up_to_three_graveyard_cards() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Graveyard type fixture")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return up to three target creature cards of the creature type of your choice from your graveyard to your hand.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let mut eligible = Vec::new();
    for (owner, subtype, zone) in [
        (alice, Subtype::Goblin, Zone::Graveyard),
        (alice, Subtype::Goblin, Zone::Graveyard),
        (alice, Subtype::Elf, Zone::Graveyard),
        (bob, Subtype::Goblin, Zone::Graveyard),
        (alice, Subtype::Goblin, Zone::Battlefield),
    ] {
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Typed card")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype]).build();
        let id = game.create_object_from_card(&card, owner, zone);
        if owner == alice && subtype == Subtype::Goblin && zone == Zone::Graveyard { eligible.push(crate::Target::Object(id)); }
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.set_chosen_subtype(source, Subtype::Goblin);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game, definition.spell_effect.as_ref().unwrap(), alice, Some(source), None);
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].min_targets, 0);
    assert_eq!(requirements[0].max_targets, Some(3));
    assert_eq!(requirements[0].legal_targets.len(), eligible.len(), "{requirements:#?}");
    for target in &eligible { assert!(requirements[0].legal_targets.contains(target)); }
    for selected_count in 0..=eligible.len() {
        let mut preview = game.clone();
        let targets = eligible[..selected_count].iter().map(|target| {
            let crate::Target::Object(id) = target else { unreachable!() };
            crate::effects::ResolvedTarget::Object(*id)
        }).collect();
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_targets(targets);
        ctx.snapshot_targets(&preview);
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut preview, effect, &mut ctx).unwrap();
            }
        }
        assert_eq!(preview.player(alice).unwrap().hand.len(), selected_count);
        assert_eq!(preview.player(alice).unwrap().graveyard.len(), 3 - selected_count);
        assert_eq!(preview.player(bob).unwrap().graveyard.len(), 1);
    }
}
