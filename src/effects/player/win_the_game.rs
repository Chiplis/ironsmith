//! Win the game effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that causes a player to win the game.
///
/// Checks for effects that prevent winning (e.g., opponent has Platinum Angel).
/// When a player wins, all other players lose.
///
/// # Fields
///
/// * `player` - The player who wins the game
///
/// # Example
///
/// ```ignore
/// // You win the game (alternate win condition)
/// let effect = WinTheGameEffect::you();
///
/// // Target player wins the game
/// let effect = WinTheGameEffect::new(PlayerFilter::Any);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct WinTheGameEffect {
    /// The player who wins the game.
    pub player: PlayerFilter,
}

impl WinTheGameEffect {
    /// Create a new win the game effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller wins the game.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for WinTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can win the game (Platinum Angel opponent effect)
        if !game.can_win_game(player_id) {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        // Player wins - mark all other players as lost
        for other_player in &mut game.players {
            if other_player.id != player_id && other_player.is_in_game() {
                other_player.has_lost = true;
            }
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

    fn setup_multiplayer_game() -> GameState {
        GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        )
    }

    #[test]
    fn test_win_the_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = WinTheGameEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Winner doesn't have has_lost set
        assert!(!game.player(alice).unwrap().has_lost);
        // Opponent loses
        assert!(game.player(bob).unwrap().has_lost);
    }

    #[test]
    fn test_win_the_game_multiplayer() {
        let mut game = setup_multiplayer_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = WinTheGameEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.player(alice).unwrap().has_lost);
        assert!(game.player(bob).unwrap().has_lost);
        assert!(game.player(charlie).unwrap().has_lost);
    }

    #[test]
    fn test_win_the_game_prevented_by_cant_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Simulate opponent's Platinum Angel effect
        game.cant_effects.cant_win_game.insert(alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = WinTheGameEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
        assert!(!game.player(alice).unwrap().has_lost);
        assert!(!game.player(bob).unwrap().has_lost);
    }

    #[test]
    fn test_win_the_game_clone_box() {
        let effect = WinTheGameEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("WinTheGameEffect"));
    }
}
