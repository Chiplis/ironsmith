//! Additional phase effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase};
use crate::turn::next_phase;
pub use ironsmith_core::{AdditionalPhase, AdditionalPhasesEffect, AdditionalStepAnchor, AdditionalStepKind, AdditionalStepsEffect};

impl EffectExecutor for AdditionalPhasesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = ctx.iteration.iterated_player.unwrap_or(ctx.controller);
        // "After this main phase, ..." / "If it's your main phase, ... after
        // this phase": outside the controller's own main phase there is no
        // "this main phase" to follow, so no phases are added.
        if self.after_main_phase
            && (!matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                || !game.is_active_player(player))
        {
            return Ok(EffectOutcome::resolved());
        }
        if !ctx.claim_shared_team_structure_operation(game, player, "additional_phases") {
            return Ok(EffectOutcome::resolved());
        }
        if game.turn_store.additional_phase_continuation.is_none()
            && game.turn_store.phase_schedule_continuation.is_none()
        {
            game.turn_store.additional_phase_continuation = next_phase(game.turn.phase);
        }
        let phases = self.phases.iter().map(|phase| match phase {
            AdditionalPhase::Combat => Phase::Combat,
            AdditionalPhase::Main => Phase::NextMain,
            AdditionalPhase::Beginning => Phase::Beginning,
        });
        let order = game.add_additional_phase_group(phases);
        ctx.combat.last_added_combat_order = if self.phases.contains(&AdditionalPhase::Combat) {
            order
        } else {
            None
        };
        Ok(EffectOutcome::resolved())
    }
}

impl EffectExecutor for AdditionalStepsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::game_state::Step;
        // CR 500.10a: "you get" additions happen only during that player's
        // own turn.
        if let Some(player) = &self.player {
            let player = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            if !game.is_active_player(player) {
                return Ok(EffectOutcome::resolved());
            }
        }
        let count = crate::effects::helpers::resolve_value(game, &self.count, ctx)?.max(0);
        let step = match self.step {
            AdditionalStepKind::Untap => Step::Untap,
            AdditionalStepKind::Upkeep => Step::Upkeep,
            AdditionalStepKind::Draw => Step::Draw,
            AdditionalStepKind::End => Step::End,
        };
        for _ in 0..count {
            match self.anchor {
                AdditionalStepAnchor::AfterThisStep => {
                    // Outside a step (a main phase) there is no "this step".
                    let Some(current) = game.turn.step else {
                        return Ok(EffectOutcome::resolved());
                    };
                    game.add_step_after(step, current);
                }
                AdditionalStepAnchor::AfterThisPhase => {
                    game.add_step_after_phase(step, game.turn.phase);
                }
            }
        }
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

    #[test]
    fn most_recently_created_phase_group_occurs_first() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let mut ctx = ExecutionContext::new_default(source, alice);

        AdditionalPhasesEffect::combat_then_main()
            .execute(&mut game, &mut ctx)
            .expect("first phase group resolves");
        AdditionalPhasesEffect::combat()
            .execute(&mut game, &mut ctx)
            .expect("later phase resolves");

        assert_eq!(
            game.turn_store.additional_phases,
            vec![Phase::Combat, Phase::Combat, Phase::NextMain],
            "the later-created combat occurs before the earlier combat/main group"
        );
        advance_phase(&mut game).expect("advance to most recent combat");
        assert_eq!(game.turn.phase, Phase::Combat);
        game.turn.step = None;
        advance_phase(&mut game).expect("advance to earlier combat");
        assert_eq!(game.turn.phase, Phase::Combat);
        game.turn.step = None;
        advance_phase(&mut game).expect("advance to earlier group's main");
        assert_eq!(game.turn.phase, Phase::NextMain);
    }
}
