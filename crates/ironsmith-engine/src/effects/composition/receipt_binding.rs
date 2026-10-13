//! Bind a retained optional-cost receipt quantity for an ordinary child program.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::types::CardType;

#[derive(Debug, Clone)]
pub(crate) struct WithOptionalCostPower {
    pub cost_ref: crate::cost::OptionalCostRef,
    pub result: crate::effect::EffectId,
    pub body: Effect,
}

impl WithOptionalCostPower {
    fn receipt<'a>(
        &self,
        game: &'a GameState,
        ctx: &'a ExecutionContext,
    ) -> Option<&'a ObjectSnapshot> {
        let key = ironsmith_core::tag::optional_cost_sacrifice_receipt(&self.cost_ref);
        let lookup =
            |tags: &'a std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>| {
                tags.get(&key)?
                    .iter()
                    .find(|snapshot| snapshot.card_types.contains(&CardType::Creature))
            };
        lookup(&ctx.tagged_objects)
            .or_else(|| {
                game.object(ctx.source)
                    .and_then(|spell| lookup(&spell.cast_tagged_objects))
            })
            .or_else(|| {
                game.turn_store
                    .cast_spell_lki
                    .get(&ctx.source)
                    .and_then(|lki| lookup(&lki.0.cast_tagged_objects))
            })
    }
}

impl EffectExecutor for WithOptionalCostPower {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Inspect
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.body);
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
        let Some(receipt) = self.receipt(game, ctx) else {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::impossible(),
            ));
        };
        // This is retained payment evidence, never the current object's power.
        let power = receipt.power.unwrap_or(0).max(0) as u32;
        let previous = ctx.effect_outcomes.remove(&self.result);
        ctx.store_outcome(self.result, EffectOutcome::count(power as i32));
        let result = self.body.0.execute_child_with_outputs(game, ctx);
        ctx.effect_outcomes.remove(&self.result);
        if let Some(previous) = previous {
            ctx.effect_outcomes.insert(self.result, previous);
        }
        result
    }
}
