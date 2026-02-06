//! Set life total effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that sets a player's life total to a specific value.
///
/// This is different from gaining or losing life:
/// - If the new total is higher, the player gains the difference
/// - If the new total is lower, the player loses the difference
/// - Used by cards like "Your life total becomes 10"
///
/// # Fields
///
/// * `amount` - The life total to set (can be fixed or variable)
/// * `player` - Which player's life total changes
///
/// # Example
///
/// ```ignore
/// // Set life total to 10 (like Sorin Markov's ability)
/// let effect = SetLifeTotalEffect {
///     amount: Value::Fixed(10),
///     player: PlayerFilter::Opponent,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SetLifeTotalEffect {
    /// The life total to set.
    pub amount: Value,
    /// Which player's life total changes.
    pub player: PlayerFilter,
}

impl SetLifeTotalEffect {
    /// Create a new set life total effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create an effect that sets your life total.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }

    /// Create an effect that sets an opponent's life total.
    pub fn opponent(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::Opponent)
    }
}

impl EffectExecutor for SetLifeTotalEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?;

        if let Some(p) = game.player_mut(player_id) {
            p.life = amount;
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
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_set_life_total_lower() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SetLifeTotalEffect::you(10);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, 10);
    }

    #[test]
    fn test_set_life_total_higher() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SetLifeTotalEffect::you(30);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, 30);
    }

    #[test]
    fn test_set_life_total_same() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SetLifeTotalEffect::you(20);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_set_life_total_negative() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SetLifeTotalEffect::you(-5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, -5);
    }

    #[test]
    fn test_set_life_total_specific_player() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Use SpecificPlayer to target Bob
        let effect = SetLifeTotalEffect::new(1, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Alice unchanged
        assert_eq!(game.player(alice).unwrap().life, 20);
        // Bob's life set to 1
        assert_eq!(game.player(bob).unwrap().life, 1);
    }

    #[test]
    fn test_set_life_total_zero() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SetLifeTotalEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, 0);
    }

    #[test]
    fn test_set_life_total_clone_box() {
        let effect = SetLifeTotalEffect::you(10);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("SetLifeTotalEffect"));
    }
}
