//! "Roll the planar die." as an instruction (Fractured Powerstone): an
//! effect-driven roll (CR 901.9). Unlike the special action it does not raise
//! the cost of the next voluntary roll. Outside a Planechase game there is no
//! planar die, so the instruction does nothing.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type RollPlanarDieEffect = ironsmith_core::RollPlanarDieEffect;

impl EffectExecutor for RollPlanarDieEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        match game.roll_planar_die_with_decision_maker(player, false, &mut ctx.decision_maker) {
            Ok(_) => Ok(EffectOutcome::resolved()),
            // No Planechase game: there is no planar die to roll.
            Err(ExecutionError::Impossible(_)) => Ok(EffectOutcome::count(0)),
            Err(error) => Err(error),
        }
    }
}
