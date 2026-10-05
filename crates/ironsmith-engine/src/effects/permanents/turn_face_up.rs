//! "Turn [the exiled card / it] face up." effect.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub type TurnFaceUpEffect = ironsmith_core::TurnFaceUpEffect;

impl EffectExecutor for TurnFaceUpEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
            let targets = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            if targets.is_empty() {
                return Ok(EffectOutcome::target_invalid());
            }

            // Turning a selected face-down exile card face up is public, but
            // it is not the Reveal keyword action. Use the existing exact
            // owner-opening protocol without creating CardRevealed events.
            for index in 0..game.players.len() {
                let owner = crate::ids::PlayerId::from_index(index as u8);
                let exiled: Vec<_> = targets.iter().copied().filter(|id|
                    game.object(*id).is_some_and(|object| object.owner == owner && object.zone == Zone::Exile)
                        && game.is_face_down(*id)).collect();
                if exiled.is_empty() { continue; }
                let private: Vec<_> = exiled.iter().copied().filter(|id| game.hidden_identity_is_private(*id)).collect();
                let Some(opened) = game.reveal_private_hidden_cards_publicly(
                    &mut *ctx.decision_maker, owner, ctx.source, &exiled,
                    "Turn selected exiled cards face up", false,
                ) else { return Ok(EffectOutcome::count(0)); };
                if private.iter().any(|id| !opened.contains(id) && !game.is_publicly_revealed_hidden_card(*id))
                    || exiled.iter().any(|id| game.is_hidden_card_placeholder(*id))
                {
                    return Err(ExecutionError::IncompleteEvidence(
                        "turning exiled cards face up lacks an authenticated selected identity".into(),
                    ));
                }
            }

            let mut turned = 0;
            let mut completed = Vec::new();
            for object_id in targets {
                let Some(object) = game.object(object_id) else {
                    continue;
                };
                if !game.is_face_down(object_id) {
                    continue;
                }
                let on_battlefield = object.zone == Zone::Battlefield;
                game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
                if !game.set_face_up(object_id).map_err(ExecutionError::ContinuousDiscovery)? {
                    continue;
                }
                game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
                turned += 1;
                if on_battlefield {
                    // CR 708.11: "As this is turned face up" abilities apply
                    // whatever turns the permanent face up, and a characteristic
                    // choice made "as it enters or is turned face up" (Aquamorph
                    // Entity) is made now, as in the special-action path.
                    let controller = game.current_controller(object_id).unwrap_or(ctx.controller);
                    game.execute_as_enters_effect_programs_for_turn_face_up(
                        object_id,
                        controller,
                        &mut *ctx.decision_maker,
                    )?;
                    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                    game.apply_power_toughness_choice_as_enters_or_turns_face_up(
                        object_id,
                        controller,
                        &mut *ctx.decision_maker,
                    );
                    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                    let event_provenance = game.alloc_child_event_provenance(
                        ctx.provenance,
                        crate::events::EventKind::TurnedFaceUp,
                    );
                    completed.push(TriggerEvent::new_with_provenance(
                        crate::events::TurnedFaceUpEvent::new(object_id, ctx.controller),
                        event_provenance,
                    ));
                }
            }

            // One instruction changes all selected permanents before any of
            // its event filters observe the completed characteristics.
            crate::events::other::freeze_completed_lifecycle_events(game, &mut completed)?;
            for event in completed { game.queue_trigger_event(ctx.provenance, event); }
            Ok(EffectOutcome::count(turned))
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
        }
        result
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "card to turn face up"
    }
}

#[cfg(test)]
mod exile_opening_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{SelectObjectsContext, SelectionRevealPolicy};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ChooseSpec;

    struct Opening { pause: bool, pending: bool, requested: Vec<Vec<ObjectId>> }
    impl DecisionMaker for Opening {
        fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
            assert_eq!(context.player, PlayerId::from_index(0));
            assert_eq!(context.reveal_policy, SelectionRevealPolicy::Public);
            let cards: Vec<_> = context.candidates.iter().map(|candidate| candidate.id).collect();
            assert_eq!(context.min, cards.len());
            assert_eq!(context.max, Some(cards.len()));
            self.requested.push(cards.clone()); self.pending = self.pause; cards
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }

    #[test]
    fn locally_known_exile_faces_still_require_a_complete_owner_opening() {
        struct Incomplete(usize);
        impl DecisionMaker for Incomplete {
            fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
                match self.0 {
                    0 => vec![],
                    1 => vec![context.candidates[0].id],
                    _ => vec![ObjectId::from_raw(9_999_999)],
                }
            }
        }
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(120_071), "Known private exile card")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        for answer in 0..3 {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let cards: Vec<_> = (0..2).map(|slot|
                game.create_hidden_card_placeholder(alice, Zone::Exile, slot, format!("malformed-{slot}"))).collect();
            for id in &cards { game.reveal_hidden_card_with_definition(*id, &definition).unwrap(); game.set_face_down(*id); }
            let snapshots = cards.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect();
            let source = game.new_object_id();
            let mut dm = Incomplete(answer);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.set_tagged_objects("pile", snapshots);
            let result = TurnFaceUpEffect { target: ChooseSpec::tagged("pile") }.execute(&mut game, &mut ctx);
            assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))));
            assert!(cards.iter().all(|id| game.is_face_down(*id) && !game.is_hidden_card_placeholder(*id)));
            assert!(game.publicly_revealed_hidden_cards().is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }

    #[test]
    fn turning_an_exile_group_face_up_uses_exact_openings_without_reveal_events() {
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(120_070), "Exile identity")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        for known in [false, true] {
            for pause in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let cards: Vec<_> = (0..4).map(|slot|
                    game.create_hidden_card_placeholder(alice, Zone::Exile, slot, format!("pile-{slot}"))).collect();
                if known {
                    for card in &cards[..2] { game.reveal_hidden_card_with_definition(*card, &definition).unwrap(); }
                }
                for card in &cards { game.set_face_down(*card); }
                let source = game.new_object_id();
                let selected = cards[..2].iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect();
                let mut dm = Opening { pause, pending: false, requested: vec![] };
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                ctx.set_tagged_objects("selected_pile", selected);
                let effect = TurnFaceUpEffect { target: ChooseSpec::tagged("selected_pile") };
                let result = effect.execute(&mut game, &mut ctx);
                if pause || !known {
                    assert!(cards.iter().all(|id| game.is_face_down(*id)));
                    assert!(game.publicly_revealed_hidden_cards().is_empty());
                    if !pause { assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_)))); }
                } else {
                    assert_eq!(result.unwrap().count_or_zero(), 2);
                    assert!(cards[..2].iter().all(|id| !game.is_face_down(*id)));
                }
                assert!(cards[2..].iter().all(|id| game.is_face_down(*id) && game.is_hidden_card_placeholder(*id)));
                assert!(game.take_pending_trigger_events().is_empty(), "no reveal or battlefield turn-up event for exile cards");
                drop(ctx);
                assert_eq!(dm.requested, vec![cards[..2].to_vec()]);
                if pause || !known {
                    if !known { for card in &cards[..2] { game.reveal_hidden_card_with_definition(*card, &definition).unwrap(); } }
                    dm.pause = false; dm.pending = false;
                    let selected = cards[..2].iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect();
                    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                    ctx.set_tagged_objects("selected_pile", selected);
                    let result = effect.execute(&mut game, &mut ctx).unwrap();
                    assert_eq!(result.count_or_zero(), 2);
                    assert!(result.events.is_empty());
                    assert!(cards[..2].iter().all(|id| !game.is_face_down(*id)));
                    assert!(cards[2..].iter().all(|id| game.is_face_down(*id) && game.is_hidden_card_placeholder(*id)));
                }
            }
        }
    }
}

#[cfg(test)]
mod replacement_program_boundary_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId};

    struct Answers { pause: bool, calls: usize, pending: bool }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &GameState,
            _: &crate::decisions::context::BooleanContext) -> bool {
            assert!(!self.pending, "execution must stop on the first pending choice");
            self.calls += 1;
            self.pending = self.pause && self.calls == 2;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }

    fn setup(programs: Vec<Vec<Effect>>) -> (GameState, PlayerId, ObjectId,
        crate::replacement::ReplacementEffectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let mut definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Face-up boundary fixture")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2));
        for effects in programs {
            definition = definition.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::from_model(
                    crate::static_abilities::CompiledStaticAbility::as_turns_face_up_effect_program(
                        effects.into(), "this creature", None))));
        }
        let target = game.create_object_from_definition(&definition.build(), alice, Zone::Battlefield);
        assert!(game.set_face_down(target));
        game.refresh_continuous_state().unwrap();
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(target, alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        game.take_pending_trigger_events();
        (game, alice, target, shield)
    }

    fn run(game: &mut GameState, ctx: &mut ExecutionContext, generic: bool)
        -> Result<EffectOutcome, ExecutionError> {
        let effect = TurnFaceUpEffect::new(crate::target::ChooseSpec::Source);
        if generic { crate::effects::execute_effect(game, &Effect::new(effect), ctx) }
        else { effect.execute(game, ctx) }
    }

    #[test]
    fn face_up_program_error_preserves_error_face_state_and_prior_program_work() {
        let mut observations = Vec::new();
        for generic in [false, true] {
            for split in [false, true] {
                let programs = if split { vec![vec![Effect::gain_life(2)], vec![Effect::lose_life(Value::X)]] }
                    else { vec![vec![Effect::gain_life(2), Effect::lose_life(Value::X)]] };
                let (mut game, alice, target, shield) = setup(programs);
                let revision = game.effect_store.continuous_effects.revision();
                let ui_count = game.ui_effect_events().count();
                for attempt in 0..2 {
                    let mut ctx = ExecutionContext::new_default(target, alice);
                    let result = run(&mut game, &mut ctx, generic);
                    observations.push((generic, split, attempt, format!("{result:?}"),
                        matches!(result, Err(ExecutionError::UnresolvableValue(_))),
                        game.is_face_down(target)
                            && game.object(target).unwrap().face_down_cast_state.is_some()
                            && game.player(alice).unwrap().life == 20
                            && game.effect_store.replacement_effects.get_effect(shield).is_some()
                            && game.effect_store.pending_trigger_events.is_empty()
                            && game.effect_store.continuous_effects.revision() == revision
                            && game.ui_effect_events().count() == ui_count
                            && ctx.replacement.suppressed_replacement_effects.is_empty()
                            && ctx.replacement.suppressed_replacement_effect_keys.is_empty()));
                }
            }
        }
        assert!(observations.iter().all(|(_, _, _, _, error, state)| *error && *state), "{observations:#?}");
    }

    #[test]
    fn face_up_program_pause_restores_entire_action_and_replays_once() {
        let mut observations = Vec::new();
        for generic in [false, true] {
            for split in [false, true] {
                let before = vec![Effect::gain_life(2), Effect::may(vec![Effect::gain_life(1)])];
                let after = vec![Effect::may(vec![Effect::gain_life(3)])];
                let programs = if split { vec![before, after] }
                    else { vec![before.into_iter().chain(after).collect()] };
                let (mut game, alice, target, shield) = setup(programs);
                let revision = game.effect_store.continuous_effects.revision();
                let ui_count = game.ui_effect_events().count();
                let mut dm = Answers { pause: true, calls: 0, pending: false };
                let mut ctx = ExecutionContext::new(target, alice, &mut dm);
                let result = run(&mut game, &mut ctx, generic).unwrap();
                observations.push((generic, split, "pause", result.count_or_zero() == 0
                    && ctx.decision_maker.awaiting_choice() && game.is_face_down(target)
                    && game.object(target).unwrap().face_down_cast_state.is_some()
                    && game.player(alice).unwrap().life == 20
                    && game.effect_store.replacement_effects.get_effect(shield).is_some()
                    && game.effect_store.pending_trigger_events.is_empty()
                    && game.effect_store.continuous_effects.revision() == revision
                    && game.ui_effect_events().count() == ui_count));
                let mut replay = Answers { pause: false, calls: 0, pending: false };
                let mut ctx = ExecutionContext::new(target, alice, &mut replay);
                let result = run(&mut game, &mut ctx, generic).unwrap();
                observations.push((generic, split, "replay", result.count_or_zero() == 1
                    && !ctx.decision_maker.awaiting_choice() && !game.is_face_down(target)
                    && game.object(target).unwrap().face_down_cast_state.is_none()
                    && game.player(alice).unwrap().life == 28
                    && game.effect_store.replacement_effects.get_effect(shield).is_none()
                    && game.effect_store.pending_trigger_events.len() == 4));
            }
        }
        assert!(observations.iter().all(|(_, _, _, correct)| *correct), "{observations:#?}");
    }

    #[derive(Debug, Clone)]
    struct FaceUpDiscoveryOverflow;
    impl crate::static_abilities::StaticAbilityKind for FaceUpDiscoveryOverflow {
        fn id(&self) -> crate::static_abilities::StaticAbilityId {
            crate::static_abilities::StaticAbilityId::Anthem
        }
        fn display(&self) -> String { "Face-up discovery boundary fixture".into() }
        fn generate_effects(&self, source: ObjectId, controller: PlayerId, _: &GameState)
            -> Vec<crate::continuous::ContinuousEffect> {
            (0..16_385).map(|_| crate::continuous::ContinuousEffect::new(source, controller,
                crate::continuous::EffectTarget::Source, crate::continuous::Modification::ModifyPower(1))).collect()
        }
    }

    #[test]
    fn primitive_face_up_discovery_failure_preserves_overlay_and_all_notifications() {
        let mut observations = Vec::new();
        for hidden_producer in [false, true] {
            let (mut game, alice, target, shield) = setup(vec![]);
            let producer = if hidden_producer { target } else {
                game.create_object_from_definition(&crate::cards::CardDefinitionBuilder::new(
                    CardId::new(), "Independent discovery boundary fixture")
                    .card_types(vec![crate::types::CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(1, 1)).build(),
                    alice, Zone::Battlefield)
            };
            let overflow = crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::new(FaceUpDiscoveryOverflow));
            let object = game.object_mut(producer).expect("producer exists");
            if hidden_producer {
                std::sync::Arc::make_mut(&mut object.face_down_cast_state.as_mut()
                    .expect("face-down overlay exists").abilities).push(overflow);
            } else {
                std::sync::Arc::make_mut(&mut object.abilities).push(overflow);
            }
            let revision = game.effect_store.continuous_effects.revision();
            let ui_count = game.ui_effect_events().count();
            let timestamp = game.effect_store.continuous_effects.get_entry_timestamp(target);
            for attempt in 0..2 {
                let returned = game.set_face_up(target);
                assert!(matches!(&returned, Err(crate::static_ability_processor::StaticEffectDiscoveryError::EffectLimit {
                    maximum: 16_384, completed_rounds: 0 })), "{returned:?}");
                observations.push((hidden_producer, attempt, format!("{returned:?}"),
                    game.is_face_down(target)
                        && game.object(target).expect("target exists").face_down_cast_state.is_some()
                        && game.effect_store.continuous_effects.revision() == revision
                        && game.effect_store.continuous_effects.get_entry_timestamp(target) == timestamp
                        && game.ui_effect_events().count() == ui_count
                        && game.effect_store.pending_trigger_events.is_empty()
                        && game.player(alice).expect("player exists").life == 20
                        && game.effect_store.replacement_effects.get_effect(shield).is_some()));
            }
            let object = game.object_mut(producer).expect("producer exists for retry");
            if hidden_producer {
                std::sync::Arc::make_mut(&mut object.face_down_cast_state.as_mut()
                    .expect("failed primitive retained the overlay").abilities).clear();
            } else { std::sync::Arc::make_mut(&mut object.abilities).clear(); }
            assert!(game.set_face_up(target).expect("corrected graph allows retry"));
            assert!(!game.is_face_down(target));
            assert!(game.object(target).expect("target exists").face_down_cast_state.is_none());
            assert!(game.take_pending_trigger_events().is_empty());
        }
        assert!(observations.iter().all(|(_, _, _, unchanged)| *unchanged), "{observations:#?}");
    }

    #[test]
    fn face_up_query_failures_reach_costs_actions_and_prepared_prompts_without_mutation() {
        use crate::static_ability_processor::StaticEffectDiscoveryError;
        use crate::special_actions::TurnFaceUpMethod;
        for hidden_producer in [false, true] {
            let (mut game, alice, target, shield) = setup(vec![]);
            game.turn.priority_player = Some(alice);
            let morph = crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::morph(
                crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(
                    vec![vec![crate::mana::ManaSymbol::Generic(2)]]))));
            std::sync::Arc::make_mut(&mut game.object_mut(target).expect("target exists")
                .face_down_cast_state.as_mut().expect("overlay exists").abilities).push(morph);
            let producer = if hidden_producer { target } else {
                game.create_object_from_definition(&crate::cards::CardDefinitionBuilder::new(CardId::new(),
                    "Query discovery boundary fixture").card_types(vec![crate::types::CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(1, 1)).build(), alice, Zone::Battlefield)
            };
            let overflow = crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::new(FaceUpDiscoveryOverflow));
            let object = game.object_mut(producer).expect("producer exists");
            if hidden_producer {
                std::sync::Arc::make_mut(&mut object.face_down_cast_state.as_mut().expect("overlay exists").abilities).push(overflow);
            } else { std::sync::Arc::make_mut(&mut object.abilities).push(overflow); }
            let revision = game.effect_store.continuous_effects.revision();
            let ui_count = game.ui_effect_events().count();
            let action = crate::decision::LegalAction::TurnFaceUp {
                creature_id: target, method: TurnFaceUpMethod::TurnFaceUpAbility };
            for _ in 0..2 {
                assert!(matches!(crate::special_actions::turn_face_up_cost_display(&game, target,
                    TurnFaceUpMethod::TurnFaceUpAbility), Err(StaticEffectDiscoveryError::EffectLimit {
                        maximum: 16_384, completed_rounds: 0 })));
                assert!(matches!(crate::special_actions::available_turn_face_up_methods(&game, target),
                    Err(StaticEffectDiscoveryError::EffectLimit { maximum: 16_384, completed_rounds: 0 })));
                assert!(matches!(crate::decision::compute_legal_actions(&game, alice),
                    Err(ExecutionError::ContinuousDiscovery(StaticEffectDiscoveryError::EffectLimit {
                        maximum: 16_384, completed_rounds: 0 }))));
                assert!(matches!(crate::decision::compute_actions_for_source(&game, alice, Some(target)),
                    Err(ExecutionError::ContinuousDiscovery(StaticEffectDiscoveryError::EffectLimit {
                        maximum: 16_384, completed_rounds: 0 }))));
                assert!(matches!(crate::decisions::context::PriorityContext::new(&game, alice, vec![action.clone()]),
                    Err(StaticEffectDiscoveryError::EffectLimit { maximum: 16_384, completed_rounds: 0 })));
                assert!(matches!(crate::decisions::specs::PrioritySpec::new(&game, vec![action.clone()]),
                    Err(StaticEffectDiscoveryError::EffectLimit { maximum: 16_384, completed_rounds: 0 })));
                assert!(matches!(crate::game_loop::analyze_priority_context(&game, alice),
                    Err(crate::game_loop::GameLoopError::ExecutionFailed(ExecutionError::ContinuousDiscovery(
                        StaticEffectDiscoveryError::EffectLimit { maximum: 16_384, completed_rounds: 0 })))));
                assert!(game.is_face_down(target));
                assert!(game.object(target).expect("target exists").face_down_cast_state.is_some());
                assert_eq!(game.effect_store.continuous_effects.revision(), revision);
                assert_eq!(game.ui_effect_events().count(), ui_count);
                assert_eq!(game.player(alice).expect("player exists").life, 20);
                assert!(game.effect_store.pending_trigger_events.is_empty());
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            }
            let object = game.object_mut(producer).expect("producer exists for correction");
            if hidden_producer {
                std::sync::Arc::make_mut(&mut object.face_down_cast_state.as_mut().expect("overlay exists").abilities).truncate(1);
            } else { std::sync::Arc::make_mut(&mut object.abilities).clear(); }
            assert_eq!(crate::special_actions::turn_face_up_cost_display(&game, target,
                TurnFaceUpMethod::TurnFaceUpAbility).expect("corrected cost query completes"), Some("{2}".into()));
            let context = crate::decisions::context::PriorityContext::new(&game, alice, vec![action.clone()])
                .expect("corrected action preparation completes");
            let (_, cost) = context.actions.iter_with_face_up_costs().next().expect("one prepared action");
            assert_eq!(cost, Some("{2}"));
            assert!(game.is_face_down(target), "successful hypothetical query must not commit the face");
            assert!(crate::decision::compute_actions_for_source(&game, alice, Some(target)).is_ok());
        }
    }
}
