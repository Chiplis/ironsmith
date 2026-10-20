//! Reveal a source-linked secret player choice as a payment action.
use crate::effect::EffectOutcome;
use crate::effects::{
    CostExecutableEffect, CostValidationError, EffectExecutor, ExecutionContext, ExecutionError,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
pub use ironsmith_core::RevealChosenPlayerEffect;

impl EffectExecutor for RevealChosenPlayerEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = game
            .reveal_secret_chosen_player(ctx.source, ctx.controller)
            .ok_or_else(|| {
                ExecutionError::Impossible(
                    "no player secretly chosen by this player for this source".into(),
                )
            })?;
        ctx.combat.chosen_player = Some(player);
        Ok(EffectOutcome::count(1))
    }
}
impl CostExecutableEffect for RevealChosenPlayerEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        game.secret_chosen_player(source, controller)
            .map(|_| ())
            .ok_or_else(|| {
                CostValidationError::Other(
                    "no player secretly chosen by this player for this source".into(),
                )
            })
    }
}
