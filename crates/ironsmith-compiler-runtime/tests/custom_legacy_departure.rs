#[path = "custom_final_next_wave_support.rs"]
mod source_support;
// Generic selected sacrifice prices compose origin, payment and discount riders.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions(name: &str, text: &str, payment: Option<&str>) -> [CardDefinition; 2] {
    source_support::definitions(&serde_json::json!({
        "name":name,"executable_text":text,
        "custom_keyword_identities":payment.into_iter().collect::<Vec<_>>(),
        "custom_keyword_payments":payment.into_iter().collect::<Vec<_>>()
    }))
    .unwrap()
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn act(game: &mut GameState, action: LegalAction) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            return;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("unfinished action");
}
fn settle(game: &mut GameState) {
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..32 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("unsettled");
}
const LEGACY: &str = "Mana cost: {0}\nType: Enchantment\nYou may cast this card from your graveyard, but not from anywhere else.\nIf this spell would leave the stack other than being put onto the battlefield, exile it instead.\nIf this permanent would leave the battlefield, exile it instead.";
#[test]
fn independent_legacy_statics_allow_entry_and_replace_stack_and_battlefield_departures() {
    for definition in definitions("Independent departure rules", LEGACY, None) {
        for countered in [false, true] {
            let mut game = game();
            for zone in [Zone::Hand, Zone::Exile] {
                let card = game.create_object_from_definition(&definition, A, zone);
                assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell {spell_id,..} if *spell_id==card)));
            }
            let card = game.create_object_from_definition(&definition, A, Zone::Graveyard);
            let stable = game.object(card).unwrap().stable_id;
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell {spell_id,..}if *spell_id==card)).unwrap();
            act(&mut game, action);
            let stack = game.find_object_by_stable_id(stable).unwrap();
            if countered {
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::CounterEffect::new(
                        ChooseSpec::SpecificObject(stack),
                    )),
                    &mut EffectContext::new_default(stack, A),
                )
                .unwrap();
            }
            settle(&mut game);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone,
                if countered {
                    Zone::Exile
                } else {
                    Zone::Battlefield
                }
            );
            if !countered {
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(
                        ChooseSpec::SpecificObject(current),
                    )),
                    &mut EffectContext::new_default(current, A),
                )
                .unwrap();
                let current = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(current).unwrap().zone, Zone::Exile);
            }
        }
    }
}
#[test]
fn battlefield_departure_replacement_disappears_with_its_source_ability() {
    let text = LEGACY.replace(
        "Type: Enchantment",
        "Type: Enchantment Creature — Spirit\nPower/Toughness: 2/2",
    );
    for definition in definitions("Removable departure rule", &text, None) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let loss = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Ability removal",
            "Type: Instant\nTarget creature loses all abilities until end of turn.",
            false,
        )
        .unwrap();
        let effects = loss.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::SequenceEffect::new(effects)),
            &mut EffectContext::new_default(source, A)
                .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(source)]),
        )
        .unwrap();
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(
                ChooseSpec::SpecificObject(source),
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(current).unwrap().zone,
            Zone::Hand,
            "a static replacement cannot survive losing its source ability"
        );
    }
}
fn source_faces(game: &mut GameState, row_id: &str, artifact_route: bool) -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-legacy.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == row_id)
        .unwrap();
    let card = &row["representative_cards"][0];
    assert_eq!(card["source_shape"], "doubleface");
    assert_eq!(card["source_layout"], "modal_dfc");
    let faces = source_support::linked_definitions(card).unwrap();
    assert_eq!(faces.len(), 2);
    let route = usize::from(artifact_route);
    let pair = [faces[0][route].clone(), faces[1][route].clone()];
    assert!(!pair[0].card.transforming_dfc && !pair[1].card.transforming_dfc);
    assert_eq!(pair[0].card.other_face, Some(pair[1].card.id));
    assert_eq!(pair[1].card.other_face, Some(pair[0].card.id));
    for face in &pair {
        game.register_linked_face_definition(face);
    }
    pair
}
fn fund(game: &mut GameState) {
    for symbol in [
        ManaSymbol::Colorless,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
    }
}
fn is_back(method: &CastingMethod) -> bool {
    matches!(
        method.origin_method().without_exact_permission(),
        CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. }
    )
}
#[test]
fn both_complete_source_modal_faces_use_their_own_zone_permissions_and_departure_rules() {
    for row in ["msem-806", "msem-807"] {
        for route in [false, true] {
            for countered in [false, true] {
                let mut game = game();
                let [front, back] = source_faces(&mut game, row, route);
                fund(&mut game);
                let target = ironsmith_compiler_runtime::compile_to_runtime_definition(
                    "Opposing target",
                    "Type: Creature — Bear\nPower/Toughness: 20/20",
                    false,
                )
                .unwrap();
                game.create_object_from_definition(&target, PlayerId(1), Zone::Battlefield);
                // Outside the stack/battlefield the physical card is its front face.
                let hand = game.create_object_from_definition(&front, A, Zone::Hand);
                let hand_actions = compute_legal_actions(&game, A).unwrap();
                assert!(hand_actions.iter().any(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==hand&&!is_back(casting_method))));
                assert!(!hand_actions.iter().any(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==hand&&is_back(casting_method))),"Legacy back cannot be cast from hand");
                let exile = game.create_object_from_definition(&front, A, Zone::Exile);
                assert!(
                    !compute_legal_actions(&game, A).unwrap().iter().any(
                        |a| matches!(a,LegalAction::CastSpell {spell_id,..}if *spell_id==exile)
                    )
                );
                let graveyard = game.create_object_from_definition(&front, A, Zone::Graveyard);
                let stable = game.object(graveyard).unwrap().stable_id;
                let options = compute_legal_actions(&game, A).unwrap();
                assert!(!options.iter().any(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==graveyard&&!is_back(casting_method))),"front receives no graveyard permission from its back");
                let action=options.into_iter().find(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==graveyard&&is_back(casting_method))).expect("own back-face graveyard permission");
                act(&mut game, action);
                let stack = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(stack).unwrap().name, back.card.name);
                if countered {
                    execute_effect(
                        &mut game,
                        &Effect::new(ironsmith::effects::CounterEffect::new(
                            ChooseSpec::SpecificObject(stack),
                        )),
                        &mut EffectContext::new_default(stack, A),
                    )
                    .unwrap();
                }
                settle(&mut game);
                let current = game.find_object_by_stable_id(stable).unwrap();
                let permanent = row == "msem-806" && !countered;
                assert_eq!(
                    game.object(current).unwrap().zone,
                    if permanent {
                        Zone::Battlefield
                    } else {
                        Zone::Exile
                    }
                );
                if permanent {
                    assert_eq!(game.object(current).unwrap().name, back.card.name);
                    execute_effect(
                        &mut game,
                        &Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(
                            ChooseSpec::SpecificObject(current),
                        )),
                        &mut EffectContext::new_default(current, A),
                    )
                    .unwrap();
                    let current = game.find_object_by_stable_id(stable).unwrap();
                    assert_eq!(game.object(current).unwrap().zone, Zone::Exile);
                }
            }
        }
    }
}
#[test]
fn source_front_spell_is_not_subject_to_its_legacy_backs_replacement() {
    for row in ["msem-806", "msem-807"] {
        for route in [false, true] {
            let mut game = game();
            let [front, _] = source_faces(&mut game, row, route);
            fund(&mut game);
            let card = game.create_object_from_definition(&front, A, Zone::Hand);
            let stable = game.object(card).unwrap().stable_id;
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==card&&!is_back(casting_method))).unwrap();
            act(&mut game, action);
            let stack = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(stack).unwrap().name, front.card.name);
            execute_effect(
                &mut game,
                &Effect::new(ironsmith::effects::CounterEffect::new(
                    ChooseSpec::SpecificObject(stack),
                )),
                &mut EffectContext::new_default(stack, A),
            )
            .unwrap();
            settle(&mut game);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone,
                Zone::Graveyard,
                "Legacy belongs exclusively to back face"
            );
        }
    }
}
#[test]
fn complete_source_legacy_enchantments_departure_static_can_be_removed() {
    for route in [false, true] {
        let mut game = game();
        let [front, _] = source_faces(&mut game, "msem-806", route);
        fund(&mut game);
        let card = game.create_object_from_definition(&front, A, Zone::Graveyard);
        let stable = game.object(card).unwrap().stable_id;
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==card&&is_back(casting_method))).unwrap();
        act(&mut game, action);
        settle(&mut game);
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
        let removal = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Remove permanent abilities",
            "Type: Instant\nTarget permanent loses all abilities until end of turn.",
            false,
        )
        .unwrap();
        let effects = removal.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::SequenceEffect::new(effects)),
            &mut EffectContext::new_default(permanent, A)
                .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(permanent)]),
        )
        .unwrap();
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(
                ChooseSpec::SpecificObject(permanent),
            )),
            &mut EffectContext::new_default(permanent, A),
        )
        .unwrap();
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, Zone::Hand);
        assert_eq!(
            game.object(current).unwrap().name,
            front.card.name,
            "zone transition restores physical front face"
        );
    }
}
#[test]
fn legacy_stack_bounce_and_all_illegal_targets_cleanup_are_exiled() {
    for route in [false, true] {
        for fizzle in [false, true] {
            let mut game = game();
            let [front, _] = source_faces(
                &mut game,
                if fizzle { "msem-807" } else { "msem-806" },
                route,
            );
            fund(&mut game);
            let target_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Only damage target",
                "Type: Creature — Bear\nPower/Toughness: 4/4",
                false,
            )
            .unwrap();
            let target = game.create_object_from_definition(
                &target_definition,
                PlayerId(1),
                Zone::Battlefield,
            );
            let card = game.create_object_from_definition(&front, A, Zone::Graveyard);
            let stable = game.object(card).unwrap().stable_id;
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==card&&is_back(casting_method))).unwrap();
            act(&mut game, action);
            let stack = game.find_object_by_stable_id(stable).unwrap();
            if fizzle {
                // No other legal targets exist; remove the actual announced one.
                assert!(
                    game.stack
                        .last()
                        .unwrap()
                        .targets
                        .iter()
                        .any(|t| matches!(t,ironsmith::Target::Object(id)if *id==target))
                );
                game.move_object_by_effect(target, Zone::Exile).unwrap();
            } else {
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(
                        ChooseSpec::SpecificObject(stack),
                    )),
                    &mut EffectContext::new_default(stack, A),
                )
                .unwrap();
            }
            settle(&mut game);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, Zone::Exile);
        }
    }
}
#[test]
fn complete_source_legacy_battlefield_destruction_is_exiled() {
    for route in [false, true] {
        let mut game = game();
        let [front, _] = source_faces(&mut game, "msem-806", route);
        fund(&mut game);
        let card = game.create_object_from_definition(&front, A, Zone::Graveyard);
        let stable = game.object(card).unwrap().stable_id;
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell {spell_id,casting_method,..}if *spell_id==card&&is_back(casting_method))).unwrap();
        act(&mut game, action);
        settle(&mut game);
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::DestroyEffect::with_spec(
                ChooseSpec::SpecificObject(permanent),
            )),
            &mut EffectContext::new_default(permanent, A),
        )
        .unwrap();
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn legacy_selected_face_permission_diagnostic() {
    for route in [false, true] {
        let mut game = game();
        let [front, back] = source_faces(&mut game, "msem-806", route);
        fund(&mut game);
        let card = game.create_object_from_definition(&front, A, Zone::Graveyard);
        // A standalone back face must not activate the original game's
        // global zone-grant fast path and hide missing selected-face discovery.
        let mut independent_game = game.clone();
        let independent = independent_game.create_object_from_definition(&back, A, Zone::Graveyard);
        eprintln!(
            "LEGACY route={route} front_id={:?} back_id={:?} back_abilities={:?} back_alternatives={:?}",
            front.card.id, back.card.id, back.abilities, back.alternative_casts
        );
        eprintln!(
            "LEGACY independent_back_grants={:?}",
            independent_game.effect_store.grant_registry.granted_play_from_for_card(
                &independent_game,
                independent,
                Zone::Graveyard,
                A
            )
        );
        let mut proposed = game.clone();
        proposed
            .object_mut(card)
            .unwrap()
            .apply_definition_face(&back);
        eprintln!(
            "LEGACY proposed_characteristics={:?}",
            proposed.calculated_characteristics(card)
        );
        eprintln!(
            "LEGACY proposed_back_grants={:?}",
            proposed
                .effect_store
                .grant_registry
                .granted_play_from_for_card(&proposed, card, Zone::Graveyard, A)
        );
        eprintln!(
            "LEGACY proposed_actions={:?}",
            compute_legal_actions(&proposed, A)
        );
        let actions = compute_legal_actions(&game, A).unwrap();
        eprintln!("LEGACY physical_actions={actions:?}");
        assert!(actions.iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==card&&is_back(casting_method))),"printed back-face graveyard permission must authorize the selected face");
    }
}
