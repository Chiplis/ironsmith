//! Keyword action event emission effect.
//!
//! Some rules text triggers on a keyword action (e.g., "when you cycle this card").
//! This effect provides a generic way to emit a KeywordActionEvent as part of an
//! effect/cost pipeline so triggers can observe it.

use std::collections::HashMap;

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::EmitKeywordActionEffect;

use super::keyword_programs::forage_payments;

fn snapshot_from_memory(_game: &GameState, snapshot: &ObjectSnapshot) -> ObjectSnapshot {
    snapshot.clone()
}

fn object_tags_from_config(
    effect: &EmitKeywordActionEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<HashMap<TagKey, Vec<ObjectSnapshot>>, ExecutionError> {
    let mut tags: HashMap<TagKey, Vec<ObjectSnapshot>> = HashMap::new();
    for config in &effect.object_tags {
        // An instruction that never ran named no objects.
        let Some(outcome) = ctx.get_outcome(config.effect_id) else {
            continue;
        };
        let memories = if config.use_affected_memory {
            outcome.affected_object_memory()
        } else {
            outcome.chosen_object_memory()
        };
        let Some(memories) = memories else {
            continue;
        };
        let snapshots = memories
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            tags.entry(config.tag.clone())
                .or_default()
                .extend(snapshots);
        }
    }
    Ok(tags)
}

impl EffectExecutor for EmitKeywordActionEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
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
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                if matches!(
                    self.action,
                    KeywordActionKind::Forage
                        | KeywordActionKind::AssembleContraption
                        | KeywordActionKind::Planeswalk
                        | KeywordActionKind::SetSchemeInMotion
                        | KeywordActionKind::AbandonScheme
                ) {
                    return super::keyword_programs::execute_keyword_program_with_outputs(
                        game,
                        ctx,
                        self.action,
                        self.amount,
                    );
                }
                if self.action == KeywordActionKind::Harness
                    && !super::keyword_programs::commit_harness(game, ctx.source)
                {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                KeywordActionCompletion(self.clone()).execute_child_with_outputs(game, ctx)
            },
        )
    }

    fn cost_description(&self) -> Option<String> {
        if self.action == KeywordActionKind::Forage {
            return Some("Forage".into());
        }
        // Internal scaffolding effect used to emit trigger-visible events from costs.
        // This should not show up as part of the printed/visible cost.
        Some(String::new())
    }
}

/// Pure completion publication, separated from executable keyword programs.
/// The compatibility façade retains the existing compiler vocabulary.
#[derive(Debug, Clone)]
struct KeywordActionCompletion(EmitKeywordActionEffect);
impl EffectExecutor for KeywordActionCompletion {
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
        let config = &self.0;
        let object_tags = object_tags_from_config(config, game, ctx)?;
        if config.action == KeywordActionKind::Exploit {
            // CR 702.110b: "when this exploits a creature" looks back in time.
            // A creature that exploited itself is gone by now, so carry its
            // last-known information for the source filter and its own trigger.
            let on_battlefield = game
                .object(ctx.source)
                .filter(|object| object.zone == crate::zone::Zone::Battlefield);
            let source_snapshot = on_battlefield
                .map(|object| game.cached_object_snapshot_with_calculated_characteristics(object))
                .or_else(|| ctx.source_snapshot.clone());
            // Only a source sacrificed by this exploit ability gets lookback.
            // An ETB-time snapshot must not revive an ability whose source left
            // before this sacrifice (CR 603.10a, 702.110b).
            let exploited_itself = object_tags
                .get(crate::tag::EXPLOITED_TAG)
                .is_some_and(|objects| objects.iter().any(|object| object.object_id == ctx.source));
            let lookback = if on_battlefield.is_none() && exploited_itself {
                source_snapshot.iter().cloned().collect()
            } else {
                Vec::new()
            };
            let event = TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(config.action, ctx.controller, ctx.source, config.amount)
                    .with_object_tags(object_tags)
                    .with_snapshot(source_snapshot),
                ctx.provenance,
            )
            .with_lookback_source_snapshots(lookback);
            return PublishKeywordActionCompletion(event).execute_child_with_outputs(game, ctx);
        }
        // CR 702.29: a cycling ability's announced X (paid as part of the
        // cycling cost) is the X of its "when you cycle this card" trigger.
        let x_value = (config.action == KeywordActionKind::Cycle)
            .then_some(ctx.x_value)
            .flatten();
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(config.action, ctx.controller, ctx.source, config.amount)
                .with_object_tags(object_tags)
                .with_x_value(x_value),
            ctx.provenance,
        );
        PublishKeywordActionCompletion(event).execute_child_with_outputs(game, ctx)
    }
}

/// Publish an already authored completion without executing the action again or
/// reconstructing its subjects from the instruction source. The caller freezes
/// action-specific observations at the semantic completion boundary.
#[derive(Debug, Clone)]
struct PublishKeywordActionCompletion(TriggerEvent);

impl EffectExecutor for PublishKeywordActionCompletion {
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
        observe_keyword_action_completion_with_outputs(game, self.0.clone())
    }
}

/// Shared completion observation owner for recorded effects and game-loop
/// actions. Root actions retain their own queue/transaction boundary; neither
/// adapter reconstructs occurrence identity, lifecycle state or grouping.
pub(crate) fn observe_keyword_action_completion(
    game: &mut GameState,
    event: TriggerEvent,
) -> Result<EffectOutcome, ExecutionError> {
    observe_keyword_action_completion_with_outputs(game, event).map(|outputs| outputs.outcome)
}

pub(crate) fn observe_keyword_action_completion_with_outputs(
    game: &mut GameState,
    event: TriggerEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if event.downcast::<KeywordActionEvent>().is_none() {
        return Err(ExecutionError::InternalError(
            "keyword completion requires a keyword action observation".into(),
        ));
    }
    let parent = event.provenance();
    let event = crate::effects::observe_action_completion(game, event, Some(parent))?;
    Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
        EffectOutcome::resolved().with_event(event),
    ))
}

pub(crate) fn complete_keyword_action(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: KeywordActionEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion(
        game,
        ctx,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    )
}

/// Publish an authored observation with its original parent, snapshots and
/// simultaneous group. Cost and resolution adapters need those identities even
/// when their publication context no longer names the original instruction.
pub(crate) fn publish_keyword_action_completion(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: TriggerEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion_receipt(game, ctx, event)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// The recorded completion leaf retains its packet through scalar adapters.
pub(crate) fn publish_keyword_action_completion_receipt(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: TriggerEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if event.downcast::<KeywordActionEvent>().is_none() {
        return Err(ExecutionError::InternalError(
            "keyword completion requires a keyword action observation".into(),
        ));
    }
    PublishKeywordActionCompletion(event).execute_child_with_outputs(game, ctx)
}

/// Keep the action body's result independent of its completion notification.
/// Both remain recorded children; publication must not add to a count or replace
/// a chosen/affected-object result supplied by the action's owner.
pub(crate) fn complete_keyword_action_with_result(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: EffectOutcome,
    event: KeywordActionEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion_with_result(
        game,
        ctx,
        original,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    )
}

/// Retain the body owner's outputs while publishing its one completion.
pub(crate) fn complete_keyword_action_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effects::CompletedEffectOutputs,
    event: KeywordActionEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    publish_keyword_action_completion_with_outputs(
        game,
        ctx,
        original,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    )
}

/// Preserve an action's primary result when its completion carries captured
/// provenance, snapshots or a group whose original scope has already closed.
pub(crate) fn publish_keyword_action_completion_with_result(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: EffectOutcome,
    event: TriggerEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion_with_outputs(
        game,
        ctx,
        crate::effects::CompletedEffectOutputs::aggregate_only(original),
        event,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Aggregate and retained callers share publication and primary projection.
pub(crate) fn publish_keyword_action_completion_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effects::CompletedEffectOutputs,
    event: TriggerEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let completion = publish_keyword_action_completion_receipt(game, ctx, event)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    Ok(original.append_batch_completion_outputs(completion))
}

impl CostExecutableEffect for EmitKeywordActionEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            crate::costs::PaymentReason::Other,
        )
    }
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        if self.action == KeywordActionKind::Forage
            && !forage_payments(reason == crate::costs::PaymentReason::CastSpell)
                .iter()
                .any(|effect| {
                    effect
                        .0
                        .can_execute_as_cost(game, source, controller)
                        .is_ok()
                })
        {
            return Err(crate::effects::CostValidationError::NotEnoughCards);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ColorSet;
    use crate::effect::EffectId;
    use crate::ids::{ObjectId, PlayerId, StableId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn forwards_affected_object_memory_as_event_object_tag() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);
        let sacrificed = ObjectId::from_raw(20);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect_id = EffectId(7);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::resolved().with_affected_object_memory(vec![{
                let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                    sacrificed,
                    StableId::from(sacrificed),
                    bob,
                    bob,
                    Zone::Battlefield,
                );
                snapshot.name = "Sacrificed Creature".to_string();
                snapshot.power = Some(3);
                snapshot.toughness = Some(5);
                snapshot.linked_face_mana_value = Some((2) as u32);
                snapshot.card_types = vec![CardType::Creature];
                snapshot.colors = ColorSet::default();
                snapshot.subtypes = Vec::new();
                snapshot.is_token = true;
                snapshot
            }]),
        );

        let effect = EmitKeywordActionEffect::new(crate::events::KeywordActionKind::Exploit, 1)
            .with_affected_object_memory_tag(effect_id, crate::tag::EXPLOITED_TAG);
        let outcome = effect.execute(&mut game, &mut ctx).expect("event emitted");
        let event = outcome.events.first().expect("keyword action event");
        let keyword = event
            .downcast::<KeywordActionEvent>()
            .expect("keyword action payload");
        let snapshots = keyword
            .object_tags
            .get(crate::tag::EXPLOITED_TAG)
            .expect("exploited tag");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].object_id, sacrificed);
        assert_eq!(snapshots[0].controller, bob);
        assert_eq!(snapshots[0].zone, Zone::Battlefield);
        assert_eq!(snapshots[0].toughness, Some(5));
    }
}
