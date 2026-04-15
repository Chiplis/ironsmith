//! Goad effect implementation.

use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that goads creature(s).
#[derive(Debug, Clone, PartialEq)]
pub struct GoadEffect {
    /// Creature target specification.
    pub target: ChooseSpec,
}

impl GoadEffect {
    /// Create a new goad effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for GoadEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_for_effect(game, ctx, &self.target)?;
        let mut count = 0_i32;
        for object_id in objects {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Battlefield || !game.current_is_creature(object_id) {
                continue;
            }
            game.add_goad_effect(object_id, ctx.controller, Until::YourNextTurn, ctx.source);
            count += 1;
        }
        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to goad"
    }
}
