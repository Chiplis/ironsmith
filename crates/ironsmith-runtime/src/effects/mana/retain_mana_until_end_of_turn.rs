use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::EffectExecutor;
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
        game.add_restriction_effect(
            Restriction::lose_unspent_mana(self.player.clone(), None),
            Until::EndOfTurn,
            ctx.source,
            ctx.controller,
            ctx.iteration.iterated_player,
        );
        Ok(EffectOutcome::resolved())
    }
}
