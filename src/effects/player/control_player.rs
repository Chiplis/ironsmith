//! Control player effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, PlayerControlDuration, PlayerControlStart};
use crate::target::{ChooseSpec, PlayerFilter};

/// Effect that lets a player control another player's decisions.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlPlayerEffect {
    /// Which player is controlled.
    pub player: PlayerFilter,
    /// When control begins.
    pub start: PlayerControlStart,
    /// How long control lasts.
    pub duration: PlayerControlDuration,
    /// Target spec if this effect targets a player.
    pub target_spec: Option<ChooseSpec>,
}

impl ControlPlayerEffect {
    /// Create a new control player effect.
    pub fn new(
        player: PlayerFilter,
        start: PlayerControlStart,
        duration: PlayerControlDuration,
    ) -> Self {
        let target_spec = match &player {
            PlayerFilter::Target(inner) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            }
            _ => None,
        };
        Self {
            player,
            start,
            duration,
            target_spec,
        }
    }

    /// Control a player until end of turn.
    pub fn until_end_of_turn(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::Immediate,
            PlayerControlDuration::UntilEndOfTurn,
        )
    }

    /// Control a player during their next turn.
    pub fn during_next_turn(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::NextTurn,
            PlayerControlDuration::UntilEndOfTurn,
        )
    }

    /// Control a player indefinitely.
    pub fn forever(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::Immediate,
            PlayerControlDuration::Forever,
        )
    }
}

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

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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

        assert_eq!(result.result, EffectResult::Resolved);
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

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.controlling_player_for(bob), bob);

        game.next_turn();
        assert_eq!(game.controlling_player_for(bob), alice);

        game.cleanup_player_control_end_of_turn();
        assert_eq!(game.controlling_player_for(bob), bob);
    }
}
