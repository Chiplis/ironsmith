//! Skip each remaining combat phase this turn.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase};
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct SkipCombatPhasesThisTurnEffect {
    pub player: PlayerFilter,
}

impl SkipCombatPhasesThisTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for SkipCombatPhasesThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if game.is_active_player(player_id)
            && matches!(
                game.turn.phase,
                Phase::Beginning | Phase::FirstMain | Phase::Combat
            )
            && ctx.claim_shared_team_structure_operation(
                game,
                player_id,
                "skip_combat_phases_this_turn",
            )
        {
            let turn_player = game.team_turn_representative(player_id);
            game.turn_store
                .skip_current_turn_combat_phases
                .insert(turn_player);
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
    fn skips_all_remaining_combat_phases_this_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;

        let mut ctx = ExecutionContext::new_default(source, alice);
        SkipCombatPhasesThisTurnEffect::new(PlayerFilter::Specific(alice))
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        advance_phase(&mut game).expect("normal combat should be skipped");
        assert_eq!(game.turn.phase, Phase::NextMain);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn_store.additional_phases.push(Phase::Combat);
        advance_phase(&mut game).expect("additional combat should also be skipped");
        assert_eq!(game.turn.phase, Phase::NextMain);
    }
}
