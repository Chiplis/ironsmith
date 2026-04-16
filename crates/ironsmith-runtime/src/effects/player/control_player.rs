//! Control player effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::ControlPlayerEffect;

impl EffectExecutor for ControlPlayerEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_player = resolve_player_filter(game, &self.player, ctx)?;
        let source = game.object(ctx.source).map(|obj| obj.stable_id);

        game.add_player_control(
            ctx.controller,
            target_player,
            self.start,
            self.duration,
            source,
        );

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to control"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{PlayerControlDuration, PlayerControlStart};
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_control_player_until_end_of_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ControlPlayerEffect::until_end_of_turn(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.controlling_player_for(bob), alice);

        game.cleanup_player_control_end_of_turn();
        assert_eq!(game.controlling_player_for(bob), bob);
    }

    #[test]
    fn test_control_player_next_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ControlPlayerEffect::during_next_turn(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.controlling_player_for(bob), bob);

        game.next_turn();
        assert_eq!(game.controlling_player_for(bob), alice);

        game.cleanup_player_control_end_of_turn();
        assert_eq!(game.controlling_player_for(bob), bob);
    }
}
