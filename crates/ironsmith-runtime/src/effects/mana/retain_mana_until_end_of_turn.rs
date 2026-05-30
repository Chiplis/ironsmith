use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type RetainManaUntilEndOfTurnEffect = ironsmith_core::RetainManaUntilEndOfTurnEffect;

impl EffectExecutor for RetainManaUntilEndOfTurnEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if let Some((mana_player, mana)) = ctx.mana.last_mana_added.clone()
            && mana_player == player
            && !mana.is_empty()
        {
            game.add_mana_retention(player, mana, self.duration.clone(), self.include_phase_ends);
        }
        Ok(EffectOutcome::resolved())
    }
}
