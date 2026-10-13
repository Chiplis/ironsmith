//! Child instructions with already frozen operands and explicit event attribution.
//! These are transient program nodes; authored effects retain their shared models.
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

#[derive(Debug, Clone)]
pub(crate) struct ResolvedLifeLoss {
    pub player: crate::target::PlayerFilter,
    pub amount: u32,
    pub from_radiation: bool,
}
impl EffectExecutor for ResolvedLifeLoss {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Leaf
    }
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
        let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        let event = if self.from_radiation {
            crate::events::LifeLossEvent::from_radiation(player, self.amount)
        } else {
            crate::events::LifeLossEvent::from_effect(player, self.amount)
        };
        crate::effects::life::life_change::execute_life_change_with_outputs(
            game,
            ctx,
            crate::events::Event::new_with_provenance(event, ctx.provenance),
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedPlayerCounterRemoval {
    pub player: crate::target::PlayerFilter,
    pub counter: crate::object::CounterType,
    pub count: u32,
    pub source: Option<crate::ids::ObjectId>,
    pub controller: Option<crate::ids::PlayerId>,
}
impl EffectExecutor for ResolvedPlayerCounterRemoval {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Leaf
    }
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
        let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        crate::effects::counters::execute_player_counter_removal_with_outputs(
            game,
            ctx,
            player,
            self.counter,
            self.count,
            self.source,
            self.controller,
        )
    }
}
