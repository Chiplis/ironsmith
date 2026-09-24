//! "The next time target creature adapts this turn, it adapts as though it had
//! no +1/+1 counters on it." (Biomancer's Familiar)

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Marks the target so its next adapt this turn ignores its +1/+1 counters.
#[derive(Debug, Clone, PartialEq)]
pub struct NextAdaptIgnoresCountersEffect {
    pub target: ChooseSpec,
}

impl NextAdaptIgnoresCountersEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for NextAdaptIgnoresCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };
        let turn = game.turn.turn_number;
        let mut count = 0_i32;
        for object_id in objects {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Battlefield {
                continue;
            }
            let stable = object.stable_id;
            let store = &mut game.turn_store.adapt_ignores_counters;
            store.retain(|(_, recorded_turn)| *recorded_turn == turn);
            store.push((stable, turn));
            count += 1;
        }
        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature"
    }
}
