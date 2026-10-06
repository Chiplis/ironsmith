//! Poison counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::helpers::{resolve_player_filter, resolve_nonnegative_u32};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;

/// Effect that gives a player poison counters.
///
/// # Fields
///
/// * `count` - How many poison counters to add (can be fixed or variable)
/// * `player` - Which player receives the poison counters
///
/// # Example
///
/// ```ignore
/// // Give yourself 2 poison counters (e.g., from a cost)
/// let effect = PoisonCountersEffect::you(2);
///
/// // Give a specific player 3 poison counters
/// let effect = PoisonCountersEffect::new(3, PlayerFilter::Specific(opponent_id));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PoisonCountersEffect {
    /// How many poison counters to add.
    pub count: Value,
    /// Which player receives the counters.
    pub player: PlayerFilter,
}

impl PoisonCountersEffect {
    /// Create a new poison counters effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you get poison counters.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for PoisonCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let count = resolve_nonnegative_u32(game, &self.count, ctx)?;
                let event = crate::events::Event::put_player_counters(
                    player,
                    CounterType::Poison,
                    count,
                    ctx.cause.clone(),
                )
                .with_provenance(ctx.provenance);
                crate::effects::counters::execute_player_counter_placement_with_outputs(
                    game, ctx, event,
                )
            },
        );
        // Preserve this adapter's existing neutral result for a suspended child,
        // including a child that failed after opening its decision. The shared
        // transaction owns rollback; an ordinary failure still propagates.
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }
}
