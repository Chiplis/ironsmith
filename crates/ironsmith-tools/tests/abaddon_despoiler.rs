//! Frozen atlas observation 24871528: canonical dynamic cascade grant.
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, default_cards_path,
    load_card_payloads_by_name,
};
#[test]
fn strict_abaddon_keeps_dynamic_cascade_threshold_in_rendered_text() {
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Abaddon the Despoiler",
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
    let text = snapshot.compiled_text.unwrap();
    assert!(!text.contains("a dynamic value"), "{text}");
    for required in [
        "During your turn",
        "from your hand",
        "your opponents have lost this turn",
        "or less",
        "cascade",
    ] {
        assert!(text.contains(required), "missing {required}: {text}");
    }
}

#[test]
fn cascade_threshold_uses_total_opponent_loss_and_only_your_turn() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decision::{
        GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
    };
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{CardType, GameState, PlayerId, Zone};
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Abaddon the Despoiler",
    )
    .unwrap();
    let def = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    for (bob_loss, cara_loss, regain, own_turn, mana_value, opponent_casts, origin, expected) in [
        (2, 0, 0, true, 3, false, Zone::Hand, 0),
        (3, 0, 0, true, 3, false, Zone::Hand, 1),
        (1, 2, 5, true, 3, false, Zone::Hand, 1),
        (3, 0, 0, false, 3, false, Zone::Hand, 0),
        (0, 0, 0, true, 3, false, Zone::Hand, 0),
        (0, 0, 0, true, 0, false, Zone::Hand, 1),
        (3, 0, 0, true, 3, true, Zone::Hand, 0),
        (3, 0, 0, true, 3, false, Zone::Exile, 0),
        (3, 0, 0, true, 3, false, Zone::Graveyard, 0),
        (4, 0, 0, true, 3, false, Zone::Hand, 2),
        (0, 0, 1, true, 0, false, Zone::Hand, 1),
    ] {
        let caster = if opponent_casts { bob } else { alice };
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.lose_life(bob, bob_loss);
        game.lose_life(cara, cara_loss);
        game.gain_life(bob, regain);
        game.lose_life(alice, 5);
        for (player, amount) in [(bob, bob_loss), (cara, cara_loss), (alice, 5)] {
            let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::LifeLossEvent::from_effect(player, amount),
                ironsmith::provenance::ProvNodeId::default(),
            );
            game.turn_store
                .turn_history
                .record_event(&event, None, None);
        }
        assert_eq!(
            game.turn_store
                .turn_history
                .total_life_lost_for_players(&[bob, cara]),
            bob_loss + cara_loss
        );
        game.turn.active_player = if own_turn { alice } else { bob };
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.priority_player = Some(caster);
        let fixture_builder = CardDefinitionBuilder::new(CardId::new(), "Threshold instant fixture")
            .card_types(vec![CardType::Instant]);
        let fixture_builder=if regain==1 {fixture_builder} else {fixture_builder.mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(mana_value)]))};
        let fixture = if expected==2 {fixture_builder.with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::cascade()))} else {fixture_builder}.build();
        let spell = game.create_object_from_definition(&fixture, caster, origin);
        if regain==1 {
            assert!(game.object(spell).unwrap().mana_cost.is_none());
            game.effect_store.grant_registry.grant_alternative_cast_to_card(spell,origin,caster,
                ironsmith::alternative_cast::AlternativeCastingMethod::Composed {
                    name:"Fixture free cast".into(),total_cost:ironsmith::cost::TotalCost::mana(ManaCost::new()),condition:None,prototype_power_toughness:None
                },ironsmith::grant_registry::GrantSource::Effect{source_id:source,expires_end_of_turn:game.turn.turn_number});
        }
        if origin != Zone::Hand {
            game.effect_store.grant_registry.grant_play_from_to_card(
                spell,
                origin,
                caster,
                Default::default(),
                ironsmith::grant_registry::GrantSource::Effect {
                    source_id: source,
                    expires_end_of_turn: game.turn.turn_number,
                },
            );
        }
        game.player_mut(caster)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, mana_value as u32);
        let action = compute_legal_actions(&game, caster)
            .into_iter()
            .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell))
            .unwrap();
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..24 {
            if !game.stack.is_empty() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}");
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1 + expected);
        let cast_events = game
            .turn_store
            .turn_history
            .event_records
            .iter()
            .chain(game.turn_store.turn_history.staged_event_records.iter())
            .filter(|r| {
                r.event
                    .downcast::<ironsmith::events::SpellCastEvent>()
                    .is_some()
            })
            .count();
        assert_eq!(
            cast_events, 1,
            "casting must finish before cascade inspection: {progress:?}"
        );
        let stack_object = game.object(game.stack[0].object_id).unwrap();
        if regain==1 {assert!(stack_object.mana_cost.is_none(),"alternative payment must not change printed mana cost");}
        let chars = game.calculated_characteristics(stack_object.id);
        let cascades = game
            .stack
            .iter()
            .filter(|entry| {
                entry.ability_effects.as_ref().is_some_and(|effects| {
                    effects.iter().any(|effect| {
                        effect
                            .downcast_ref::<ironsmith::effects::CascadeEffect>()
                            .is_some()
                    })
                })
            })
            .count();
        assert_eq!(
            cascades, expected,
            "loss={bob_loss}+{cara_loss}, regained={regain}, own_turn={own_turn}, stack={stack_object:#?}, derived={chars:#?}"
        );
        if mana_value==0 && expected==1 {
            assert!(game.player(alice).unwrap().library.is_empty());
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            assert_eq!(game.stack.len(),1,"empty library finishes cascade without a cast");
        }
        if opponent_casts {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            game.set_current_controller(source,bob);
            game.turn.active_player=bob;
            game.turn.priority_player=Some(bob);
            let next_spell=game.create_object_from_definition(&fixture,bob,Zone::Hand);
            game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Colorless,3);
            let action=compute_legal_actions(&game,bob).into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==next_spell)).unwrap();
            let mut state=PriorityLoopState::new(game.players_in_game());
            let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
            for _ in 0..24 {
                if !game.stack.is_empty(){break;}
                let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}");};
                progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
            }
            assert_eq!(game.stack.len(),2,"new controller's turn and opponents must determine grant");
            assert!(game.stack[1].ability_effects.as_ref().unwrap().iter().any(|effect|effect.downcast_ref::<ironsmith::effects::CascadeEffect>().is_some()));
        }
        if bob_loss == 3 && own_turn && !opponent_casts && origin == Zone::Hand {
            // Cascade must survive the loss of the permanent that granted it.
            game.move_object_by_effect(source, Zone::Graveyard);
            let cheap = CardDefinitionBuilder::new(CardId::new(), "Cascade cheaper fixture")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
                .build();
            let cheap_id = game.create_object_from_definition(&cheap, alice, Zone::Library);
            let cheap_stable = game.object(cheap_id).unwrap().stable_id;
            let equal_id = game.create_object_from_definition(&fixture, alice, Zone::Library);
            let equal_stable = game.object(equal_id).unwrap().stable_id;
            let land = CardDefinitionBuilder::new(CardId::new(), "Cascade land fixture")
                .card_types(vec![CardType::Land])
                .build();
            let land_id = game.create_object_from_definition(&land, alice, Zone::Library);
            let land_stable = game.object(land_id).unwrap().stable_id;
            assert_eq!(game.player(alice).unwrap().library.last(), Some(&land_id));
            for original_leaves in [false,true] {
                for decline in [false,true] {
                    let mut branch=game.clone();
                    if original_leaves {
                        let original=branch.stack.remove(0).object_id;
                        branch.move_object_by_effect(original,Zone::Graveyard);
                    }
                    if decline {
                        ironsmith::game_loop::resolve_stack_entry(&mut branch).unwrap();
                    } else {
                        ironsmith::game_loop::resolve_stack_entry_with(&mut branch,&mut dm).unwrap();
                    }
                    assert_eq!(branch.stack.len(),usize::from(!original_leaves)+usize::from(!decline),"original leaves={original_leaves}, decline={decline}");
                    let retained=branch.find_object_by_stable_id(cheap_stable).unwrap();
                    assert_eq!(branch.object(retained).unwrap().zone,if decline {Zone::Library} else {Zone::Stack});
                    for stable in [equal_stable,land_stable] {
                        let returned=branch.find_object_by_stable_id(stable).unwrap();
                        assert_eq!(branch.object(returned).unwrap().zone,Zone::Library);
                    }
                    assert_eq!(branch.player(alice).unwrap().library.len(),2+usize::from(decline));
                    assert_eq!(branch.player(alice).unwrap().mana_pool.total(),0,"cascade pays no mana");
                }
            }

        }
    }
}

#[test]
fn cascade_threshold_counts_chosen_x_on_the_stack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decision::{
        GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
    };
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{CardType, GameState, PlayerId, Zone};
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Abaddon the Despoiler",
    )
    .unwrap();
    let def = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (loss, expected, next_turn) in [(2, 0, false), (3, 1, false), (3, 0, true)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.lose_life(bob, loss);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::LifeLossEvent::from_effect(bob, loss),
            ironsmith::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
        if next_turn {
            game.turn_store.turn_history.clear_for_new_turn();
            game.turn.turn_number+=1;
            assert_eq!(game.player(bob).unwrap().life,17,"lost life remains reflected in life total");
        }
        game.turn.active_player = alice;
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.priority_player = Some(alice);
        let fixture = CardDefinitionBuilder::new(CardId::new(), "Variable instant fixture")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
            .build();
        let spell = game.create_object_from_definition(&fixture, alice, Zone::Hand);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell))
            .unwrap();
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..24 {
            if !game.stack.is_empty() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}");
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack[0].x_value, Some(3), "fixture must choose X=3");
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        let cascades = game
            .stack
            .iter()
            .filter(|entry| {
                entry.ability_effects.as_ref().is_some_and(|effects| {
                    effects.iter().any(|effect| {
                        effect
                            .downcast_ref::<ironsmith::effects::CascadeEffect>()
                            .is_some()
                    })
                })
            })
            .count();
        assert_eq!(cascades, expected, "opponent loss={loss}; X=3");
    }
}

#[test]
fn abaddon_trample_assigns_only_damage_beyond_lethal_to_player() {
    use ironsmith::{GameState,PlayerId,Zone,CardType};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::combat_state::{AttackTarget,AttackerInfo,CombatState};
    let payloads=load_card_payloads_by_name(default_cards_path().to_str().unwrap(),"Abaddon the Despoiler").unwrap();
    let def=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
    for toughness in [2,5,7] {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source=game.create_object_from_definition(&def,alice,Zone::Battlefield);
        assert_eq!(game.calculated_power(source),Some(5));
        assert_eq!(game.calculated_toughness(source),Some(5));
        let blocker=CardDefinitionBuilder::new(CardId::new(),"Trample blocker fixture")
            .card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(1,toughness)).build();
        let blocker=game.create_object_from_definition(&blocker,bob,Zone::Battlefield);
        let mut combat=CombatState::default();
        combat.attackers.push(AttackerInfo{creature:source,target:AttackTarget::Player(bob)});
        combat.blockers.insert(source,vec![blocker]);
        ironsmith::game_loop::execute_combat_damage_step(&mut game,&combat,false);
        assert_eq!(game.damage_on(blocker),std::cmp::min(5,toughness) as u32);
        assert_eq!(game.damage_on(source),1);
        assert_eq!(game.player(bob).unwrap().life,20-(5-toughness).max(0));
    }
}
