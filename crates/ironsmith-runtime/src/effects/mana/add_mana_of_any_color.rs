//! Add mana of any color effect implementation.

use super::choice_helpers::{
    choose_mana_colors, credit_mana_symbols_from_context, mana_added_count_outcome,
};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub use ironsmith_core::AddManaOfAnyColorEffect;

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
impl EffectExecutor for AddManaOfAnyColorEffect {
    fn directly_produces_mana(&self) -> bool {
        self.available_colors
            .as_ref()
            .is_none_or(|colors| !colors.is_empty())
    }

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

        let colors = choose_mana_colors(
            game,
            ctx,
            player_id,
            amount,
            false,
            self.available_colors.as_deref(),
            Color::Green,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let symbols = colors
            .into_iter()
            .map(ManaSymbol::from_color)
            .collect::<Vec<_>>();
        let symbols = credit_mana_symbols_from_context(game, player_id, symbols, ctx);

        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            symbols.clone(),
            symbols.len() as i32,
        ))
    }

    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let symbols = if let Some(colors) = &self.available_colors {
            colors
                .iter()
                .copied()
                .map(ManaSymbol::from_color)
                .collect::<Vec<_>>()
        } else {
            vec![
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
                ManaSymbol::Green,
            ]
        };
        Some(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;
    use crate::test_prelude::*;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_add_mana_of_any_color_clone_box() {
        let effect = AddManaOfAnyColorEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddManaOfAnyColorEffect"));
    }

    #[test]
    fn test_add_mana_of_any_color_restricted_defaults_to_allowed_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaOfAnyColorEffect::you_restricted(2, vec![Color::Red, Color::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 2);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
    }
}
