//! "Choose new targets" effect implementation.
//!
//! This effect supports text like "You may choose new targets for the copy."
//! by re-targeting stack objects produced by a prior effect.

use crate::decisions::context::{BooleanContext, TargetRequirementContext, TargetsContext};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::targeting::normalize_targets_for_requirements;
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
                super::retarget_stack_object::stack_entry_retarget_requirements(game, &entry, true)?
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
                slots.iter().map(|slot| slot.requirement.clone()).collect();

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
                .filter(|targets| {
                    super::retarget_stack_object::retarget_proposal_respects_other_targets(
                        &slots, targets,
                    )
                })
            else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::target_invalid());
            };

            match super::retarget_stack_object::commit_retarget(
                game, entry.target_id(), new_targets, ctx.provenance,
            ) {
                Some((did_change, completion)) => {
                    changed += i32::from(did_change);
                    events.extend(completion);
                }
                None if self.may => continue,
                None => return Ok(EffectOutcome::target_invalid()),
            }
        }

        Ok(EffectOutcome::count(changed).with_events(events))
    }
}
