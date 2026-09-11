use super::*;

#[test]
fn independent_optional_exile_groups_are_targets_and_both_resolve() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Exile Pair Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Exile up to one target nonland permanent and up to one target nonland permanent card from a graveyard.").unwrap();
    for (choose_battlefield, choose_graveyard) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let battlefield = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let graveyard = game.create_object_from_card(&card, bob, Zone::Graveyard);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let program = definition.spell_effect.as_ref().unwrap();
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            program,
            alice,
            Some(source),
            None,
        );
        assert_eq!(requirements.len(), 2, "{requirements:#?}");
        let targets = [
            (choose_battlefield, battlefield),
            (choose_graveyard, graveyard),
        ]
        .into_iter()
        .filter(|(choose, _)| *choose)
        .map(|(_, id)| crate::effects::ResolvedTarget::Object(id))
        .collect();
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_targets(targets);
        ctx.snapshot_targets(&game);
        for segment in &program.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        for (chosen, id) in [
            (choose_battlefield, battlefield),
            (choose_graveyard, graveyard),
        ] {
            assert_eq!(
                game.object(id).is_none(),
                chosen,
                "battlefield={choose_battlefield}, graveyard={choose_graveyard}"
            );
        }
        assert_eq!(
            game.exile.len(),
            usize::from(choose_battlefield) + usize::from(choose_graveyard)
        );
    }
}

#[test]
fn independent_exiles_return_to_each_owner_after_the_source_dies() {
    for source_reference in ["it", "this creature"] {
        for source_move in 0..3 {
            check_linked_exile_return(source_reference, source_move);
        }
    }
}

fn check_linked_exile_return(source_reference: &str, source_move: usize) {
    let source_left_graveyard = source_move != 0;
    let oracle = "When this creature enters, exile up to one target nonland permanent and up to one target nonland permanent card from a graveyard.\nWhen this creature dies, put it on the bottom of its owner's library. If you do, return the exiled cards to their owners' hands.";
    let oracle = oracle.replace("put it on", &format!("put {source_reference} on"));
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Linked Exile Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(&oracle)
            .unwrap();
    let abilities: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| {
            if let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind {
                Some(triggered)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(abilities.len(), 2);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let carol = game.players[2].id;
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Victim")
        .card_types(vec![CardType::Creature])
        .build();
    let battlefield = game.create_object_from_card(&card, bob, Zone::Battlefield);
    let graveyard = game.create_object_from_card(&card, carol, Zone::Graveyard);
    let unrelated = game.create_object_from_card(&card, carol, Zone::Exile);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let source_stable = game.object(source).unwrap().stable_id;
    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    let entry = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_triggering_event(entry)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(battlefield),
            crate::effects::ResolvedTarget::Object(graveyard),
        ]);
    ctx.snapshot_targets(&game);
    for segment in &abilities[0].effects.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert_eq!(game.exile.len(), 3);
    assert_eq!(game.get_exiled_with_source_links(source).len(), 2);
    let dead_source = game
        .move_object(
            source,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
        )
        .unwrap();
    if source_left_graveyard {
        let exiled_source = game
            .move_object(
                dead_source,
                Zone::Exile,
                crate::events::cause::EventCause::effect(),
            )
            .unwrap();
        if source_move == 2 {
            game.move_object(
                exiled_source,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
            )
            .unwrap();
        }
    }
    let death = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_results(
            source,
            vec![dead_source],
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.stack.push(
        crate::game_state::StackEntry::ability(source, alice, abilities[1].effects.clone())
            .with_source_snapshot(snapshot)
            .with_triggering_event(death),
    );
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    let source_now = game.find_object_by_stable_id(source_stable).unwrap();
    assert_eq!(
        game.object(source_now).unwrap().zone,
        match source_move {
            0 => Zone::Library,
            1 => Zone::Exile,
            _ => Zone::Graveyard,
        },
        "a source that left its death zone is a new object"
    );
    let expected_returns = usize::from(!source_left_graveyard);
    assert_eq!(game.player(bob).unwrap().hand.len(), expected_returns);
    assert_eq!(game.player(carol).unwrap().hand.len(), expected_returns);
    assert_eq!(game.object(unrelated).unwrap().zone, Zone::Exile);
    let returned_source = game.find_object_by_stable_id(source_stable).unwrap();
    if !source_left_graveyard {
        assert_eq!(
            game.player(alice).unwrap().library.first(),
            Some(&returned_source)
        );
    }
}
