//! Skip draw step effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::game_state::Step;
use crate::target::PlayerFilter;

/// Effect that causes a player to skip their next draw step.
#[derive(Debug, Clone, PartialEq)]
pub struct SkipDrawStepEffect {
    /// The player who skips their next draw step.
    pub player: PlayerFilter,
}

impl SkipDrawStepEffect {
    /// Create a new skip draw step effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller skips their next draw step.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for SkipDrawStepEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if !ctx.claim_shared_team_structure_operation(game, player_id, "skip_draw_step") {
            return Ok(EffectOutcome::resolved());
        }
        game.skip_next_step(player_id, Step::Draw);
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
    fn test_skip_draw_step_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipDrawStepEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.pending_step_skips(alice, Step::Draw), 1);
    }

    #[test]
    fn test_skip_draw_step_specific_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipDrawStepEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.pending_step_skips(alice, Step::Draw), 0);
        assert_eq!(game.pending_step_skips(bob, Step::Draw), 1);
    }
}
