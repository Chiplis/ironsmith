use super::*;

#[test]
fn cohort_self_return_uses_both_authorized_zones() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Survivor Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{2}{G}: Return this card from your graveyard or from exile to the battlefield tapped.",
        )
        .unwrap();
    let ability = &card.abilities[0];
    assert_eq!(ability.functional_zones, [Zone::Graveyard, Zone::Exile]);
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        panic!("activated")
    };
    for origin in [Zone::Graveyard, Zone::Exile] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&card, alice, origin);
        let stable = game.object(source).unwrap().stable_id;
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        for effect in activated.effects.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let returned = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().stable_id == stable)
            .unwrap();
        assert!(game.is_tapped(returned));
    }
}

#[test]
fn cohort_three_counter_placements_keep_every_target_and_amount() {
    for (counter, symbol) in [
        (crate::CounterType::MinusOneMinusOne, "-1/-1"),
        (crate::CounterType::PlusOnePlusOne, "+1/+1"),
    ] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Distribution Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(format!("Put a {symbol} counter on target creature, two {symbol} counters on another target creature, and three {symbol} counters on a third target creature."))
            .unwrap();
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(5, 5))
            .build();
        let targets = (0..3)
            .map(|_| game.create_object_from_definition(&creature, alice, Zone::Battlefield))
            .collect::<Vec<_>>();
        let program = card.spell_effect.as_ref().unwrap();
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            program,
            alice,
            Some(source),
            None,
        );
        assert_eq!(requirements.len(), 3, "{requirements:#?}");
        let contexts = requirements
            .iter()
            .map(|requirement| {
                let mut context = crate::decisions::context::TargetRequirementContext::single(
                    requirement.description.clone(),
                    requirement.legal_targets.clone(),
                );
                context.distinct_player_group = requirement.distinct_player_group;
                context
            })
            .collect::<Vec<_>>();
        assert!(
            !crate::targeting::validate_flat_target_assignment(
                &contexts,
                &[crate::game_state::Target::Object(targets[0]); 3],
            ),
            "another/third targets must be distinct"
        );
        assert!(crate::targeting::validate_flat_target_assignment(
            &contexts,
            &targets
                .iter()
                .copied()
                .map(crate::game_state::Target::Object)
                .collect::<Vec<_>>(),
        ));
        game.stack.push(
            crate::game_state::StackEntry::new(source, alice)
                .with_targets(
                    targets
                        .iter()
                        .copied()
                        .map(crate::game_state::Target::Object)
                        .collect(),
                )
                .with_target_assignments(
                    requirements
                        .iter()
                        .enumerate()
                        .map(|(index, requirement)| crate::game_state::TargetAssignment {
                            spec: requirement.spec.clone(),
                            range: index..index + 1,
                        })
                        .collect(),
                ),
        );
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        for (index, target) in targets.iter().enumerate() {
            assert_eq!(
                game.object(*target)
                    .unwrap()
                    .counters
                    .get(&counter)
                    .copied(),
                Some(index as u32 + 1)
            );
        }
    }
}

#[test]
fn cohort_combined_entry_and_multitype_card_play_triggers_resolve_for_all_players() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nest Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(5, 5))
        .parse_text("When this creature enters and whenever you play a card with two or more card types, each player creates a tapped 2/2 black Bird creature token with flying. The tokens are goaded for the rest of the game.")
        .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&card).join("\n");
    assert!(
        rendered.contains("and whenever you play a card with 2 or more card types"),
        "{rendered}"
    );
    assert!(
        rendered.ends_with("The tokens are goaded for the rest of the game."),
        "{rendered}"
    );
    for (land, own, types, expected) in [
        (
            false,
            true,
            vec![CardType::Artifact, CardType::Creature],
            true,
        ),
        (
            false,
            false,
            vec![CardType::Artifact, CardType::Creature],
            false,
        ),
        (false, true, vec![CardType::Creature], false),
        (
            false,
            true,
            vec![CardType::Kindred, CardType::Instant],
            true,
        ),
        (
            false,
            true,
            vec![
                CardType::Artifact,
                CardType::Enchantment,
                CardType::Creature,
            ],
            true,
        ),
        (
            false,
            true,
            vec![CardType::Creature, CardType::Creature],
            false,
        ),
        (true, true, vec![CardType::Artifact, CardType::Land], true),
        (true, false, vec![CardType::Artifact, CardType::Land], false),
        (true, true, vec![CardType::Land], false),
    ] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let player = if own { alice } else { game.players[1].id };
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let event_card =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Played Card")
                .card_types(types.clone())
                .supertypes(vec![crate::types::Supertype::Legendary])
                .build();
        let played = game.create_object_from_definition(
            &event_card,
            player,
            if land { Zone::Battlefield } else { Zone::Stack },
        );
        let event = if land {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LandPlayedEvent::new(played, player, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            )
        } else {
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(played).unwrap(), &game);
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new_with_snapshot(
                    played,
                    player,
                    Zone::Hand,
                    snapshot,
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            usize::from(expected),
            "land={land} own={own} types={types:?}"
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        if expected {
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            let birds = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.object(*id).unwrap().name == "Bird")
                .collect::<Vec<_>>();
            assert_eq!(birds.len(), 3);
            for player in &game.players {
                assert_eq!(
                    birds
                        .iter()
                        .filter(|id| game.controller_of(game.object(**id).unwrap()) == player.id)
                        .count(),
                    1
                );
            }
            for bird in birds {
                assert!(game.is_tapped(bird));
                assert!(game.is_goaded(bird), "every created token must be goaded");
            }
        }
        let entered = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                source,
                Zone::Stack,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                None,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(crate::triggers::check_triggers(&game, &entered).len(), 1);
    }
}

#[test]
fn cohort_relative_later_object_target_can_be_the_source() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Pattern Keeper")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(5, 5))
        .parse_text("{0}: Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature.\n{0}: Put a +1/+1 counter on another target creature.")
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let recipient = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(5, 5))
        .build();
    let first = game.create_object_from_definition(&recipient, alice, Zone::Battlefield);
    let third = game.create_object_from_definition(&recipient, alice, Zone::Battlefield);
    let crate::ability::AbilityKind::Activated(ability) = &card.abilities[0].kind else {
        panic!("activated")
    };
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &ability.effects,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 3, "{requirements:#?}\n{card:#?}");
    assert!(requirements.iter().all(|requirement| {
        requirement
            .legal_targets
            .contains(&crate::game_state::Target::Object(source))
    }));
    game.stack.push(
        crate::game_state::StackEntry::ability(source, alice, ability.effects.clone())
            .with_targets(
                [first, source, third]
                    .map(crate::game_state::Target::Object)
                    .to_vec(),
            )
            .with_target_assignments(
                requirements
                    .iter()
                    .enumerate()
                    .map(|(index, requirement)| crate::game_state::TargetAssignment {
                        spec: requirement.spec.clone(),
                        range: index..index + 1,
                    })
                    .collect(),
            ),
    );
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    for (id, count) in [(first, 1), (source, 2), (third, 3)] {
        assert_eq!(
            game.object(id)
                .unwrap()
                .counters
                .get(&crate::CounterType::PlusOnePlusOne)
                .copied(),
            Some(count)
        );
    }
    let crate::ability::AbilityKind::Activated(single) = &card.abilities[1].kind else {
        panic!("activated")
    };
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &single.effects,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(source))
    );
}
