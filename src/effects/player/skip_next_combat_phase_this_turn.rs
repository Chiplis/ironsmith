//! Skip next combat phase this turn effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::game_state::Phase;
use crate::target::PlayerFilter;

/// Effect that causes a player to skip their next combat phase this turn.
#[derive(Debug, Clone, PartialEq)]
pub struct SkipNextCombatPhaseThisTurnEffect {
    /// The player who skips their next combat phase this turn.
    pub player: PlayerFilter,
}

impl SkipNextCombatPhaseThisTurnEffect {
    /// Create a new skip-next-combat-phase-this-turn effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller skips their next combat phase this turn.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for SkipNextCombatPhaseThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        // This effect is scoped to the current turn only. If that player is not the
        // active player or combat has already passed, there is no combat phase left
        // for them this turn in the current turn model.
        let before_combat = matches!(game.turn.phase, Phase::Beginning | Phase::FirstMain);
        if player_id == game.turn.active_player && before_combat {
            game.skip_next_combat_phases.insert(player_id);
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
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_sets_skip_for_active_player_before_combat() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipNextCombatPhaseThisTurnEffect::new(PlayerFilter::Specific(alice));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.skip_next_combat_phases.contains(&alice));
    }

    #[test]
    fn test_noop_for_non_active_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipNextCombatPhaseThisTurnEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.skip_next_combat_phases.contains(&bob));
    }
}
