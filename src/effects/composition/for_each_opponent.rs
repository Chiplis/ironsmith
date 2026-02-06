//! ForEachOpponent effect implementation.
//!
//! Note: This is a convenience wrapper around `ForPlayersEffect` with `PlayerFilter::Opponent`.
//! Consider using `ForPlayersEffect` directly for more flexibility.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::composition::ForPlayersEffect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;

/// Effect that applies effects once for each opponent.
///
/// Sets `ctx.iterated_player` for each iteration, allowing inner effects
/// to reference the current opponent via `PlayerFilter::IteratedPlayer`.
///
/// Note: This is a convenience wrapper around `ForPlayersEffect` with `PlayerFilter::Opponent`.
///
/// # Fields
///
/// * `effects` - Effects to execute for each opponent
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to each opponent
/// let effect = ForEachOpponentEffect::new(vec![
///     Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer)),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachOpponentEffect {
    /// The underlying ForPlayersEffect that does the actual work.
    inner: ForPlayersEffect,
}

impl ForEachOpponentEffect {
    /// Create a new ForEachOpponent effect.
    pub fn new(effects: Vec<Effect>) -> Self {
        Self {
            inner: ForPlayersEffect::new(PlayerFilter::Opponent, effects),
        }
    }

    /// Get the effects being executed.
    pub fn effects(&self) -> &[Effect] {
        &self.inner.effects
    }
}

impl EffectExecutor for ForEachOpponentEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.inner.execute(game, ctx)
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
    fn test_for_each_opponent_two_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_alice_life = game.player(alice).unwrap().life;
        let initial_bob_life = game.player(bob).unwrap().life;

        // Each opponent loses 3 life (which Alice gains)
        // In 2-player, this is just Bob
        let effect = ForEachOpponentEffect::new(vec![Effect::gain_life(3)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Count should be 3 (one opponent, one effect returning 3)
        assert_eq!(result.result, EffectResult::Count(3));
        // Alice should gain 3 life
        assert_eq!(game.player(alice).unwrap().life, initial_alice_life + 3);
        // Bob's life unchanged (effect is gain_life for controller)
        assert_eq!(game.player(bob).unwrap().life, initial_bob_life);
    }

    #[test]
    fn test_for_each_opponent_multiplayer() {
        let mut game = setup_multiplayer_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // Each opponent (Bob and Charlie) triggers gain 2 life for Alice
        let effect = ForEachOpponentEffect::new(vec![Effect::gain_life(2)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Count should be 4 (2 opponents * 2 life each)
        assert_eq!(result.result, EffectResult::Count(4));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 4);
    }

    #[test]
    fn test_for_each_opponent_no_opponents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Eliminate Bob
        if let Some(p) = game.player_mut(bob) {
            p.has_lost = true;
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForEachOpponentEffect::new(vec![Effect::gain_life(5)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No opponents in game
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_for_each_opponent_preserves_iterated_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Set an initial iterated_player
        let original = PlayerId::from_index(99);
        ctx.iterated_player = Some(original);

        let effect = ForEachOpponentEffect::new(vec![Effect::gain_life(1)]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should restore original iterated_player
        assert_eq!(ctx.iterated_player, Some(original));
    }

    #[test]
    fn test_for_each_opponent_clone_box() {
        let effect = ForEachOpponentEffect::new(vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ForEachOpponentEffect"));
    }
}
