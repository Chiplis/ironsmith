//! "Until end of turn, you may tap lands you don't control for mana."
//! (Piracy): a player-level permission to activate the {T} mana abilities of
//! lands controlled by other players for the rest of this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub use ironsmith_core::GrantTapUncontrolledLandsForManaEffect;

impl EffectExecutor for GrantTapUncontrolledLandsForManaEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        // A surrounding "Spend this mana only to ..." wrapper is active while
        // this resolves; it binds all mana produced through the permission.
        let restrictions = ctx.mana.mana_usage_restrictions.clone();
        game.grant_tap_uncontrolled_lands_for_mana_this_turn(player, restrictions);
        Ok(EffectOutcome::resolved())
    }
}
