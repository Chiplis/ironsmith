//! Keyword action event emission effect.
//!
//! Some rules text triggers on a keyword action (e.g., "when you cycle this card").
//! This effect provides a generic way to emit a KeywordActionEvent as part of an
//! effect/cost pipeline so triggers can observe it.

use std::collections::HashMap;

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
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
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            if matches!(
                self.action,
                KeywordActionKind::Forage
                    | KeywordActionKind::AssembleContraption
                    | KeywordActionKind::Planeswalk
                    | KeywordActionKind::SetSchemeInMotion
                    | KeywordActionKind::AbandonScheme
            ) {
                return super::keyword_programs::execute_keyword_program(
                    game,
                    ctx,
                    self.action,
                    self.amount,
                );
            }
            if self.action == KeywordActionKind::Harness
                && !super::keyword_programs::commit_harness(game, ctx.source)
            {
                return Ok(EffectOutcome::count(0));
            }
            KeywordActionCompletion(self.clone()).execute_child(game, ctx)
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending {
            return Ok(EffectOutcome::count(0));
        }
        result
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
            return Ok(EffectOutcome::resolved().with_event(event));
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
        Ok(EffectOutcome::resolved().with_event(event))
    }
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
