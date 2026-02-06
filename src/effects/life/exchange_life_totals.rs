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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_exchange_life_totals_basic() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set different life totals
        game.player_mut(alice).unwrap().life = 15;
        game.player_mut(bob).unwrap().life = 25;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::You, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Life totals should be swapped
        assert_eq!(game.player(alice).unwrap().life, 25);
        assert_eq!(game.player(bob).unwrap().life, 15);
    }

    #[test]
    fn test_exchange_life_totals_same_life() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Both at 20
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::You, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Both still at 20
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }

    #[test]
    fn test_exchange_life_totals_with_low_life() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice at 2, Bob at 40
        game.player_mut(alice).unwrap().life = 2;
        game.player_mut(bob).unwrap().life = 40;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::You, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, 40);
        assert_eq!(game.player(bob).unwrap().life, 2);
    }

    #[test]
    fn test_exchange_life_totals_with_negative_life() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice at -5, Bob at 10
        game.player_mut(alice).unwrap().life = -5;
        game.player_mut(bob).unwrap().life = 10;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::You, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, 10);
        assert_eq!(game.player(bob).unwrap().life, -5);
    }

    #[test]
    fn test_exchange_life_totals_prevented_by_cant_effect() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set different life totals
        game.player_mut(alice).unwrap().life = 15;
        game.player_mut(bob).unwrap().life = 25;

        // Prevent alice from changing life total
        game.cant_effects.life_total_cant_change.insert(alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::You, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
        // Life totals unchanged
        assert_eq!(game.player(alice).unwrap().life, 15);
        assert_eq!(game.player(bob).unwrap().life, 25);
    }

    #[test]
    fn test_exchange_life_totals_clone_box() {
        let effect = ExchangeLifeTotalsEffect::with_opponent();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExchangeLifeTotalsEffect"));
    }
}
