//! Additional phase effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase};
use crate::turn::next_phase;
pub use ironsmith_core::{AdditionalPhase, AdditionalPhasesEffect};

impl EffectExecutor for AdditionalPhasesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if game.turn_store.additional_phase_continuation.is_none() {
            game.turn_store.additional_phase_continuation = next_phase(game.turn.phase);
        }
        game.turn_store
            .additional_phases
            .extend(self.phases.iter().map(|phase| match phase {
                AdditionalPhase::Combat => Phase::Combat,
                AdditionalPhase::Main => Phase::NextMain,
            }));
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::ExecutionContext;
    use crate::game_state::Step;
    use crate::ids::PlayerId;
    use crate::turn::advance_phase;

    #[test]
    fn additional_combat_then_main_is_inserted_before_normal_next_phase() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let mut ctx = ExecutionContext::new_default(source, alice);
        AdditionalPhasesEffect::combat_then_main()
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        advance_phase(&mut game).expect("advance to inserted combat");
        assert_eq!(game.turn.phase, Phase::Combat);
        assert_eq!(game.turn.step, Some(Step::BeginCombat));

        game.turn.step = None;
        advance_phase(&mut game).expect("advance to inserted main");
        assert_eq!(game.turn.phase, Phase::NextMain);
        assert_eq!(game.turn.step, None);

        advance_phase(&mut game).expect("advance to normal combat");
        assert_eq!(game.turn.phase, Phase::Combat);
    }
}
