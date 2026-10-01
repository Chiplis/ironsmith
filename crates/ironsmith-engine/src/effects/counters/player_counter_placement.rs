//! Commit player counters from the complete replacement event and retain its outcomes.

use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::{
    ExecutionContext, ExecutionContextCheckpoint, ExecutionError, ResolvedTarget,
};
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

pub(crate) fn execute_player_counter_placement(
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
                "player counter placement requires a counter event".into(),
            )
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
            return Ok(EffectOutcome::count(0));
        }
        if game
            .turn_store
            .turn_history
            .player_counter_is_locked_this_turn(player, proposed.counter_type)
            || (proposed.counter_type == CounterType::Poison
                && !game.can_get_poison_counters(player))
        {
            return Ok(prevented());
        }
        let processed = process_trait_event_with_execution_context(game, event, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        commit_player_counter_placement(game, ctx, processed, &checkpoint)
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

fn commit_player_counter_placement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    processed: TraitEventResult,
    pre_event_game: &GameState,
) -> Result<EffectOutcome, ExecutionError> {
    match processed {
        expanded @ TraitEventResult::Expanded { .. } =>
            crate::effects::replacement::execute_event_expansion_with_targets(
                game, ctx, expanded, |game, ctx, result| commit_player_counter_placement(game, ctx, result, pre_event_game),
                |_game, context, _original_outcome| {
                    let captured = downcast_event::<PutCountersEvent>(context.event.inner())
                        .ok_or_else(|| ExecutionError::InternalError("added counter program lost its captured event".into()))?;
                    let Target::Player(recipient) = captured.target else {
                        return Err(ExecutionError::InternalError("added counter program has an incompatible recipient".into()));
                    };
                    Ok(Some(vec![ResolvedTarget::Player(recipient)]))
                },
            ),
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let resolved =
                downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
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
                return Ok(prevented());
            }
            let before = game
                .player(player)
                .ok_or(ExecutionError::PlayerNotFound(player))?
                .counter_count(resolved.counter_type);
            let proposed_after = before.checked_add(resolved.count).ok_or_else(|| {
                ExecutionError::InternalError(
                    "player counter placement exceeds the supported counter range".into(),
                )
            })?;
            i32::try_from(resolved.count).map_err(|_| {
                ExecutionError::InternalError(
                    "player counter outcome exceeds the supported count range".into(),
                )
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
                return Ok(prevented());
            }
            let count = i32::try_from(actual).map_err(|_| {
                ExecutionError::InternalError(
                    "player counter outcome exceeds the supported count range".into(),
                )
            })?;
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
            Ok(EffectOutcome::count(count).with_event(notification))
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
                        "player counter replacement lost its counter event".into(),
                    )
                })?;
            let Target::Player(player) = resolved.target else {
                return Err(ExecutionError::InternalError(
                    "player counter replacement lost its player recipient".into(),
                ));
            };
            let payload = crate::effects::replacement::execute_replacement_payload(
                game,
                ctx,
                &effects,
                source,
                controller,
                &context,
                Some(vec![ResolvedTarget::Player(player)]),
            )?;
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            Ok(EffectOutcome::aggregate_replacement_outcomes(original, [payload]))
        }
        TraitEventResult::Prevented => Ok(prevented()),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            Err(ExecutionError::InternalError(
                "player counter replacement suspended without a captured decision".into(),
            ))
        }
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
                        0,
                        i32::MAX / 2 + 1,
                        Some("player counter outcome exceeds the supported count range"),
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
                    let expected_count = if let Some(message) = expected_error {
                        assert_eq!(
                            result.unwrap_err(),
                            ExecutionError::InternalError(message.into())
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
