//! Add mana effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Effect that adds specific mana symbols to a player's mana pool.
///
/// # Fields
///
/// * `mana` - The mana symbols to add
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Add two green mana
/// let effect = AddManaEffect::new(vec![ManaSymbol::Green, ManaSymbol::Green], PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaEffect {
    /// The mana symbols to add.
    pub mana: Vec<ManaSymbol>,
    /// Which player receives the mana.
    pub player: PlayerFilter,
}

impl AddManaEffect {
    /// Create a new add mana effect.
    pub fn new(mana: Vec<ManaSymbol>, player: PlayerFilter) -> Self {
        Self { mana, player }
    }

    /// Create an effect where you add mana.
    pub fn you(mana: Vec<ManaSymbol>) -> Self {
        Self::new(mana, PlayerFilter::You)
    }
}

impl EffectExecutor for AddManaEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        if let Some(p) = game.player_mut(player_id) {
            for symbol in &self.mana {
                p.mana_pool.add(*symbol, 1);
            }
        }

        Ok(EffectOutcome::from_result(EffectResult::ManaAdded(
            self.mana.clone(),
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
    fn test_add_mana_single_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![ManaSymbol::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![ManaSymbol::Green])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn test_add_mana_multiple_same_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![ManaSymbol::Green, ManaSymbol::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![ManaSymbol::Green, ManaSymbol::Green])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_add_mana_multiple_colors() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![ManaSymbol::Red, ManaSymbol::Blue, ManaSymbol::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![ManaSymbol::Red, ManaSymbol::Blue, ManaSymbol::Green])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn test_add_mana_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::new(vec![ManaSymbol::White], PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.result,
            EffectResult::ManaAdded(vec![ManaSymbol::White])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.white, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.white, 1);
    }

    #[test]
    fn test_add_mana_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::ManaAdded(vec![]));
    }

    #[test]
    fn test_add_mana_clone_box() {
        let effect = AddManaEffect::you(vec![ManaSymbol::Black]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddManaEffect"));
    }
}
