//! Generic counters placed on players.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
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
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
        let Some(event) = game.add_player_counters_with_source(
            player,
            self.counter_type,
            count,
            Some(ctx.source),
            Some(ctx.controller),
        ) else {
            return Ok(EffectOutcome::count(0));
        };
        Ok(EffectOutcome::count(count as i32).with_event(event))
    }
}
