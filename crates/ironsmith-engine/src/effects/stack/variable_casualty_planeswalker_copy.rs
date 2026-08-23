//! Variable casualty support for planeswalker spells such as "Casualty X".

use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::stack::copy_spell::{create_stack_copy, resolving_source_stack_entry};
use crate::effects::zones::SacrificeEffect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::Comparison;
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::types::Supertype;

pub type VariableCasualtyPlaneswalkerCopyEffect =
    ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect;

impl EffectExecutor for VariableCasualtyPlaneswalkerCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut filter = ObjectFilter::creature();
        filter.power = Some(Comparison::GreaterThanOrEqual(0));

        let sacrifice = SacrificeEffect::you(filter, 1);
        let sacrifice_outcome = sacrifice.execute(game, ctx)?;
        let Some(sacrificed) = sacrifice_outcome
            .affected_object_memory()
            .and_then(|memory| memory.first())
        else {
            return Ok(sacrifice_outcome);
        };
        let loyalty = sacrificed_power(sacrificed);

        let original_entry = resolving_source_stack_entry(ctx);
        let copy_id = create_stack_copy(
            game,
            ctx.source,
            &original_entry,
            ctx.controller,
            &[Supertype::Legendary],
            None,
        )?;

        if let Some(copy) = game.object_mut(copy_id) {
            copy.base_loyalty = Some(loyalty);
            copy.counters.remove(&CounterType::Loyalty);
        }

        Ok(EffectOutcome::aggregate_summing_counts([
            sacrifice_outcome,
            EffectOutcome::with_objects(vec![copy_id]),
        ]))
    }
}

fn sacrificed_power(memory: &OutcomeObjectMemory) -> u32 {
    memory.power.unwrap_or(0).max(0) as u32
}
