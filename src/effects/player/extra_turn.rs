//! Extra turn effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that gives a player an extra turn.
///
/// Adds a turn to the extra turns queue.
///
/// # Fields
///
/// * `player` - The player who gets the extra turn
///
/// # Example
///
/// ```ignore
/// // Take an extra turn after this one
/// let effect = ExtraTurnEffect::you();
///
/// // Target player takes an extra turn
/// let effect = ExtraTurnEffect::new(PlayerFilter::Any);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnEffect {
    /// The player who gets the extra turn.
    pub player: PlayerFilter,
}

impl ExtraTurnEffect {
    /// Create a new extra turn effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller gets an extra turn.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for ExtraTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Add an extra turn for this player
        game.extra_turns.push(player_id);

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

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_extra_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        assert!(game.extra_turns.is_empty());

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExtraTurnEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.extra_turns.len(), 1);
        assert_eq!(game.extra_turns[0], alice);
    }

    #[test]
    fn test_extra_turn_for_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExtraTurnEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.extra_turns.len(), 1);
        assert_eq!(game.extra_turns[0], bob);
    }

    #[test]
    fn test_multiple_extra_turns() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExtraTurnEffect::you();

        // Take three extra turns
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.extra_turns.len(), 3);
    }

    #[test]
    fn test_extra_turn_clone_box() {
        let effect = ExtraTurnEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExtraTurnEffect"));
    }
}
