//! Skip combat phases effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that causes a player to skip all combat phases of their next turn.
#[derive(Debug, Clone, PartialEq)]
pub struct SkipCombatPhasesEffect {
    /// The player who skips combat phases on their next turn.
    pub player: PlayerFilter,
}

impl SkipCombatPhasesEffect {
    /// Create a new skip combat phases effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller skips all combat phases on their next turn.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for SkipCombatPhasesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        game.turn_store.skip_next_combat_phases.insert(player_id);
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_skip_combat_phases_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipCombatPhasesEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(game.turn_store.skip_next_combat_phases.contains(&alice));
    }

    #[test]
    fn test_skip_combat_phases_specific_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipCombatPhasesEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(!game.turn_store.skip_next_combat_phases.contains(&alice));
        assert!(game.turn_store.skip_next_combat_phases.contains(&bob));
    }
}
