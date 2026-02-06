//! Add mana of any one color effect implementation.

use crate::color::Color;
use crate::decisions::ask_mana_color;
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that adds mana of any ONE color to a player's mana pool.
///
/// Unlike `AddManaOfAnyColorEffect`, all mana must be the same color
/// (e.g., for "add three mana of any one color", the player must choose
/// all red, all blue, etc.).
///
/// # Fields
///
/// * `amount` - Number of mana to add
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Add 3 mana of any one color (must all be same color)
/// let effect = AddManaOfAnyOneColorEffect::you(3);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyOneColorEffect {
    /// Number of mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
}

impl AddManaOfAnyOneColorEffect {
    /// Create a new add mana of any one color effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create an effect where you add mana of any one color.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

impl EffectExecutor for AddManaOfAnyOneColorEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }

        // Ask player to choose ONE color (we only need to ask for 1, then add that many)
        let color = ask_mana_color(
            game,
            &mut ctx.decision_maker,
            player_id,
            ctx.source,
            None,         // All colors available
            Color::Green, // Default
        );

        // Add all the mana of the chosen color
        if let Some(p) = game.player_mut(player_id) {
            match color {
                Color::White => p.mana_pool.white += amount,
                Color::Blue => p.mana_pool.blue += amount,
                Color::Black => p.mana_pool.black += amount,
                Color::Red => p.mana_pool.red += amount,
                Color::Green => p.mana_pool.green += amount,
            }
        }

        Ok(EffectOutcome::count(amount as i32))
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

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_add_mana_of_any_one_color_default() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No decision maker, should default to green
        let effect = AddManaOfAnyOneColorEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 3);
    }

    #[test]
    fn test_add_mana_of_any_one_color_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_add_mana_of_any_one_color_single() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn test_add_mana_of_any_one_color_variable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(5);

        let effect = AddManaOfAnyOneColorEffect::you(Value::X);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 5);
    }

    #[test]
    fn test_add_mana_of_any_one_color_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_add_mana_of_any_one_color_clone_box() {
        let effect = AddManaOfAnyOneColorEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddManaOfAnyOneColorEffect"));
    }
}
