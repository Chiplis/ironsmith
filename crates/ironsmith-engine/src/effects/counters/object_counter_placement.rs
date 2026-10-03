//! Apply permanent counters from the resolved event and retain replacement consequences.

use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::{
    ExecutionContext, ExecutionContextCheckpoint, ExecutionError, ResolvedTarget,
};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, PutCountersEvent, downcast_event};
use crate::game_state::{GameState, Target};

fn prevented() -> EffectOutcome {
    let mut outcome = EffectOutcome::prevented();
    outcome.value = OutcomeValue::Count(0);
    outcome
}

pub(crate) fn execute_object_counter_placement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let proposed = downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
            ExecutionError::InternalError(
                "object counter placement requires a counter event".into(),
            )
        })?;
        let Target::Object(object) = proposed.target else {
            return Err(ExecutionError::InternalError(
                "object counter placement requires an object".into(),
            ));
        };
        if proposed.count == 0 {
            return Ok(EffectOutcome::count(0));
        }
        if !game.can_have_counter_type_placed(object, proposed.counter_type) {
            return Ok(prevented());
        }
        // Entry-counter programs already participate in the enclosing entry
        // replacement event. Do not apply the same modifiers a second time.
        let processed = if ctx.replacement.entry_counter_source == Some(object) {
            TraitEventResult::Proceed(event)
        } else {
            process_trait_event_with_execution_context(game, event, ctx)?
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        commit_object_counter_placement(game, ctx, processed)
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
        context_checkpoint.restore(ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    result
}

fn commit_object_counter_placement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    processed: TraitEventResult,
) -> Result<EffectOutcome, ExecutionError> {
    match processed {
        expanded @ TraitEventResult::Expanded { .. } =>
            crate::effects::replacement::execute_event_expansion_with_targets(
                game, ctx, expanded, commit_object_counter_placement,
                |_game, context, _original_outcome| {
                    let captured = downcast_event::<PutCountersEvent>(context.event.inner())
                        .ok_or_else(|| ExecutionError::InternalError("added counter program lost its captured event".into()))?;
                    let Target::Object(recipient) = captured.target else {
                        return Err(ExecutionError::InternalError("added counter program has an incompatible recipient".into()));
                    };
                    Ok(Some(vec![ResolvedTarget::Object(recipient)]))
                },
            ),
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let resolved =
                downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
                    ExecutionError::InternalError(
                        "object counter replacement returned an incompatible event".into(),
                    )
                })?;
            let Target::Object(object) = resolved.target else {
                return Err(ExecutionError::InternalError(
                    "object counter replacement returned an incompatible recipient".into(),
                ));
            };
            if !game.can_have_counter_type_placed(object, resolved.counter_type) {
                return Ok(prevented());
            }
            let before = game.counter_count(object, resolved.counter_type);
            before.checked_add(resolved.count).ok_or_else(|| {
                ExecutionError::InternalError(
                    "object counter placement exceeds the supported counter range".into(),
                )
            })?;
            i32::try_from(resolved.count).map_err(|_| {
                ExecutionError::InternalError(
                    "object counter outcome exceeds the supported count range".into(),
                )
            })?;
            let Some(mut notification) = game.add_counters_with_source(
                object,
                resolved.counter_type,
                resolved.count,
                resolved.cause.source,
                resolved.cause.source_controller,
            ) else {
                return Ok(if resolved.count == 0 {
                    prevented()
                } else {
                    EffectOutcome::target_invalid()
                });
            };
            let actual = game
                .counter_count(object, resolved.counter_type)
                .saturating_sub(before);
            let count = i32::try_from(actual).map_err(|_| {
                ExecutionError::InternalError(
                    "object counter outcome exceeds the supported count range".into(),
                )
            })?;
            notification = notification.with_provenance(event.provenance());
            if game.object(ctx.source).is_none()
                && let Some(snapshot) = &ctx.source_snapshot
            {
                notification = notification.with_source_snapshot(snapshot.clone());
            }
            Ok(EffectOutcome::count(count)
                .with_event(notification)
                .with_affected_objects_from_game(game, vec![object]))
        }
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let resolved = downcast_event::<PutCountersEvent>(context.event.inner())
                .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "object counter replacement lost its counter event".into(),
                    )
                })?;
            let Target::Object(object) = resolved.target else {
                return Err(ExecutionError::InternalError(
                    "object counter replacement lost its object recipient".into(),
                ));
            };
            let payload = crate::effects::replacement::execute_replacement_payload(
                game,
                ctx,
                &effects,
                source,
                controller,
                &context,
                Some(vec![ResolvedTarget::Object(object)]),
            )?;
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            Ok(EffectOutcome::aggregate_replacement_outcomes(original, [payload]))
        }
        TraitEventResult::Prevented => Ok(prevented()),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            Err(ExecutionError::InternalError(
                "object counter replacement suspended without a captured decision".into(),
            ))
        }
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;
    use crate::effects::EffectExecutor;

    #[test]
    fn replacement_counter_range_errors_restore_the_instruction_and_one_shot() {
        for (before, amount, expected_error) in [
            (
                u32::MAX - 2,
                2,
                "object counter placement exceeds the supported counter range",
            ),
            (
                0,
                i32::MAX / 2 + 1,
                "object counter outcome exceeds the supported count range",
            ),
        ] {
            let alice = crate::ids::PlayerId::from_index(0);
            let bob = crate::ids::PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_card(
                &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Replacement source")
                    .build(),
                alice,
                crate::zone::Zone::Battlefield,
            );
            let recipient = game.create_object_from_card(
                &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter recipient")
                    .card_types(vec![crate::types::CardType::Artifact])
                    .build(),
                bob,
                crate::zone::Zone::Battlefield,
            );
            game.object_mut(recipient)
                .unwrap()
                .counters
                .insert(crate::object::CounterType::Charge, before);
            let replacement = crate::static_abilities::StaticAbility::double_counters_replacement(
                crate::target::ObjectFilter::specific(recipient),
                Some(crate::object::CounterType::Charge),
                "Double proposed counters".into(),
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
            let result = crate::effects::PutCountersEffect::new(
                crate::object::CounterType::Charge,
                amount,
                crate::target::ChooseSpec::SpecificObject(recipient),
            )
            .execute(&mut game, &mut ctx);
            assert_eq!(
                result.unwrap_err(),
                ExecutionError::InternalError(expected_error.into())
            );
            assert_eq!(
                game.counter_count(recipient, crate::object::CounterType::Charge),
                before
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some()
            );
            assert_eq!(ctx.get_tagged_players("retained"), Some(&vec![bob]));
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[cfg(test)]
mod zero_count_replacement_chain_tests {
    use super::*;
    fn check_removed_counter_placement(player_target: bool) {
        struct PreferHalving(crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for PreferHalving {
            fn decide_options(&mut self, _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let choice = ctx.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![choice.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter chain source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let halver = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let doubler = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let recipient = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let half = crate::static_abilities::StaticAbility::actor_counter_multiplier_replacement(
            crate::target::PlayerFilter::Any, true, "Put half as many counters, rounded down".into());
        let double = crate::static_abilities::StaticAbility::actor_counter_multiplier_replacement(
            crate::target::PlayerFilter::Any, false, "Put twice as many counters".into());
        game.effect_store.replacement_effects.add_resolution_effect(half.generate_replacement_effect(halver, alice).unwrap());
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(double.generate_replacement_effect(doubler, alice).unwrap());
        let counter = if player_target { crate::object::CounterType::Energy } else { crate::object::CounterType::Charge };
        let mut chooser = PreferHalving(halver);
        for count in [1, 2] {
            game.take_pending_trigger_events();
            let cause = crate::events::cause::EventCause::from_effect(doubler, alice);
            let event = if player_target { Event::put_player_counters(alice, counter, count, cause) }
                else { Event::put_counters(recipient, counter, count, cause) };
            let mut ctx = ExecutionContext::new(doubler, alice, &mut chooser);
            let outcome = if player_target {
                crate::effects::counters::execute_player_counter_placement(&mut game, &mut ctx, event)
            } else {
                execute_object_counter_placement(&mut game, &mut ctx, event)
            }.unwrap();
            assert_eq!(outcome.value, OutcomeValue::Count(if count == 1 { 0 } else { 2 }));
            let actual = if player_target { game.player(alice).unwrap().counter_count(counter) }
                else { game.counter_count(recipient, counter) };
            assert_eq!(actual, if count == 1 { 0 } else { 2 });
            assert!(game.take_pending_trigger_events().is_empty(), "owner returns notifications for its caller to publish");
            let markers = outcome.events.iter()
                .filter(|event| event.downcast::<crate::events::MarkersChangedEvent>().is_some()).count();
            assert_eq!(markers, usize::from(count == 2));
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count == 1,
                "halving one to zero removes the counter placement before the later one-shot applies");
        }
    }
    #[test]
    fn halving_object_counter_placement_to_zero_preserves_later_one_shot() {
        check_removed_counter_placement(false);
    }
    #[test]
    fn halving_player_counter_placement_to_zero_preserves_later_one_shot() {
        check_removed_counter_placement(true);
    }
}

#[cfg(test)]
mod removed_counter_selected_api_tests {
    use super::*;
    fn check_zero(player_target: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selected counter replacement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let replacement = crate::static_abilities::StaticAbility::actor_counter_multiplier_replacement(
            crate::target::PlayerFilter::Any, false, "Put twice as many counters".into());
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(replacement.generate_replacement_effect(source, alice).unwrap());
        for count in [0, 1] {
            let cause = crate::events::cause::EventCause::from_effect(source, alice);
            let event = if player_target { Event::put_player_counters(alice, crate::object::CounterType::Energy, count, cause) }
                else { Event::put_counters(source, crate::object::CounterType::Charge, count, cause) };
            let result = crate::events::processing::process_event_with_chosen_replacement_trait(&mut game, event, shield).unwrap();
            let event = match result {
                TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => event,
                other => panic!("selected counter replacement must return resolved carrier: {other:?}"),
            };
            let resolved = downcast_event::<PutCountersEvent>(event.inner()).unwrap();
            assert_eq!(resolved.count, count * 2);
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count == 0,
                "selected API must not consume a one-shot for an absent counter placement");
        }
    }
    #[test]
    fn selected_zero_object_counter_placement_preserves_one_shot() { check_zero(false); }
    #[test]
    fn selected_zero_player_counter_placement_preserves_one_shot() { check_zero(true); }
}

#[cfg(test)]
mod removed_counter_expansion_resume_tests {
    use super::*;
    fn check_removed_placement(captured: bool) {
        struct Ordered(crate::ids::ObjectId, crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for Ordered {
            fn decide_options(&mut self, _game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let option = [self.0, self.1].into_iter().find_map(|source|
                    ctx.options.iter().find(|option| option.legal && option.object_id == Some(source)))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![option.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter expansion source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let program_source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let half_source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let double_source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let recipient = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let program = crate::replacement::ReplacementEffect::with_matcher(program_source, alice,
            crate::events::counters::matchers::WouldPutCountersMatcher::any(),
            crate::replacement::ReplacementAction::Additionally(vec![crate::effect::Effect::gain_life(1)]));
        let program_id = game.effect_store.replacement_effects.add_one_shot_effect(program);
        let half_id = game.effect_store.replacement_effects.add_resolution_effect(
            crate::static_abilities::StaticAbility::actor_counter_multiplier_replacement(crate::target::PlayerFilter::Any,
                true, "Put half as many counters".into()).generate_replacement_effect(half_source, alice).unwrap());
        let double_id = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::static_abilities::StaticAbility::actor_counter_multiplier_replacement(crate::target::PlayerFilter::Any,
                false, "Put twice as many counters".into()).generate_replacement_effect(double_source, alice).unwrap());
        let mut chooser = Ordered(program_source, half_source);
        for count in [1, 2] {
            game.take_pending_trigger_events();
            let event = Event::put_counters(recipient, crate::object::CounterType::Charge, count,
                crate::events::cause::EventCause::from_effect(half_source, alice));
            let outcome = if captured {
                let mut result = crate::events::processing::process_trait_event(&mut game, event).unwrap();
                if count == 1 {
                    result = crate::events::processing::continue_replacement_choice_with_scope(
                        &mut game, result, program_id, None, &[], None).unwrap();
                    assert!(matches!(&result, TraitEventResult::Expanded { programs, .. } if programs.len() == 1));
                    assert_eq!(game.player(alice).unwrap().life, 20, "captured program must not execute during choice preparation");
                }
                result = crate::events::processing::continue_replacement_choice_with_scope(
                    &mut game, result, half_id, None, &[], None).unwrap();
                if count == 1 {
                    assert!(matches!(&result, TraitEventResult::Expanded { programs, .. } if programs.len() == 1),
                        "removing the placement must preserve the already captured additional action");
                }
                let mut ctx = ExecutionContext::new_default(half_source, alice);
                commit_object_counter_placement(&mut game, &mut ctx, result).unwrap()
            } else {
                let mut ctx = ExecutionContext::new(half_source, alice, &mut chooser);
                execute_object_counter_placement(&mut game, &mut ctx, event).unwrap()
            };
            assert_eq!(outcome.value, OutcomeValue::Count(if count == 1 { 0 } else { 2 }));
            assert_eq!(game.counter_count(recipient, crate::object::CounterType::Charge), if count == 1 { 0 } else { 2 });
            assert_eq!(game.player(alice).unwrap().life, 21, "the earlier additional action executes exactly once");
            let gains = outcome.events_of_type::<crate::events::LifeGainEvent>().collect::<Vec<_>>();
            assert_eq!(gains.len(), usize::from(count == 1));
            if count == 1 { assert_eq!(gains[0].amount, 1); assert_eq!(gains[0].source, Some(program_source)); }
            assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), usize::from(count == 2));
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(game.effect_store.replacement_effects.get_effect(program_id).is_none());
            assert_eq!(game.effect_store.replacement_effects.get_effect(double_id).is_some(), count == 1);
        }
    }
    #[test]
    fn zero_counter_placement_preserves_prior_additional_action_and_its_lifetime() { check_removed_placement(false); }
    #[test]
    fn captured_counter_choices_preserve_additional_action_when_placement_disappears() { check_removed_placement(true); }
}
