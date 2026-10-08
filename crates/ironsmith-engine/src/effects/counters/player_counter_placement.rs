//! Commit player counters from the complete replacement event and retain its outcomes.

use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, MarkersChangedEvent, PutCountersEvent, downcast_event};
use crate::game_state::{GameState, Target};
use crate::object::CounterType;
use crate::triggers::TriggerEvent;

fn prevented() -> EffectOutcome {
    let mut outcome = EffectOutcome::prevented();
    outcome.value = OutcomeValue::Count(0);
    outcome
}

/// One owner for player counter eligibility. Zero placement is a successful
/// no-op; missing recipients and locks retain their existing order and policy.
pub(super) fn player_counter_request_outcome(
    game: &GameState,
    event: &Event,
) -> Result<Option<EffectOutcome>, ExecutionError> {
    let proposed = downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
        ExecutionError::InternalError("player counter placement requires a counter event".into())
    })?;
    let Target::Player(player) = proposed.target else {
        return Err(ExecutionError::InternalError(
            "player counter placement requires a player".into(),
        ));
    };
    if game.player(player).is_none() {
        return Err(ExecutionError::PlayerNotFound(player));
    }
    if proposed.count == 0 {
        return Ok(Some(EffectOutcome::count(0)));
    }
    if game
        .turn_store
        .turn_history
        .player_counter_is_locked_this_turn(player, proposed.counter_type)
        || (proposed.counter_type == CounterType::Poison && !game.can_get_poison_counters(player))
    {
        return Ok(Some(prevented()));
    }
    Ok(None)
}

pub(crate) fn execute_player_counter_placement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    execute_player_counter_placement_with_outputs(game, ctx, event)
        .map(CompletedEffectOutputs::into_outcome)
}

pub(crate) fn execute_player_counter_placement_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    game.clear_pending_decision_controllers();
    crate::effects::composition::execute_original_view_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx, pre_event_game| {
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            if let Some(outcome) = player_counter_request_outcome(game, &event)? {
                return Ok(CompletedEffectOutputs::aggregate_only(outcome));
            }
            let processed = process_trait_event_with_execution_context(game, event, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            commit_player_counter_placement_with_outputs(game, ctx, processed, pre_event_game)
        },
    )
}

pub(super) fn commit_player_counter_placement_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    processed: TraitEventResult,
    pre_event_game: &GameState,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    match processed {
        expanded @ TraitEventResult::Expanded { .. } => {
            crate::effects::replacement::execute_event_expansion_with_outputs(
                game,
                ctx,
                expanded,
                |game, ctx, result| {
                    commit_player_counter_placement_with_outputs(game, ctx, result, pre_event_game)
                },
                |_game, context, _original_outcome| {
                    let captured = downcast_event::<PutCountersEvent>(context.event.inner())
                        .ok_or_else(|| {
                            ExecutionError::InternalError(
                                "added counter program lost its captured event".into(),
                            )
                        })?;
                    let Target::Player(recipient) = captured.target else {
                        return Err(ExecutionError::InternalError(
                            "added counter program has an incompatible recipient".into(),
                        ));
                    };
                    Ok(crate::effects::replacement::ReplacementProgramBindings {
                        targets: Some(vec![ResolvedTarget::Player(recipient)]),
                        object_tags: Vec::new(),
                    })
                },
            )
        }
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let resolved = downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
                ExecutionError::InternalError(
                    "player counter replacement returned an incompatible event".into(),
                )
            })?;
            let Target::Player(player) = resolved.target else {
                return Err(ExecutionError::InternalError(
                    "player counter replacement returned an incompatible recipient".into(),
                ));
            };
            // A replacement may establish a lock as it modifies this very
            // event. Only a lock present before the proposal prevents it.
            if pre_event_game
                .turn_store
                .turn_history
                .player_counter_is_locked_this_turn(player, resolved.counter_type)
                || (resolved.counter_type == CounterType::Poison
                    && !game.can_get_poison_counters(player))
            {
                return Ok(CompletedEffectOutputs::aggregate_only(prevented()));
            }
            let before = game
                .player(player)
                .ok_or(ExecutionError::PlayerNotFound(player))?
                .counter_count(resolved.counter_type);
            let proposed_after = before.checked_add(resolved.count).ok_or_else(|| {
                ExecutionError::ResourceLimitExceeded {
                    resource: "player counter placement",
                    requested: u128::from(before) + u128::from(resolved.count),
                    maximum: u128::from(u32::MAX),
                }
            })?;
            if resolved.counter_type == CounterType::Poison {
                game.write_shared_poison(player, proposed_after);
            } else {
                game.player_mut(player)
                    .ok_or(ExecutionError::PlayerNotFound(player))?
                    .add_counters(resolved.counter_type, resolved.count);
            }
            let after = game
                .player(player)
                .ok_or(ExecutionError::PlayerNotFound(player))?
                .counter_count(resolved.counter_type);
            let actual = after.saturating_sub(before);
            if actual == 0 {
                return Ok(CompletedEffectOutputs::aggregate_only(prevented()));
            }
            let count = i64::from(actual);
            let mut notification = TriggerEvent::new_with_provenance(
                MarkersChangedEvent::added(
                    resolved.counter_type,
                    player,
                    actual,
                    resolved.cause.source,
                    resolved.cause.source_controller,
                )
                .with_count_after(after),
                event.provenance(),
            );
            if game.object(ctx.source).is_none()
                && let Some(snapshot) = &ctx.source_snapshot
            {
                notification = notification.with_source_snapshot(snapshot.clone());
            }
            Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(count).with_event(notification),
            ))
        }
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let resolved =
                downcast_event::<PutCountersEvent>(context.event.inner()).ok_or_else(|| {
                    ExecutionError::InternalError(
                        "player counter replacement lost its counter event".into(),
                    )
                })?;
            let Target::Player(player) = resolved.target else {
                return Err(ExecutionError::InternalError(
                    "player counter replacement lost its player recipient".into(),
                ));
            };
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            crate::effects::replacement::execute_replacement_original_payload_with_outputs(
                game,
                ctx,
                &effects,
                source,
                controller,
                &context,
                crate::effects::replacement::ReplacementProgramBindings {
                    targets: Some(vec![ResolvedTarget::Player(player)]),
                    object_tags: Vec::new(),
                },
                None,
                original,
            )
        }
        TraitEventResult::Prevented => Ok(CompletedEffectOutputs::aggregate_only(prevented())),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            Err(ExecutionError::InternalError(
                "player counter replacement suspended without a captured decision".into(),
            ))
        }
    }
}

enum PlayerCounterInstructionState {
    Selection,
    Selected(Event),
    Prepared {
        event: Event,
        placement: super::PreparedCounterPlacement,
    },
    Finished(EffectOutcome),
    Preparing,
}
struct PlayerCounterInstruction {
    effect: crate::effects::PlayerCountersEffect,
    iterated_player: Option<crate::ids::PlayerId>,
    state: PlayerCounterInstructionState,
}
impl std::fmt::Debug for PlayerCounterInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerCounterInstruction")
            .field("effect", &self.effect)
            .finish_non_exhaustive()
    }
}
pub(crate) fn prepare_player_counter_instruction(
    effect: crate::effects::PlayerCountersEffect,
    ctx: &ExecutionContext,
) -> Box<dyn crate::effects::SimultaneousEffectProposal> {
    Box::new(PlayerCounterInstruction {
        effect,
        iterated_player: ctx.iteration.iterated_player,
        state: PlayerCounterInstructionState::Selection,
    })
}
impl crate::effects::SimultaneousEffectProposal for PlayerCounterInstruction {
    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if ctx.decision_maker.awaiting_choice()
            || !matches!(self.state, PlayerCounterInstructionState::Selection)
        {
            return Ok(());
        }
        let event = ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            self.effect.selected_counter_event(game, ctx)
        })?;
        if let Some(event) = event {
            self.state = PlayerCounterInstructionState::Selected(event);
        }
        Ok(())
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
        let state = std::mem::replace(&mut self.state, PlayerCounterInstructionState::Preparing);
        self.state = match state {
            PlayerCounterInstructionState::Selected(event) => {
                if let Some(outcome) = player_counter_request_outcome(game, &event)? {
                    PlayerCounterInstructionState::Finished(outcome)
                } else {
                    let placement = ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
                        super::prepare_counter_placement(game, ctx, event.clone())
                    })?;
                    PlayerCounterInstructionState::Prepared { event, placement }
                }
            }
            PlayerCounterInstructionState::Prepared { event, placement }
                if placement.requires_replacement_input() =>
            {
                let placement = ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
                    super::prepare_counter_placement(game, ctx, event.clone())
                })?;
                PlayerCounterInstructionState::Prepared { event, placement }
            }
            other => other,
        };
        Ok(())
    }
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        ctx.with_temp_iterated_player(self.iterated_player, |ctx| match self.state {
            PlayerCounterInstructionState::Finished(outcome) => {
                Ok(crate::effects::SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(outcome),
                ))
            }
            PlayerCounterInstructionState::Prepared { placement, .. } => {
                super::commit_prepared_counter_original_with_outputs(game, ctx, placement)
            }
            _ => Err(ExecutionError::InternalError(
                "player counter instruction committed before selection and replacement preparation"
                    .into(),
            )),
        })
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original(self, game, ctx)
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;
    use crate::effects::EffectExecutor;

    #[test]
    fn player_counter_range_errors_restore_replacements_and_shared_poison() {
        for counter_type in [
            CounterType::Energy,
            CounterType::Experience,
            CounterType::Rad,
            CounterType::Poison,
        ] {
            for shared in [false, true] {
                for (before, amount, expected_error) in [
                    (
                        u32::MAX - 2,
                        2,
                        Some("player counter placement exceeds the supported counter range"),
                    ),
                    (
                        i32::MAX as u32 + 1,
                        i32::MAX / 2 + 1,
                        Some("player counter placement exceeds the supported counter range"),
                    ),
                    (u32::MAX - 4, 2, None),
                ] {
                    let alice = crate::ids::PlayerId::from_index(0);
                    let bob = crate::ids::PlayerId::from_index(1);
                    let mut game = GameState::new(
                        vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()],
                        20,
                    );
                    if shared {
                        game.enable_two_headed_giant(vec![
                            vec![alice, bob],
                            vec![
                                crate::ids::PlayerId::from_index(2),
                                crate::ids::PlayerId::from_index(3),
                            ],
                        ])
                        .unwrap();
                    }
                    if counter_type == CounterType::Poison {
                        game.write_shared_poison(alice, before);
                    } else {
                        game.player_mut(alice)
                            .unwrap()
                            .add_counters(counter_type, before);
                    }
                    let source = game.create_object_from_card(
                        &crate::card::CardBuilder::new(
                            crate::ids::CardId::new(),
                            "Counter replacement",
                        )
                        .build(),
                        alice,
                        crate::zone::Zone::Battlefield,
                    );
                    let replacement =
                        crate::static_abilities::StaticAbility::double_player_counters_replacement(
                            crate::target::PlayerFilter::Specific(alice),
                            Some(counter_type),
                            "Double proposed player counters".into(),
                        )
                        .generate_replacement_effect(source, alice)
                        .unwrap();
                    let one_shot = game
                        .effect_store
                        .replacement_effects
                        .add_one_shot_effect(replacement);
                    game.take_pending_trigger_events();
                    let mut ctx = ExecutionContext::new_default(source, alice);
                    ctx.set_tagged_players("retained", vec![bob]);
                    let result = crate::effects::PlayerCountersEffect::new(
                        counter_type,
                        amount,
                        crate::target::PlayerFilter::Specific(alice),
                    )
                    .execute(&mut game, &mut ctx);
                    let expected_count = if expected_error.is_some() {
                        assert_eq!(
                            result.unwrap_err(),
                            ExecutionError::ResourceLimitExceeded {
                                resource: "player counter placement",
                                requested: u128::from(before) + (amount as u128) * 2,
                                maximum: u128::from(u32::MAX),
                            }
                        );
                        assert!(
                            game.effect_store
                                .replacement_effects
                                .get_effect(one_shot)
                                .is_some()
                        );
                        before
                    } else {
                        let outcome = result.unwrap();
                        assert_eq!(outcome.count_or_zero(), 4);
                        assert_eq!(outcome.events.len(), 1);
                        let marker = outcome.events[0].downcast::<MarkersChangedEvent>().unwrap();
                        assert_eq!(marker.amount, 4);
                        assert_eq!(marker.count_after, Some(u32::MAX));
                        assert_eq!(
                            marker.location,
                            crate::marker::MarkerLocation::Player(alice)
                        );
                        assert!(
                            game.effect_store
                                .replacement_effects
                                .get_effect(one_shot)
                                .is_none()
                        );
                        u32::MAX
                    };
                    assert_eq!(
                        game.player(alice).unwrap().counter_count(counter_type),
                        expected_count
                    );
                    assert_eq!(
                        game.player(bob).unwrap().counter_count(counter_type),
                        if shared && counter_type == CounterType::Poison {
                            expected_count
                        } else {
                            0
                        }
                    );
                    assert_eq!(ctx.get_tagged_players("retained"), Some(&vec![bob]));
                    assert!(game.take_pending_trigger_events().is_empty());
                }
            }
        }
    }
}

#[cfg(test)]
mod wide_player_counter_quantity_tests {
    use super::*;
    use crate::effect::{Effect, EffectId, Value};
    use crate::effects::{
        EnergyCountersEffect, ExperienceCountersEffect, PlayerCountersEffect, PoisonCountersEffect,
        PutCountersEffect, execute_effect,
    };
    use crate::ids::{CardId, PlayerId};
    use crate::target::{ChooseSpec, PlayerFilter};
    fn object(game: &mut GameState, alice: PlayerId) -> crate::ids::ObjectId {
        let card = crate::card::CardBuilder::new(CardId::new(), "Player quantity source")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield)
    }
    fn instruction(kind: CounterType, count: Value, player: PlayerId) -> Effect {
        let filter = PlayerFilter::Specific(player);
        match kind {
            CounterType::Energy => Effect::new(EnergyCountersEffect::new(count, filter)),
            CounterType::Experience => Effect::new(ExperienceCountersEffect::new(count, filter)),
            CounterType::Poison => Effect::new(PoisonCountersEffect::new(count, filter)),
            _ => Effect::new(PlayerCountersEffect::new(kind, count, filter)),
        }
    }
    fn prior_total_reaches_player(kind: CounterType) {
        for amount in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = object(&mut game, alice);
            let following = object(&mut game, alice);
            let mut ctx = ExecutionContext::new_default(source, alice);
            let prior = Effect::with_id(
                31,
                Effect::new(PutCountersEffect::new(
                    CounterType::Charge,
                    amount,
                    ChooseSpec::SpecificObject(source),
                )),
            );
            let placed = execute_effect(&mut game, &prior, &mut ctx).unwrap();
            assert_eq!(placed.as_count(), Some(i64::from(amount)));
            let effect =
                Effect::with_id(57, instruction(kind, Value::EffectValue(EffectId(31)), bob));
            let out = execute_effect(&mut game, &effect, &mut ctx)
                .expect("a real unsigned prior receipt reaches the selected player unchanged");
            assert_eq!(out.as_count(), Some(i64::from(amount)));
            assert_eq!(game.player(bob).unwrap().counter_count(kind), amount);
            assert_eq!(game.player(alice).unwrap().counter_count(kind), 0);
            let marker = out
                .events
                .iter()
                .find_map(|event| event.downcast::<MarkersChangedEvent>())
                .unwrap();
            assert_eq!(marker.amount, amount);
            assert_eq!(marker.count_after, Some(amount));
            assert_eq!(marker.location, crate::marker::MarkerLocation::Player(bob));
            let follow = Effect::new(PutCountersEffect::new(
                CounterType::Charge,
                Value::EffectValue(EffectId(57)),
                ChooseSpec::SpecificObject(following),
            ));
            let out = execute_effect(&mut game, &follow, &mut ctx).unwrap();
            assert_eq!(out.as_count(), Some(i64::from(amount)));
            assert_eq!(game.counter_count(following, CounterType::Charge), amount);
        }
    }
    #[test]
    fn real_unsigned_prior_receipt_reaches_energy_counter_event() {
        prior_total_reaches_player(CounterType::Energy);
    }
    #[test]
    fn real_unsigned_prior_receipt_reaches_experience_counter_event() {
        prior_total_reaches_player(CounterType::Experience);
    }
    #[test]
    fn real_unsigned_prior_receipt_reaches_poison_counter_event() {
        prior_total_reaches_player(CounterType::Poison);
    }
    #[test]
    fn real_unsigned_prior_receipt_reaches_generic_player_counter_event() {
        prior_total_reaches_player(CounterType::Rad);
    }
    #[test]
    fn doubled_player_counter_event_retains_unsigned_receipt_and_following_value() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = object(&mut game, alice);
        let following = object(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let amount = i32::MAX as u32 / 2 + 1;
        let resolved = amount * 2;
        let prior = Effect::with_id(
            31,
            Effect::new(PutCountersEffect::new(
                CounterType::Charge,
                amount,
                ChooseSpec::SpecificObject(source),
            )),
        );
        execute_effect(&mut game, &prior, &mut ctx).unwrap();
        let replacement =
            crate::static_abilities::StaticAbility::double_player_counters_replacement(
                PlayerFilter::Specific(alice),
                Some(CounterType::Energy),
                "Double player counter proposal".into(),
            )
            .generate_replacement_effect(source, alice)
            .unwrap();
        let one_shot = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        let effect = Effect::with_id(
            57,
            instruction(CounterType::Energy, Value::EffectValue(EffectId(31)), alice),
        );
        let out = execute_effect(&mut game, &effect, &mut ctx)
            .expect("replacement-modified unsigned quantity fits player counter storage");
        assert_eq!(out.as_count(), Some(i64::from(resolved)));
        assert_eq!(game.player(alice).unwrap().energy_counters, resolved);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        let marker = out
            .events
            .iter()
            .find_map(|event| event.downcast::<MarkersChangedEvent>())
            .unwrap();
        assert_eq!(marker.amount, resolved);
        assert_eq!(marker.count_after, Some(resolved));
        let follow = Effect::new(PutCountersEffect::new(
            CounterType::Charge,
            Value::EffectValue(EffectId(57)),
            ChooseSpec::SpecificObject(following),
        ));
        let out = execute_effect(&mut game, &follow, &mut ctx).unwrap();
        assert_eq!(out.as_count(), Some(i64::from(resolved)));
        assert_eq!(game.counter_count(following, CounterType::Charge), resolved);
    }
}
