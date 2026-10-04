//! Generic counters placed on players.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_nonnegative_u32};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;

/// Give a player counters of a typed counter kind.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCountersEffect {
    pub counter_type: CounterType,
    pub count: Value,
    pub player: PlayerFilter,
}

impl PlayerCountersEffect {
    pub fn new(counter_type: CounterType, count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            counter_type,
            count: count.into(),
            player,
        }
    }
}

impl EffectExecutor for PlayerCountersEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Replacement choices are deferred until the counter event is applied.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let player = resolve_player_filter(game, &self.player, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let count = resolve_nonnegative_u32(game, &self.count, ctx)?;
            let event = crate::events::Event::put_player_counters(
                player,
                self.counter_type,
                count,
                ctx.cause.clone(),
            )
            .with_provenance(ctx.provenance);
            crate::effects::counters::execute_player_counter_placement(game, ctx, event)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
    }
}

#[cfg(test)]
#[path = "player_counter_choice_tests.rs"]
mod choice_tests;
