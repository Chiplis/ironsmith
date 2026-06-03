//! Lose the game effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::PlayerLostGameEvent;
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::LoseTheGameEffect;

/// Effect that causes a player to lose the game.
///
/// Checks for effects that prevent losing (e.g., Platinum Angel).
///
/// # Fields
///
/// * `player` - The player who loses the game
///
/// # Example
///
/// ```ignore
/// // Target player loses the game
/// let effect = LoseTheGameEffect::new(PlayerFilter::Opponent);
///
/// // You lose the game (alternate win condition trigger)
/// let effect = LoseTheGameEffect::you();
/// ```
impl EffectExecutor for LoseTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can lose the game (Platinum Angel effect)
        if !game.can_lose_game(player_id) {
            return Ok(EffectOutcome::prevented());
        }

        let lost_now = if let Some(player) = game.player_mut(player_id) {
            if player.has_lost {
                false
            } else {
                player.has_lost = true;
                true
            }
        } else {
            false
        };
        let outcome = if lost_now {
            EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
                PlayerLostGameEvent::new(player_id),
                ctx.provenance,
            ))
        } else {
            EffectOutcome::resolved()
        };
        Ok(outcome)
    }
}
