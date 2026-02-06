//! Add colorless mana effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Effect that adds colorless mana to a player's mana pool.
///
/// # Fields
///
/// * `amount` - The amount of colorless mana to add (can be fixed or variable)
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Add 3 colorless mana
/// let effect = AddColorlessManaEffect::new(3, PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AddColorlessManaEffect {
    /// The amount of colorless mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
}

impl AddColorlessManaEffect {
    /// Create a new add colorless mana effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create an effect where you add colorless mana.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

impl EffectExecutor for AddColorlessManaEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        if let Some(p) = game.player_mut(player_id) {
            p.mana_pool.add(ManaSymbol::Colorless, count);
        }

        let mana_added: Vec<ManaSymbol> = (0..count).map(|_| ManaSymbol::Colorless).collect();
        Ok(EffectOutcome::from_result(EffectResult::ManaAdded(
            mana_added,
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_add_colorless_mana_fixed() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddColorlessManaEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
                ManaSymbol::Colorless
            ])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 3);
    }

    #[test]
    fn test_add_colorless_mana_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddColorlessManaEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::ManaAdded(vec![]));
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
    }

    #[test]
    fn test_add_colorless_mana_negative_becomes_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddColorlessManaEffect::new(Value::Fixed(-5), PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Negative amounts are clamped to 0
        assert_eq!(result.result, EffectResult::ManaAdded(vec![]));
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
    }

    #[test]
    fn test_add_colorless_mana_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(4);

        let effect = AddColorlessManaEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
                ManaSymbol::Colorless
            ])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 4);
    }

    #[test]
    fn test_add_colorless_mana_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddColorlessManaEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![ManaSymbol::Colorless, ManaSymbol::Colorless])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.colorless, 2);
    }

    #[test]
    fn test_add_colorless_mana_clone_box() {
        let effect = AddColorlessManaEffect::you(2);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddColorlessManaEffect"));
    }
}
