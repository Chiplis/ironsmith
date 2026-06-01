//! Skip each remaining main phase this turn.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase};
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct SkipMainPhasesThisTurnEffect {
    pub player: PlayerFilter,
}

impl SkipMainPhasesThisTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for SkipMainPhasesThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if player_id == game.turn.active_player
            && matches!(
                game.turn.phase,
                Phase::Beginning | Phase::FirstMain | Phase::Combat
            )
        {
            game.turn_store
                .skip_current_turn_main_phases
                .insert(player_id);
        }
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::turn::advance_phase;

    #[test]
    fn skips_precombat_main_and_remains_for_postcombat_main() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.turn.active_player = alice;
        game.turn.phase = Phase::Beginning;

        let mut ctx = ExecutionContext::new_default(source, alice);
        SkipMainPhasesThisTurnEffect::new(PlayerFilter::Specific(alice))
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        advance_phase(&mut game).expect("advance to combat instead of first main");
        assert_eq!(game.turn.phase, Phase::Combat);

        advance_phase(&mut game).expect("advance to ending instead of second main");
        assert_eq!(game.turn.phase, Phase::Ending);
    }
}
