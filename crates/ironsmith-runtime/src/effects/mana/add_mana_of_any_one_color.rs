//! Add mana of any one color effect implementation.

use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub use ironsmith_core::AddManaOfAnyOneColorEffect;

use super::choice_helpers::{
    choose_mana_colors, credit_repeated_mana_symbol_from_context, mana_added_count_outcome,
};

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

        let color = choose_mana_colors(game, ctx, player_id, 1, true, None, Color::Green)
            .into_iter()
            .next()
            .unwrap_or(Color::Green);

        let symbol = ManaSymbol::from_color(color);
        credit_repeated_mana_symbol_from_context(game, player_id, symbol, amount, ctx);
        let mana = std::iter::repeat_n(symbol, amount as usize).collect::<Vec<_>>();

        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            mana,
            amount as i32,
        ))
    }

    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        Some(vec![
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Value;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_add_mana_of_any_one_color_single() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyOneColorEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
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
