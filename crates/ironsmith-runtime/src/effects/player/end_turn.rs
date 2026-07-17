use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

#[derive(Debug, Clone, PartialEq)]
pub struct EndTurnEffect {
    pub player: crate::target::PlayerFilter,
}

impl EndTurnEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for EndTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if !game.is_active_player(player) {
            return Ok(EffectOutcome::resolved());
        }

        // CR 724.1a: discard abilities that triggered before this procedure
        // began but have not reached the stack. The external TriggerQueue is
        // cleared by TurnRunner when it consumes the scheduler marker below.
        // Delayed-trigger definitions that have not triggered remain active.
        let _ = game.take_pending_trigger_events();
        let _ = game.take_pending_trigger_entries();

        // CR 724.1b: the resolving spell/ability has already been popped from
        // GameState::stack, so include its concrete stack object explicitly.
        // Ordinary ability entries have no separate zone object; spell and
        // ability copies do, and therefore move to exile before the SBA pass.
        exile_stack_for_ending_procedure(game, ctx);

        // CR 724.1c-f require turn-runner state (including resumable SBA and
        // cleanup decisions), so yield the remainder of the procedure to it.
        game.turn_store.end_turn_procedure_pending = true;
        game.turn.priority_player = None;
        Ok(EffectOutcome::resolved())
    }
}

/// Exile every concrete object on the stack, including the resolving object.
///
/// Both CR 724.1b and 724.2b use this exact transaction. Ordinary ability
/// entries whose source is elsewhere have no independent zone object to move.
pub(crate) fn exile_stack_for_ending_procedure(game: &mut GameState, ctx: &mut ExecutionContext) {
    use crate::zone::Zone;

    let mut stack_objects = Vec::with_capacity(game.stack.len() + 1);
    if let Some(resolving_object) = ctx.cause.source {
        stack_objects.push(resolving_object);
    }
    stack_objects.extend(game.stack.iter().rev().map(|entry| entry.object_id));
    game.stack.clear();
    let mut seen = std::collections::HashSet::new();
    stack_objects.retain(|object_id| seen.insert(*object_id));
    for object_id in stack_objects {
        if game
            .object(object_id)
            .is_some_and(|object| object.zone == Zone::Stack)
        {
            let _ = crate::effects::zones::apply_zone_change(
                game,
                object_id,
                Zone::Stack,
                Zone::Exile,
                crate::events::cause::EventCause::from_game_rule(),
                &mut *ctx.decision_maker,
            );
        }
    }
}
