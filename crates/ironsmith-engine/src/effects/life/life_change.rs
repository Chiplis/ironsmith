//! Commit life changes from their resolved event, never the authored amount/player.

use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, LifeGainEvent, LifeLossEvent, downcast_event};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;

fn prevented_life_change() -> EffectOutcome {
    let mut outcome = EffectOutcome::prevented();
    outcome.value = OutcomeValue::Count(0);
    outcome
}

/// Keep replacement choices and payload execution in the same transaction.
/// The decision maker retains its prompt/answers, but no game mutation or
/// emitted result escapes a suspended or failed operation.
pub(crate) fn execute_life_change(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    execute_life_changes(game, ctx, vec![event])
}

/// Resolve every life proposal against the pre-commit state. In particular,
/// the first side of an exchange must not affect the second side's matching.
pub(crate) fn execute_life_changes(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut events: Vec<Event>,
) -> Result<EffectOutcome, ExecutionError> {
    let checkpoint = game.clone();
    let order = game.team_apnap_player_order();
    events.sort_by_key(|event| {
        let player = event.inner().affected_player(game);
        order
            .iter()
            .position(|candidate| *candidate == player)
            .unwrap_or(usize::MAX)
    });
    let result = (|| {
        let mut prepared = Vec::new();
        for event in events {
            prepared.push(prepare_life_change(game, ctx, event)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        let mut outcomes = Vec::new();
        for proposal in prepared {
            outcomes.push(commit_life_change(game, ctx, proposal)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    })();
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || result.is_err() {
        *game = checkpoint;
    }
    result
}

fn prepare_life_change(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<TraitEventResult, ExecutionError> {
    let (player, amount, allowed) =
        if let Some(gain) = downcast_event::<LifeGainEvent>(event.inner()) {
            (gain.player, gain.amount, game.can_gain_life(gain.player))
        } else if let Some(loss) = downcast_event::<LifeLossEvent>(event.inner()) {
            (loss.player, loss.amount, game.can_lose_life(loss.player))
        } else {
            return Err(ExecutionError::InternalError(
                "life change requires a life event".into(),
            ));
        };
    if game.player(player).is_none() {
        return Err(ExecutionError::PlayerNotFound(player));
    }
    if amount == 0 {
        return Ok(TraitEventResult::Proceed(event));
    }
    if !allowed {
        return Ok(TraitEventResult::Prevented);
    }
    process_trait_event_with_execution_context(game, event, ctx)
}

fn commit_life_change(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    result: TraitEventResult,
) -> Result<EffectOutcome, ExecutionError> {
    match result {
        expanded @ TraitEventResult::Expanded { .. } =>
            crate::effects::replacement::execute_event_expansion(
                game, ctx, expanded, commit_life_change,
            ),
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let provenance = event.provenance();
            let (actual, mut notification) = if let Some(gain) =
                downcast_event::<LifeGainEvent>(event.inner())
            {
                if gain.amount == 0 {
                    return Ok(EffectOutcome::count(0));
                }
                let actual = game.gain_life(gain.player, gain.amount);
                (
                    actual,
                    TriggerEvent::new_with_provenance(gain.with_amount(actual), provenance),
                )
            } else if let Some(loss) = downcast_event::<LifeLossEvent>(event.inner()) {
                if loss.amount == 0 {
                    return Ok(EffectOutcome::count(0));
                }
                let amount =
                    if loss.from_damage && game.damage_cant_reduce_life_below_one(loss.player) {
                        let life = game
                            .player(loss.player)
                            .ok_or(ExecutionError::PlayerNotFound(loss.player))?
                            .life;
                        loss.amount
                            .min(u32::try_from(life.saturating_sub(1).max(0)).unwrap_or(0))
                    } else {
                        loss.amount
                    };
                let actual = game.lose_life(loss.player, amount);
                (
                    actual,
                    TriggerEvent::new_with_provenance(loss.with_amount(actual), provenance),
                )
            } else {
                return Err(ExecutionError::InternalError(
                    "life replacement returned an incompatible event".into(),
                ));
            };
            if actual == 0 {
                return Ok(prevented_life_change());
            }
            // A proposal can produce several physical life changes. Each
            // committed observation has its own identity under that proposal.
            let observation = game.alloc_child_event_provenance(provenance, notification.kind());
            notification = notification.with_provenance(observation);
            if let Some(batch) = game.simultaneous_action_batch() {
                notification = notification.with_simultaneous_batch(batch);
            }
            if game.object(ctx.source).is_none()
                && let Some(snapshot) = &ctx.source_snapshot
            {
                notification = notification.with_source_snapshot(snapshot.clone());
            }
            Ok(EffectOutcome::count(actual as i32).with_event(notification))
        }
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let payload = crate::effects::replacement::execute_replacement_payload(
                game, ctx, &effects, source, controller, &context, None,
            )?;
            // The payload's notifications happened, but none of the original
            // life change happened "this way".
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            Ok(EffectOutcome::aggregate_replacement_outcomes(original, [payload]))
        }
        TraitEventResult::Prevented => Ok(prevented_life_change()),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            if ctx.decision_maker.awaiting_choice() {
                Ok(EffectOutcome::count(0))
            } else {
                Err(ExecutionError::InternalError(
                    "life replacement suspended without a captured decision".into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod removed_life_operation_tests {
    use super::*;
    fn check_removed_life_change(gain: bool) {
        struct PreferSubtractor(crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for PreferSubtractor {
            fn decide_options(&mut self, _game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let choice = ctx.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![choice.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Life replacement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let subtractor = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let adder = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let effect = |source, modification| {
            let action = crate::replacement::ReplacementAction::Modify(modification);
            if gain {
                crate::replacement::ReplacementEffect::with_matcher(source, alice,
                    crate::events::life::matchers::WouldGainLifeMatcher::any_player(), action)
            } else {
                crate::replacement::ReplacementEffect::with_matcher(source, alice,
                    crate::events::life::matchers::WouldLoseLifeMatcher::any_player(), action)
            }
        };
        game.effect_store.replacement_effects.add_resolution_effect(effect(subtractor, crate::replacement::EventModification::Subtract(1)));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(effect(adder, crate::replacement::EventModification::Add(2)));
        let mut chooser = PreferSubtractor(subtractor);
        for count in [0, 1, 2] {
            game.take_pending_trigger_events();
            let before = game.player(alice).unwrap().life;
            let event = if gain { Event::new_with_provenance(LifeGainEvent::new(alice, count).with_source(subtractor), Default::default()) }
                else { Event::life_loss(alice, count, false) };
            let mut ctx = ExecutionContext::new(subtractor, alice, &mut chooser);
            let outcome = execute_life_change(&mut game, &mut ctx, event).unwrap();
            let amount = if count == 2 { 3 } else { 0 };
            assert_eq!(outcome.value, OutcomeValue::Count(amount));
            assert_eq!(i64::from(game.player(alice).unwrap().life), i64::from(before) + if gain { amount } else { -amount });
            assert_eq!(outcome.events.len(), usize::from(count == 2));
            assert!(game.take_pending_trigger_events().is_empty(), "owner returns notifications for its caller to publish");
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count != 2,
                "absent life operation must preserve later one-shot until a positive change");
        }
    }
    #[test]
    fn life_gain_reduced_to_zero_does_not_revive_or_consume_later_one_shot() { check_removed_life_change(true); }
    #[test]
    fn life_loss_reduced_to_zero_does_not_revive_or_consume_later_one_shot() { check_removed_life_change(false); }
}
