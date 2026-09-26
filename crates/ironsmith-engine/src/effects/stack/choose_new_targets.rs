//! "Choose new targets" effect implementation.
//!
//! This effect supports text like "You may choose new targets for the copy."
//! by re-targeting stack objects produced by a prior effect.

use crate::decisions::context::{BooleanContext, TargetRequirementContext, TargetsContext};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::spells::BecomesTargetedEvent;
use crate::game_state::{GameState, Target};
use crate::targeting::normalize_targets_for_requirements;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::ChooseNewTargetsEffect;

/// Effect that lets a player choose new targets for stack object(s).
///
/// The objects are read from a prior effect outcome, preferring explicit
/// object outputs and falling back to preserved chosen/affected-object facts.
impl EffectExecutor for ChooseNewTargetsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = match ctx.get_outcome(self.from_effect) {
            Some(outcome) => outcome.output_objects().to_vec(),
            None => return Ok(EffectOutcome::resolved()),
        };
        if object_ids.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut changed = 0;
        let mut events = Vec::new();

        for object_id in object_ids {
            let Some(stack_idx) = game.stack.iter().position(|e| e.object_id == object_id) else {
                continue;
            };

            if game
                .object(object_id)
                .is_none_or(|obj| obj.zone != Zone::Stack)
            {
                continue;
            }

            let entry = game.stack[stack_idx].clone();
            // One requirement per announced target slot; each may keep its
            // current targets (CR 707.10c, 115.7d).
            let Some(slots) =
                super::retarget_stack_object::stack_entry_retarget_requirements(game, &entry, true)
            else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::target_invalid());
            };

            if slots.is_empty() {
                continue;
            }
            let requirements: Vec<TargetRequirementContext> =
                slots.into_iter().map(|slot| slot.requirement).collect();

            let chooser = if let Some(filter) = &self.chooser {
                resolve_player_filter(game, filter, ctx)?
            } else {
                entry.controller
            };

            if self.may {
                let source_name = game
                    .object(object_id)
                    .map(|o| o.name.to_string())
                    .unwrap_or_else(|| "copy".to_string());
                let choose = ctx.decision_maker.decide_boolean(
                    game,
                    &BooleanContext::new(
                        chooser,
                        Some(object_id),
                        format!("Choose new targets for {source_name}?"),
                    ),
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                if !choose {
                    continue;
                }
            }

            let targets_ctx =
                TargetsContext::new(chooser, object_id, "copy".to_string(), requirements.clone());
            let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(new_targets) = normalize_targets_for_requirements(&requirements, proposed)
            else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::target_invalid());
            };

            if game.stack[stack_idx].targets != new_targets {
                let old_targets = game.stack[stack_idx].targets.clone();
                let mut updated_entry = game.stack[stack_idx].clone();
                updated_entry.targets = new_targets;
                if !updated_entry.remap_target_distributions(&old_targets) {
                    if self.may {
                        continue;
                    }
                    return Ok(EffectOutcome::target_invalid());
                }
                game.stack[stack_idx] = updated_entry;
                changed += 1;
                // Only targets that are new become the target (CR 115.7);
                // unchanged ones were targeted when the object was created.
                // Each distinct new target becomes a target once (CR 115.3).
                let mut newly_targeted: Vec<Target> = Vec::new();
                for target in &game.stack[stack_idx].targets {
                    if old_targets.contains(target) || newly_targeted.contains(target) {
                        continue;
                    }
                    newly_targeted.push(*target);
                    events.push(TriggerEvent::new_with_provenance(
                        BecomesTargetedEvent::new_target(
                            *target,
                            object_id,
                            entry.controller,
                            entry.is_ability,
                        ),
                        ctx.provenance,
                    ));
                }
            }
        }

        Ok(EffectOutcome::count(changed).with_events(events))
    }
}
