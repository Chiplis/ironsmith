//! Skip each remaining main phase this turn.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
#[cfg(test)]
use crate::game_state::Phase;
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
        super::skip_scheduled::execute_schedule_operation(
            game,
            ctx,
            &self.player,
            "skip_main_phases_this_turn",
            super::skip_scheduled::ScheduleOperation::MainPhasesThisTurn,
        )
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
