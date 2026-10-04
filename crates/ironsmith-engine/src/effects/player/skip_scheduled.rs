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
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if !ctx.claim_shared_team_structure_operation(game, player, "skip_scheduled") {
            return Ok(EffectOutcome::resolved());
        }
        let player = game.team_turn_representative(player);
        match self.kind {
            ScheduledSkipKind::Turn => game.turn_store.skip_next_turn.add(player, self.count),
            ScheduledSkipKind::CombatPhase => game
                .turn_store
                .pending_combat_phase_skips
                .add(player, self.count),
            ScheduledSkipKind::UntapStep | ScheduledSkipKind::DrawStep => {
                let step = if self.kind == ScheduledSkipKind::UntapStep {
                    Step::Untap
                } else {
                    Step::Draw
                };
                let entry = game
                    .turn_store
                    .skipped_steps
                    .entry((player, step))
                    .or_default();
                *entry = entry.saturating_add(self.count);
            }
        }
        Ok(EffectOutcome::resolved())
    }
}
