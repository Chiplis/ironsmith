//! Duration-bound token-creation multiplier created by a resolving
//! spell/ability (Kaya, Geist Hunter).
use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityKind;

pub type RegisterTokenCreationMultiplierEffect =
    ironsmith_core::RegisterTokenCreationMultiplierEffect;

impl EffectExecutor for RegisterTokenCreationMultiplierEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The resolved effect is created by the resolving ability and
        // controlled by its controller; it lasts for its duration whether or
        // not its source remains on the battlefield (CR 611.2a, 614.1a). It
        // shares the matcher and modification of the static multiplier
        // ("twice that many of those tokens are created instead", CR 111.1).
        let Some(replacement) = crate::static_abilities::MultiplyTokenCreationReplacement::new(
            self.controller.clone(),
            None,
            self.factor,
            "token creation multiplier",
        )
        .generate_replacement_effect(ctx.source, ctx.controller) else {
            return Ok(EffectOutcome::resolved());
        };
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute_child(game, ctx)
    }
}
