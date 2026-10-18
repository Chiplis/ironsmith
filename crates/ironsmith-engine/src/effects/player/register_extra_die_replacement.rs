//! One-shot numeric roll-batch replacements with an independent ignore chooser.
pub type RegisterExtraDieReplacementEffect = ironsmith_core::RegisterExtraDieReplacementEffect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::effect::EffectOutcome;
use crate::game_state::GameState;
impl EffectExecutor for RegisterExtraDieReplacementEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let player = crate::effects::helpers::resolve_player_from_spec(game, &self.player, ctx)?;
        let chooser = crate::effects::helpers::resolve_player_filter(game, &self.chooser, ctx)?;
        game.register_extra_die_replacement(ctx.source, player, chooser, self.additional);
        Ok(EffectOutcome::count(1))
    }
}
