//! Frozen atlas observation 24743830: strict compilation and airbend behavior.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

fn definition() -> CardDefinition {
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Aang, Airbending Master",
    )
    .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
    let text = snapshot.compiled_text.as_deref().unwrap();
    assert!(text.contains("alternative casting cost of {2}"), "{text}");
    assert!(!text.contains("gains Airbend"), "{text}");
    assert!(!text.contains(",."), "{text}");
    let definition = compile_definition_from_payload(&payloads[0]).unwrap();
    assert_eq!(definition.abilities.len(), 3);
    assert!(
        definition
            .abilities
            .iter()
            .all(|a| matches!(a.kind, AbilityKind::Triggered(_)))
    );
    definition
}

#[test]
fn airbend_exiles_another_creature_and_grants_only_its_owner_the_exact_incarnation() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for controlled_by_alice in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature = CardDefinitionBuilder::new(CardId::new(), "Airbend subject fixture")
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
                ironsmith::mana::ManaSymbol::Generic(6),
            ]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .additional_cost(ironsmith::cost::TotalCost::from_cost(
                ironsmith::costs::Cost::life(3),
            ))
            .build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        if controlled_by_alice {
            game.set_current_controller(target, alice);
        }
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        assert!(game.object(target).is_none());
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        let grants = |game: &GameState, id, player| {
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(game, id, Zone::Exile, player)
        };
        assert_eq!(grants(&game, exiled, bob).len(), 1);
        // Exercise real casting from exile, including ordinary sorcery timing
        // and the alternative price rather than the printed six mana.
        {
            use ironsmith::decision::{GameProgress, LegalAction, compute_legal_actions};
            use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
            use ironsmith::game_state::Phase;
            use ironsmith::mana::ManaSymbol;
            let offered = |g: &GameState, player| {
                compute_legal_actions(g, player).into_iter()
                .find(|a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == exiled))
            };
            let mut cast_game = game.clone();
            cast_game.turn.active_player = alice;
            cast_game.turn.phase = Phase::FirstMain;
            cast_game.turn.priority_player = Some(bob);
            cast_game
                .player_mut(bob)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 2);
            assert!(
                offered(&cast_game, bob).is_none(),
                "no off-turn creature casting permission"
            );
            cast_game.turn.active_player = bob;
            cast_game.turn.phase = Phase::Combat;
            assert!(
                offered(&cast_game, bob).is_none(),
                "normal creature timing remains"
            );
            cast_game.turn.phase = Phase::FirstMain;
            let action =
                offered(&cast_game, bob).expect("owner can cast for two in own main phase");
            assert!(offered(&cast_game, alice).is_none());
            let mut insufficient = cast_game.clone();
            insufficient.player_mut(bob).unwrap().mana_pool = Default::default();
            insufficient
                .player_mut(bob)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 1);
            let mut insufficient_queue = TriggerQueue::new();
            let mut insufficient_state = PriorityLoopState::new(insufficient.players_in_game());
            let mut insufficient_dm = SelectFirstDecisionMaker;
            let mut attempt = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut insufficient,
                &mut insufficient_queue,
                &mut insufficient_state,
                &PriorityResponse::PriorityAction(action.clone()),
                &mut insufficient_dm,
            );
            for _ in 0..20 {
                let Ok(GameProgress::NeedsDecisionCtx(ctx)) = attempt else {
                    break;
                };
                attempt = ironsmith::game_loop::apply_decision_context_with_dm(
                    &mut insufficient,
                    &mut insufficient_queue,
                    &mut insufficient_state,
                    &ctx,
                    &mut insufficient_dm,
                );
            }
            assert!(
                insufficient.stack.is_empty(),
                "one mana cannot complete the two-mana cast"
            );
            let mut cast_queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(cast_game.players_in_game());
            let mut dm = SelectFirstDecisionMaker;
            let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut cast_game,
                &mut cast_queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            for _ in 0..20 {
                if !cast_game.stack.is_empty() {
                    break;
                }
                let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                    panic!("{progress:?}")
                };
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    &mut cast_game,
                    &mut cast_queue,
                    &mut state,
                    &ctx,
                    &mut dm,
                )
                .unwrap();
            }
            assert_eq!(cast_game.stack.len(), 1);
            assert_eq!(cast_game.player(bob).unwrap().mana_pool.total(), 0);
            assert_eq!(
                cast_game.player(bob).unwrap().life,
                17,
                "mandatory additional cost remains payable alongside airbend"
            );
            ironsmith::game_loop::resolve_stack_entry(&mut cast_game).unwrap();
            let recast = cast_game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(cast_game.object(recast).unwrap().zone, Zone::Battlefield);
            assert_eq!(cast_game.controller_of_id(recast), Some(bob));
        }

        assert!(grants(&game, exiled, alice).is_empty());
        assert_eq!(
            game.player(alice).unwrap().experience_counters,
            u32::from(controlled_by_alice)
        );
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(
            grants(&game, exiled, bob).len(),
            1,
            "source departure preserves permission"
        );
        let returned = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
        let exiled_again = game.move_object_by_effect(returned, Zone::Exile).unwrap();
        assert!(
            grants(&game, exiled_again, bob).is_empty(),
            "new exile incarnation has no old grant"
        );
    }
}

#[test]
fn upkeep_uses_trigger_controller_and_experience_at_resolution() {
    use ironsmith::object::ObjectKind;
    use ironsmith::types::Subtype;
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (controller, before, after, depart) in [
        (alice, 0, 0, false),
        (alice, 0, 3, false),
        (alice, 3, 1, true),
        (bob, 1, 2, false),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.set_current_controller(source, controller);
        game.player_mut(controller).unwrap().experience_counters = before;
        let other = if controller == alice { bob } else { alice };
        game.player_mut(other).unwrap().experience_counters = 7;
        let event = |player| {
            ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfUpkeepEvent::new(player),
                Default::default(),
            )
        };
        assert!(check_triggers(&game, &event(other)).is_empty());
        let entries = check_triggers(&game, &event(controller));
        assert_eq!(entries.len(), 1);
        let mut queue = TriggerQueue::new();
        for entry in entries {
            queue.add(entry);
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        game.player_mut(controller).unwrap().experience_counters = after;
        if depart {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        } else {
            game.set_current_controller(source, other);
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let tokens: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|o| o.kind == ObjectKind::Token)
            .collect();
        assert_eq!(tokens.len(), after as usize);
        for token in tokens {
            assert_eq!(game.controller_of(token), controller);
            assert_eq!(token.owner, controller);
            assert_eq!(token.colors(), ironsmith::color::ColorSet::WHITE);
            assert!(token.card_types.contains(&CardType::Creature));
            assert_eq!(token.subtypes.to_vec(), vec![Subtype::Ally]);
            assert_eq!(token.power(), Some(1));
            assert_eq!(token.toughness(), Some(1));
        }
    }
}

#[test]
fn experience_counts_controlled_creature_departures_but_not_deaths() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (owner, controller, creature, self_leaves, destination, expected) in [
        (alice, alice, true, false, Zone::Hand, 1),
        (bob, alice, true, false, Zone::Exile, 1),
        (alice, bob, true, false, Zone::Exile, 0),
        (alice, alice, false, false, Zone::Exile, 0),
        (alice, alice, true, false, Zone::Graveyard, 0),
        (alice, alice, true, true, Zone::Exile, 1),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let fixture = CardDefinitionBuilder::new(CardId::new(), "Departure fixture")
            .card_types(vec![if creature {
                CardType::Creature
            } else {
                CardType::Artifact
            }])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let target = if self_leaves {
            source
        } else {
            game.create_object_from_definition(&fixture, owner, Zone::Battlefield)
        };
        game.set_current_controller(target, controller);
        game.move_object_by_effect(target, destination).unwrap();
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), expected as usize);
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().experience_counters, expected);
        assert_eq!(game.player(bob).unwrap().experience_counters, 0);
    }
}

#[test]
fn one_or_more_departures_include_source_and_form_one_trigger_per_batch() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::{ChooseSpec, ObjectFilter};
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let fixture = CardDefinitionBuilder::new(CardId::new(), "Batch departure fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&fixture, alice, Zone::Battlefield);
    game.create_object_from_definition(&fixture, alice, Zone::Battlefield);
    game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, alice, &mut dm);
    execute_effect(
        &mut game,
        &Effect::move_to_zone(ChooseSpec::all(ObjectFilter::creature()), Zone::Exile, true),
        &mut ctx,
    )
    .unwrap();
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for entry in check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    assert_eq!(
        queue.entries.len(),
        1,
        "one or more creatures leave in the same action"
    );
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(game.player(alice).unwrap().experience_counters, 1);
    assert_eq!(game.player(bob).unwrap().experience_counters, 0);
}

#[test]
fn airbend_permission_casts_spell_faces_without_permitting_land_plays() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Phase;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (land_front, linked) in [(true, false), (false, true), (true, true)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let front_id = CardId::new();
        let back_id = CardId::new();
        let face = |id, other_id, land, name, other_name| {
            let mut builder = CardDefinitionBuilder::new(id, name).card_types(vec![if land {
                CardType::Land
            } else {
                CardType::Creature
            }]);
            if !land {
                builder = builder
                    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
                    .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2));
            }
            if linked {
                builder = builder
                    .other_face(other_id)
                    .other_face_name(other_name)
                    .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike);
            }
            builder.build()
        };
        let front = face(
            front_id,
            back_id,
            land_front,
            "Airbend face front",
            "Airbend face back",
        );
        let back = face(
            back_id,
            front_id,
            !land_front,
            "Airbend face back",
            "Airbend face front",
        );
        game.register_linked_face_definition(&front);
        if linked {
            game.register_linked_face_definition(&back);
        }
        let target = game.create_object_from_definition(&front, bob, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        if land_front {
            // Simulate a battlefield-only animation; zone movement reconstructs
            // the printed land face from its registered definition.
            game.object_mut(target)
                .unwrap()
                .card_types
                .push(CardType::Creature);
            game.object_mut(target).unwrap().base_power = Some(ironsmith::card::PtValue::Fixed(2));
            game.object_mut(target).unwrap().base_toughness =
                Some(ironsmith::card::PtValue::Fixed(2));
        }
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        game.move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap();
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        game.turn.active_player = bob;
        game.turn.phase = Phase::FirstMain;
        game.turn.priority_player = Some(bob);
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let actions = compute_legal_actions(&game, bob);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == exiled)),
            "airbend grants casting only: {land_front} {linked} {actions:?}"
        );
        assert_eq!(
            actions.iter().any(
                |a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled)
            ),
            linked,
            "spell face availability: {land_front} {linked} {actions:?}"
        );
        if linked {
            use ironsmith::decision::GameProgress;
            use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
            let action = actions
                .into_iter()
                .find(|a| {
                    matches!(a,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled)
                })
                .unwrap();
            let mut cast_queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(game.players_in_game());
            let mut dm = SelectFirstDecisionMaker;
            let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut cast_queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            for _ in 0..20 {
                if !game.stack.is_empty() {
                    break;
                }
                let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                    panic!("{progress:?}")
                };
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    &mut game,
                    &mut cast_queue,
                    &mut state,
                    &ctx,
                    &mut dm,
                )
                .unwrap();
            }
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let resolved = game.find_object_by_stable_id(stable).unwrap();
            let creature = game.object(resolved).unwrap();
            assert_eq!(creature.zone, Zone::Battlefield);
            assert!(creature.is_creature() && !creature.is_land());
            assert_eq!(
                creature.name.as_ref(),
                if land_front {
                    "Airbend face back"
                } else {
                    "Airbend face front"
                }
            );
            assert_eq!(game.controller_of(creature), bob);
        }
    }
}

#[test]
fn airbend_rechecks_target_legality_and_does_not_follow_a_new_incarnation() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for change_zone in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let fixture = CardDefinitionBuilder::new(CardId::new(), "Airbend invalidation fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let target = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        let retained = if change_zone {
            let hand = game.move_object_by_effect(target, Zone::Hand).unwrap();
            game.move_object_by_effect(hand, Zone::Battlefield).unwrap()
        } else {
            game.object_mut(target).unwrap().card_types = vec![CardType::Artifact].into();
            target
        };
        let bystander = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        for id in [source, retained, bystander] {
            assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
        }
        assert!(game.exile.is_empty());
        assert!(game.effect_store.grant_registry.grants.is_empty());
    }
}

#[test]
fn airbend_event_requires_an_actual_exile_and_keeps_the_trigger_controller() {
    use ironsmith::events::zones::matchers::WouldBeExiledMatcher;
    use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for replacement in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut fixture = CardDefinitionBuilder::new(CardId::new(), "Airbend replacement fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        if replacement == 3 {
            fixture.card.is_token = true;
        }
        let target = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        if replacement == 1 || replacement == 2 {
            let action = if replacement == 1 {
                ReplacementAction::Prevent
            } else {
                ReplacementAction::Instead(vec![ironsmith::Effect::move_to_zone(
                    ironsmith::target::ChooseSpec::SpecificObject(target),
                    Zone::Hand,
                    true,
                )])
            };
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    WouldBeExiledMatcher::new(ironsmith::filter::ObjectFilter::specific(target)),
                    action,
                ),
            );
        }
        // The source may leave before its ETB ability resolves.
        game.move_object_by_effect(source, Zone::Hand).unwrap();
        game.take_pending_trigger_events();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let history = &game.turn_store.turn_history;
        let airbends: Vec<_> = history
            .event_records
            .iter()
            .chain(history.staged_event_records.iter())
            .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::Airbend)
            .collect();
        assert_eq!(
            airbends.len(),
            usize::from(replacement == 0 || replacement == 3),
            "replacement {replacement}"
        );
        if let Some(event) = airbends.first() {
            assert_eq!(event.player, alice);
            assert_eq!(event.amount, 1);
        }
        if replacement == 3 {
            ironsmith::game_loop::run_priority_loop_with(
                &mut game,
                &mut queue,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert!(
                game.find_object_by_stable_id(stable).is_none(),
                "exiled token ceases to exist"
            );
            continue;
        }
        let retained = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(retained).unwrap().zone,
            match replacement {
                0 => Zone::Exile,
                1 => Zone::Battlefield,
                _ => Zone::Hand,
            }
        );
        assert_eq!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, retained, Zone::Exile, bob)
                .len(),
            usize::from(replacement == 0)
        );
    }
}
