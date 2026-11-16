//! Resolved "that ability triggers an additional time" rules keyed to an
//! object (Lwaxana Troi, Charming Guest).

use crate::effect::{EffectOutcome, Until};
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub use ironsmith_core::TriggerAdditionalTimeEffect;

impl EffectExecutor for TriggerAdditionalTimeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) if !self.target.is_target() => {
                return Ok(EffectOutcome::count(0));
            }
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };
        let mut count = 0_i32;
        for object_id in objects {
            if game
                .object(object_id)
                .is_none_or(|object| object.zone != Zone::Battlefield)
            {
                continue;
            }
            // CR 611.2b: a predicate duration names its objects as the
            // effect begins.
            let duration = match &self.duration {
                Until::ForAsLongAs(predicate) => {
                    match crate::effects::continuous::materialize_duration_predicate(
                        predicate,
                        &crate::continuous::EffectTarget::Specific(object_id),
                        &None,
                        game,
                        ctx,
                    ) {
                        Some(predicate) => Until::ForAsLongAs(predicate),
                        None => continue,
                    }
                }
                other => other.clone(),
            };
            // "For as long as this creature remains tapped": a source that
            // is already untapped starts no effect (CR 611.2b).
            if duration == Until::SourceUntaps && !game.is_tapped(ctx.source) {
                continue;
            }
            game.add_trigger_multiplier_effect(
                object_id,
                ctx.controller,
                self.copies as usize,
                duration,
                ctx.source,
            );
            count += 1;
        }
        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
