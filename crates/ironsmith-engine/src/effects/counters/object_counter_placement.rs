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
