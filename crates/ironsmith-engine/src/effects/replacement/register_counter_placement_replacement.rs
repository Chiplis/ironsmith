use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type RegisterCounterPlacementReplacementEffect =
    ironsmith_core::RegisterCounterPlacementReplacementEffect;

impl EffectExecutor for RegisterCounterPlacementReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The resolved effect is created by the resolving ability and
        // controlled by its controller; it lasts for its duration whether or
        // not its source remains on the battlefield (CR 611.2a, 614.1a).
        let replacement = crate::static_abilities::counter_placement_addition_replacement(
            ctx.source,
            ctx.controller,
            self.filter.clone(),
            self.counter_type,
            i64::from(self.additional),
        );
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute_child(game, ctx)
    }
}
