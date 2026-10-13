//! Independently consumable skips of future scheduled units.
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Step};
use crate::target::PlayerFilter;
use ironsmith_core::ScheduledSkipKind;

#[derive(Debug, Clone, PartialEq)]
pub struct SkipScheduledEffect {
    pub player: PlayerFilter,
    pub kind: ScheduledSkipKind,
    pub count: u32,
}
impl EffectExecutor for SkipScheduledEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        execute_schedule_operation(
            game,
            ctx,
            &self.player,
            "skip_scheduled",
            ScheduleOperation::Skip {
                kind: self.kind,
                count: self.count,
            },
        )
    }
}

/// Resolved scheduling vocabulary shared by the public adapters. Operations
/// retain their own count/expiry semantics; claim keys belong to instructions.
#[derive(Clone, Copy)]
pub(super) enum ScheduleOperation {
    Skip { kind: ScheduledSkipKind, count: u32 },
    CombatPhasesNextTurn,
    CombatPhasesThisTurn,
    MainPhasesThisTurn,
    NextCombatThisTurn,
    ExtraTurnAfterNextTurn,
}

pub(super) fn execute_schedule_operation(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: &PlayerFilter,
    claim: &'static str,
    operation: ScheduleOperation,
) -> Result<EffectOutcome, ExecutionError> {
    use crate::game_state::Phase;
    let player = resolve_player_filter(game, player, ctx)?;
    let eligible = match operation {
        ScheduleOperation::NextCombatThisTurn => {
            game.is_active_player(player)
                && matches!(game.turn.phase, Phase::Beginning | Phase::FirstMain)
        }
        ScheduleOperation::CombatPhasesThisTurn | ScheduleOperation::MainPhasesThisTurn => {
            game.is_active_player(player)
                && matches!(
                    game.turn.phase,
                    Phase::Beginning | Phase::FirstMain | Phase::Combat
                )
        }
        _ => true,
    };
    if !eligible || !ctx.claim_shared_team_structure_operation(game, player, claim) {
        return Ok(EffectOutcome::resolved());
    }
    let player = game.team_turn_representative(player);
    match operation {
        ScheduleOperation::Skip { kind, count } => match kind {
            ScheduledSkipKind::Turn => game.turn_store.skip_next_turn.add(player, count),
            ScheduledSkipKind::CombatPhase => game
                .turn_store
                .pending_combat_phase_skips
                .add(player, count),
            ScheduledSkipKind::UntapStep | ScheduledSkipKind::DrawStep => {
                let step = if kind == ScheduledSkipKind::UntapStep {
                    Step::Untap
                } else {
                    Step::Draw
                };
                let entry = game
                    .turn_store
                    .skipped_steps
                    .entry((player, step))
                    .or_default();
                *entry = entry.saturating_add(count);
            }
        },
        ScheduleOperation::CombatPhasesNextTurn => {
            game.turn_store
                .skip_all_combat_phases_next_turn
                .insert(player);
        }
        ScheduleOperation::CombatPhasesThisTurn => {
            game.turn_store
                .skip_current_turn_combat_phases
                .insert(player);
        }
        ScheduleOperation::MainPhasesThisTurn => {
            game.turn_store.skip_current_turn_main_phases.insert(player);
        }
        ScheduleOperation::NextCombatThisTurn => {
            game.turn_store.skip_next_combat_phases.insert(player);
        }
        // A scheduler operation, never a delayed trigger.
        ScheduleOperation::ExtraTurnAfterNextTurn => {
            game.turn_store
                .extra_turns_after_next_turn
                .push((player, game.turn.turn_number));
        }
    }
    Ok(EffectOutcome::resolved())
}

#[cfg(test)]
mod composition_tests {
    use super::*;

    #[test]
    fn counted_and_single_skips_accumulate_and_consume_independently() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, player);
        super::super::SkipTurnEffect::you()
            .execute(&mut game, &mut ctx)
            .unwrap();
        SkipScheduledEffect {
            player: PlayerFilter::You,
            kind: ScheduledSkipKind::Turn,
            count: 2,
        }
        .execute(&mut game, &mut ctx)
        .unwrap();
        for _ in 0..3 {
            assert!(game.turn_store.skip_next_turn.remove(&player));
        }
        assert!(!game.turn_store.skip_next_turn.remove(&player));
        super::super::SkipDrawStepEffect::you()
            .execute(&mut game, &mut ctx)
            .unwrap();
        SkipScheduledEffect {
            player: PlayerFilter::You,
            kind: ScheduledSkipKind::DrawStep,
            count: 2,
        }
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(game.pending_step_skips(player, Step::Draw), 3);
    }
}
