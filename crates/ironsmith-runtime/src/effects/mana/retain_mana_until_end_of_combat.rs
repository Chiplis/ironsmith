use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type RetainManaUntilEndOfCombatEffect = ironsmith_core::RetainManaUntilEndOfCombatEffect;

impl EffectExecutor for RetainManaUntilEndOfCombatEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let Some((last_player, mana)) = ctx.mana.last_added_mana.clone() else {
            return Ok(EffectOutcome::resolved());
        };
        if last_player != player_id {
            return Ok(EffectOutcome::resolved());
        }

        if let Some(player) = game.player_mut(player_id) {
            for symbol in mana {
                player.mana_pool.retain_existing_until_end_of_combat(symbol, 1);
            }
        }

        Ok(EffectOutcome::resolved())
    }
}
