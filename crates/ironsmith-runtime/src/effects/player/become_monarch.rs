//! Become-the-monarch effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct BecomeMonarchEffect {
    pub player: PlayerFilter,
}

impl BecomeMonarchEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for BecomeMonarchEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if !game.can_become_monarch(player_id) {
            return Ok(EffectOutcome::prevented());
        }
        game.set_monarch(Some(player_id));
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    #[test]
    fn become_monarch_sets_designation_for_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = BecomeMonarchEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("resolve become-monarch effect");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.monarch, Some(alice));
    }

    #[test]
    fn become_monarch_sets_designation_for_selected_player() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        BecomeMonarchEffect::new(PlayerFilter::Specific(bob))
            .execute(&mut game, &mut ctx)
            .expect("resolve become-monarch effect for chosen player");

        assert_eq!(game.monarch, Some(bob));
    }
}
