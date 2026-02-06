//! Lose the game effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that causes a player to lose the game.
///
/// Checks for effects that prevent losing (e.g., Platinum Angel).
///
/// # Fields
///
/// * `player` - The player who loses the game
///
/// # Example
///
/// ```ignore
/// // Target player loses the game
/// let effect = LoseTheGameEffect::new(PlayerFilter::Opponent);
///
/// // You lose the game (alternate win condition trigger)
/// let effect = LoseTheGameEffect::you();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LoseTheGameEffect {
    /// The player who loses the game.
    pub player: PlayerFilter,
}

impl LoseTheGameEffect {
    /// Create a new lose the game effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller loses the game.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Target opponent loses the game.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl EffectExecutor for LoseTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can lose the game (Platinum Angel effect)
        if !game.can_lose_game(player_id) {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        if let Some(player) = game.player_mut(player_id) {
            player.has_lost = true;
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

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_lose_the_game_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = LoseTheGameEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.player(alice).unwrap().has_lost);
    }

    #[test]
    fn test_lose_the_game_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = LoseTheGameEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.player(alice).unwrap().has_lost);
        assert!(game.player(bob).unwrap().has_lost);
    }

    #[test]
    fn test_lose_the_game_prevented_by_cant_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Simulate Platinum Angel effect
        game.cant_effects.cant_lose_game.insert(alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = LoseTheGameEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
        assert!(!game.player(alice).unwrap().has_lost);
    }

    #[test]
    fn test_lose_the_game_clone_box() {
        let effect = LoseTheGameEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("LoseTheGameEffect"));
    }
}
