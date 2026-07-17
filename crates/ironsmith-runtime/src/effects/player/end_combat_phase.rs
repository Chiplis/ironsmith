use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase};

/// Ends the current combat phase using the ordered CR 724.2 procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EndCombatPhaseEffect;

impl EndCombatPhaseEffect {
    pub const fn new() -> Self {
        Self
    }
}

impl EffectExecutor for EndCombatPhaseEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // CR 724.2g: outside a combat phase the effect does nothing.
        if game.turn.phase != Phase::Combat {
            return Ok(EffectOutcome::resolved());
        }

        // CR 724.2a: discard abilities that triggered before the procedure
        // but have not reached the stack. TurnRunner clears its external queue
        // when it consumes the scheduler marker below.
        let _ = game.take_pending_trigger_events();
        let _ = game.take_pending_trigger_entries();

        // CR 724.2b: exile all stack objects, including this resolving one.
        super::end_turn::exile_stack_for_ending_procedure(game, ctx);

        // CR 724.2c-f require resumable runner state: a no-priority SBA pass,
        // combat cleanup, a skipped end-of-combat step, and deferred triggers.
        game.turn_store.end_combat_phase_procedure_pending = true;
        game.turn.priority_player = None;
        Ok(EffectOutcome::resolved())
    }
}
