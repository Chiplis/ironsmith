//! ForPlayers effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::ids::PlayerId;

/// Effect that applies effects once for each player matching a filter.
///
/// Sets `ctx.iterated_player` for each iteration, allowing inner effects
/// to reference the current player via `PlayerFilter::IteratedPlayer`.
///
/// # Fields
///
/// * `filter` - Filter for which players to iterate over
/// * `effects` - Effects to execute for each matching player
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to each opponent
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Opponent,
///     vec![Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer))],
/// );
///
/// // Each player draws a card
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Any,
///     vec![Effect::target_draws(1, PlayerFilter::IteratedPlayer)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForPlayersEffect {
    /// Filter for which players to iterate over.
    pub filter: PlayerFilter,
    /// Effects to execute for each matching player.
    pub effects: Vec<Effect>,
}

impl ForPlayersEffect {
    /// Create a new ForPlayers effect.
    pub fn new(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        Self { filter, effects }
    }
}

impl EffectExecutor for ForPlayersEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);

        // Iterate over all players that match the filter
        let players: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .filter(|p| self.filter.matches_player(p.id, &filter_ctx))
            .map(|p| p.id)
            .collect();

        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();

        for player_id in players {
            ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                // Execute all inner effects for this player
                for effect in &self.effects {
                    outcomes.push(execute_effect(game, effect, ctx)?);
                }
                Ok::<(), ExecutionError>(())
            })?;
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }
}
