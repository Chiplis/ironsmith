//! Exchange life totals effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that exchanges life totals between two players.
///
/// Used by cards like "Exchange life totals with target player."
/// Both players' life totals are simultaneously set to what
/// the other player's life total was.
///
/// # Fields
///
/// * `player1` - First player in the exchange (usually the controller)
/// * `player2` - Second player in the exchange (usually target opponent)
///
/// # Example
///
/// ```ignore
/// // Exchange life totals with target player
/// let effect = ExchangeLifeTotalsEffect::with_target();
///
/// // Or with a specific opponent
/// let effect = ExchangeLifeTotalsEffect::with_opponent();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeLifeTotalsEffect {
    /// First player in the exchange.
    pub player1: PlayerFilter,
    /// Second player in the exchange.
    pub player2: PlayerFilter,
}

impl ExchangeLifeTotalsEffect {
    /// Create a new exchange life totals effect.
    pub fn new(player1: PlayerFilter, player2: PlayerFilter) -> Self {
        Self { player1, player2 }
    }

    /// Create an effect that exchanges life totals with target opponent.
    pub fn with_opponent() -> Self {
        Self::new(PlayerFilter::You, PlayerFilter::Opponent)
    }

    /// Create an effect that exchanges life totals with target player.
    pub fn with_target() -> Self {
        Self::new(
            PlayerFilter::You,
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        )
    }
}

impl EffectExecutor for ExchangeLifeTotalsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player1_id = resolve_player_filter(game, &self.player1, ctx)?;
        let player2_id = resolve_player_filter(game, &self.player2, ctx)?;

        let life1 = game.player(player1_id).map(|p| p.life).unwrap_or(0);
        let life2 = game.player(player2_id).map(|p| p.life).unwrap_or(0);

        // Check if life totals can change
        if !game.can_change_life_total(player1_id) || !game.can_change_life_total(player2_id) {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        if let Some(p1) = game.player_mut(player1_id) {
            p1.life = life2;
        }
        if let Some(p2) = game.player_mut(player2_id) {
            p2.life = life1;
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
