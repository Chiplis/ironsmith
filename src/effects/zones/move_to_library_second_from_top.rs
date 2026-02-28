//! Move an object to second from top of its owner's library.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// "Put target [object] into its owner's library second from the top."
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToLibrarySecondFromTopEffect {
    pub target: ChooseSpec,
}

impl MoveToLibrarySecondFromTopEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for MoveToLibrarySecondFromTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = resolve_objects_from_spec(game, &self.target, ctx)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let mut moved_ids = Vec::new();
        let mut any_replaced = false;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;

            let result = process_zone_change(
                game,
                object_id,
                from_zone,
                Zone::Library,
                &mut ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(EffectOutcome::from_result(EffectResult::Prevented));
                }
                EventOutcome::Proceed(final_zone) => {
                    if let Some(new_id) = game.move_object(object_id, final_zone) {
                        if final_zone == Zone::Exile {
                            game.add_exiled_with_source_link(ctx.source, new_id);
                        } else if final_zone == Zone::Library
                            && let Some(owner) = game.object(new_id).map(|o| o.owner)
                            && let Some(player) = game.player_mut(owner)
                            && let Some(pos) = player.library.iter().position(|id| *id == new_id)
                        {
                            player.library.remove(pos);
                            let insert_pos = player.library.len().saturating_sub(1);
                            player.library.insert(insert_pos, new_id);
                        }
                        moved_ids.push(new_id);
                    }
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                }
                EventOutcome::NotApplicable => {}
            }
        }

        if !moved_ids.is_empty() {
            return Ok(EffectOutcome::from_result(EffectResult::Objects(moved_ids)));
        }
        if any_replaced {
            return Ok(EffectOutcome::from_result(EffectResult::Replaced));
        }
        Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to move second from top"
    }
}
