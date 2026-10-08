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
        Ok(prepare_face_up_instruction(self.clone(), ctx))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                crate::effects::composition::complete_prepared_original_with_outputs(
                    prepare_face_up_instruction(self.clone(), ctx),
                    game,
                    ctx,
                    false,
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "card to turn face up"
    }
}

#[derive(Debug)]
enum FaceUpInstructionState {
    Selection,
    Selected(Vec<crate::ids::ObjectId>),
    Ready(Vec<crate::ids::ObjectId>),
    Finished(EffectOutcome),
}

#[derive(Debug)]
struct PreparedFaceUpInstruction {
    effect: TurnFaceUpEffect,
    iterated_player: Option<crate::ids::PlayerId>,
    state: FaceUpInstructionState,
}

fn prepare_face_up_instruction(
    effect: TurnFaceUpEffect,
    ctx: &ExecutionContext,
) -> Box<dyn crate::effects::SimultaneousEffectProposal> {
    Box::new(PreparedFaceUpInstruction {
        effect,
        iterated_player: ctx.iteration.iterated_player,
        state: FaceUpInstructionState::Selection,
    })
}

fn authenticate_face_up_targets(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    targets: &[crate::ids::ObjectId],
) -> Result<bool, ExecutionError> {
    // Turning a selected face-down exile card face up is public, but
    // it is not the Reveal keyword action. Use the existing exact
    // owner-opening protocol without creating CardRevealed events.
    for index in 0..game.players.len() {
        let owner = crate::ids::PlayerId::from_index(index as u8);
        let exiled: Vec<_> = targets
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.owner == owner && object.zone == Zone::Exile)
                    && game.is_face_down(*id)
            })
            .collect();
        if exiled.is_empty() {
            continue;
        }
        let private: Vec<_> = exiled
            .iter()
            .copied()
            .filter(|id| game.hidden_identity_is_private(*id))
            .collect();
        let Some(opened) = game.reveal_private_hidden_cards_publicly(
            &mut *ctx.decision_maker,
            owner,
            ctx.source,
            &exiled,
            "Turn selected exiled cards face up",
            false,
        ) else {
            return Ok(false);
        };
        if private
            .iter()
            .any(|id| !opened.contains(id) && !game.is_publicly_revealed_hidden_card(*id))
            || exiled.iter().any(|id| game.is_hidden_card_placeholder(*id))
        {
            return Err(ExecutionError::IncompleteEvidence(
                "turning exiled cards face up lacks an authenticated selected identity".into(),
            ));
        }
    }

    Ok(!ctx.decision_maker.awaiting_choice())
}

impl crate::effects::SimultaneousEffectProposal for PreparedFaceUpInstruction {
    fn has_simultaneous_originals(&self) -> bool {
        matches!(&self.state, FaceUpInstructionState::Ready(targets) if targets.len() > 1)
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if !matches!(self.state, FaceUpInstructionState::Selection)
            || ctx.decision_maker.awaiting_choice()
        {
            return Ok(());
        }
        ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            game.clear_pending_decision_controllers();
            game.refresh_continuous_state()
                .map_err(ExecutionError::ContinuousDiscovery)?;
            let targets = crate::effects::helpers::resolve_objects_for_effect(
                game,
                ctx,
                &self.effect.target,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            self.state = if targets.is_empty() {
                FaceUpInstructionState::Finished(EffectOutcome::target_invalid())
            } else {
                FaceUpInstructionState::Selected(targets)
            };
            Ok(())
        })
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_selection(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if let FaceUpInstructionState::Selected(targets) = &self.state {
            let ready = ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
                authenticate_face_up_targets(game, ctx, targets)
            })?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            if !ready {
                self.state = FaceUpInstructionState::Finished(EffectOutcome::count(0));
            } else {
                let FaceUpInstructionState::Selected(targets) =
                    std::mem::replace(&mut self.state, FaceUpInstructionState::Selection)
                else {
                    unreachable!()
                };
                self.state = FaceUpInstructionState::Ready(targets);
            }
        }
        Ok(())
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            let targets = match self.state {
                FaceUpInstructionState::Finished(outcome) => {
                    return Ok(crate::effects::SimultaneousEffectCommit::finished(
                        crate::effects::CompletedEffectOutputs::aggregate_only(outcome),
                    ));
                }
                FaceUpInstructionState::Ready(targets) => targets,
                _ => {
                    return Err(ExecutionError::InternalError(
                        "face-up original committed before selection and authentication".into(),
                    ));
                }
            };
            let ((turned, completed, children), observations) =
                crate::effects::with_action_observations(game, |game| {
                    let mut turned = 0;
                    let mut completed = Vec::new();
                    let mut children = Vec::new();
                    for object_id in targets {
                        if !game.is_face_down(object_id) {
                            continue;
                        }
                        let Some(transition) = super::turn_face_up_with_choices(
                            game,
                            object_id,
                            super::FaceUpChoiceController::CurrentOr(ctx.controller),
                            &mut *ctx.decision_maker,
                        )?
                        else {
                            continue;
                        };
                        turned += 1;
                        children.extend(transition.outputs);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok((turned, completed, children));
                        }
                        if transition.was_on_battlefield {
                            let event_provenance = game.alloc_child_event_provenance(
                                ctx.provenance,
                                crate::events::EventKind::TurnedFaceUp,
                            );
                            let mut event = TriggerEvent::new_with_provenance(
                                crate::events::TurnedFaceUpEvent::new(object_id, ctx.controller),
                                event_provenance,
                            );
                            if let Some(batch) = game.simultaneous_action_batch() {
                                event = event.with_simultaneous_batch(batch);
                            }
                            completed.push(event);
                        }
                    }

                    Ok((turned, completed, children))
                })?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(turned),
            );
            outputs.retain_published_children(children);
            Ok(crate::effects::SimultaneousEffectCommit {
                outcome: outputs,
                completion: Some(Box::new(FaceUpInstructionCompletion {
                    events: completed,
                    observations,
                    provenance: ctx.provenance,
                    frozen: false,
                    observed: false,
                })),
            })
        })
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let receipt = self.commit_original_with_outputs(game, ctx)?;
        crate::effects::composition::complete_standalone_original_with_outputs(game, ctx, receipt)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

/// Lifecycle observation belongs after every sibling physical original and
/// before any completion programme. Immediate characteristic choices stay in
/// the original transition; this owner never repeats them.
struct FaceUpInstructionCompletion {
    events: Vec<TriggerEvent>,
    observations: Vec<TriggerEvent>,
    provenance: crate::provenance::ProvNodeId,
    frozen: bool,
    observed: bool,
}

impl crate::effects::SimultaneousEffectCompletion for FaceUpInstructionCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if self.frozen {
            return Err(ExecutionError::InternalError(
                "face-up completion frozen twice".into(),
            ));
        }
        crate::events::other::retain_departed_lifecycle_snapshots(
            game,
            &mut self.events,
            &self.observations,
        );
        crate::events::other::freeze_completed_lifecycle_events(game, &mut self.events)?;
        self.frozen = true;
        Ok(())
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
        _original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        if self.observed {
            return Ok(());
        }
        if !self.frozen {
            return Err(ExecutionError::InternalError(
                "face-up completion observed before freezing originals".into(),
            ));
        }
        crate::effects::observe_lifecycle_completions(game, &mut self.events)?;
        for event in &self.events {
            game.queue_trigger_event(self.provenance, event.clone());
        }
        self.observed = true;
        Ok(())
    }

    fn complete(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        mut original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.observe_original(game, ctx, &mut original)?;
        Ok(original)
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
