//! Add mana of any color effect implementation.

use crate::color::Color;
use crate::decisions::{ManaColorsSpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that adds mana of any color(s) to a player's mana pool.
///
/// The player chooses the color of each mana independently (e.g., for "add two
/// mana of any color", the player could choose one red and one blue).
///
/// # Fields
///
/// * `amount` - Number of mana to add
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Add 2 mana of any color (can be different colors)
/// let effect = AddManaOfAnyColorEffect::you(2);
///
/// // Add X mana of any color
/// let effect = AddManaOfAnyColorEffect::you(Value::X);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyColorEffect {
    /// Number of mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
}

impl AddManaOfAnyColorEffect {
    /// Create a new add mana of any color effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create an effect where you add mana of any color.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

impl EffectExecutor for AddManaOfAnyColorEffect {
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

        // Ask player to choose colors using the new spec-based system
        let spec = ManaColorsSpec::any_color(ctx.source, amount, false);
        let mut colors = make_decision(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            spec,
        );

        // Pad or truncate to exact amount
        while colors.len() < amount as usize {
            colors.push(Color::Green); // Default
        }
        colors.truncate(amount as usize);

        // Add the mana
        if let Some(p) = game.player_mut(player_id) {
            for color in &colors {
                match color {
                    Color::White => p.mana_pool.white += 1,
                    Color::Blue => p.mana_pool.blue += 1,
                    Color::Black => p.mana_pool.black += 1,
                    Color::Red => p.mana_pool.red += 1,
                    Color::Green => p.mana_pool.green += 1,
                }
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
    fn test_add_mana_of_any_color_default() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No decision maker, should default to green
        let effect = AddManaOfAnyColorEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_add_mana_of_any_color_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyColorEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
    }

    #[test]
    fn test_add_mana_of_any_color_variable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);

        let effect = AddManaOfAnyColorEffect::you(Value::X);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 3); // Defaults to green
    }

    #[test]
    fn test_add_mana_of_any_color_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyColorEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_add_mana_of_any_color_clone_box() {
        let effect = AddManaOfAnyColorEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddManaOfAnyColorEffect"));
    }
}
