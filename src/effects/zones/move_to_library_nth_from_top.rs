//! Move an object to the Nth position from the top of its owner's library.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_value};
use crate::event_processor::EventOutcome;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::{apply_zone_change_with_additional_effects, maybe_prompt_for_split_result_order};

/// "Put target [object] into its owner's library Nth from the top."
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToLibraryNthFromTopEffect {
    pub target: ChooseSpec,
    pub position: Value,
}

impl MoveToLibraryNthFromTopEffect {
    pub fn new(target: ChooseSpec, position: Value) -> Self {
        Self { target, position }
    }
}

impl EffectExecutor for MoveToLibraryNthFromTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let raw_position = resolve_value(game, &self.position, ctx)?;
        let position = raw_position.max(1) as usize;

        let mut moved_ids = Vec::new();
        let mut any_replaced = false;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            let result = apply_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Library,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(EffectOutcome::prevented());
                }
                EventOutcome::Proceed(mut result) => {
                    if !result.new_object_ids.is_empty() {
                        if result.final_zone == Zone::Exile {
                            for &new_id in &result.new_object_ids {
                                game.add_exiled_with_source_link(ctx.source, new_id);
                            }
                        } else if result.final_zone == Zone::Library {
                            for &new_id in &result.new_object_ids {
                                if let Some(owner) = game.object(new_id).map(|o| o.owner)
                                    && let Some(player) = game.player_mut(owner)
                                    && let Some(current_idx) =
                                        player.library.iter().position(|id| *id == new_id)
                                {
                                    player.library.remove(current_idx);
                                    let insert_idx =
                                        player.library.len().saturating_sub(position - 1);
                                    player.library.insert(insert_idx, new_id);
                                }
                            }
                            if from_zone == Zone::Battlefield {
                                maybe_prompt_for_split_result_order(
                                    game,
                                    &mut ctx.decision_maker,
                                    result.final_zone,
                                    &ctx.cause,
                                    &mut result,
                                );
                                game.record_zone_change_results(
                                    object_id,
                                    result.new_object_ids.clone(),
                                );
                            }
                        }
                        moved_ids.extend(result.new_object_ids.iter().copied());
                    }
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                }
                EventOutcome::NotApplicable => {}
            }
        }

        if !moved_ids.is_empty() {
            return Ok(EffectOutcome::with_objects(moved_ids));
        }
        if any_replaced {
            return Ok(EffectOutcome::replaced());
        }
        Ok(EffectOutcome::target_invalid())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to move into library at a fixed top position"
    }
}
