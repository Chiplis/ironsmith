//! Variable casualty support for planeswalker spells such as "Casualty X".

use crate::effect::EffectOutcome;
use crate::effects::stack::copy_spell::{
    create_stack_copy_from_object, departed_source_spell_lki, resolving_source_stack_entry,
};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::types::{CardType, Supertype};

pub type VariableCasualtyPlaneswalkerCopyEffect =
    ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect;

/// The creature sacrificed for the casualty cost while the spell was cast
/// (CR 702.153a: the sacrifice is an additional cost, 601.2f-h). Spell-cost
/// sacrifices are recorded under `sacrifice_cost_N` tags on the spell.
fn casualty_sacrificed_creature<'a>(
    game: &'a GameState,
    ctx: &'a ExecutionContext,
) -> Option<&'a ObjectSnapshot> {
    let from_tags = |tags: &'a std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>| {
        let mut keys: Vec<_> = tags
            .keys()
            .filter(|tag| tag.as_str().starts_with("sacrifice_cost_"))
            .collect();
        keys.sort_by_key(|tag| tag.as_str().to_string());
        keys.into_iter()
            .filter_map(|tag| tags.get(tag))
            .flatten()
            .find(|snapshot| snapshot.card_types.contains(&CardType::Creature))
    };
    from_tags(&ctx.tagged_objects).or_else(|| {
        game.object(ctx.source)
            .and_then(|spell| from_tags(&spell.cast_tagged_objects))
            .or_else(|| {
                game.turn_store
                    .cast_spell_lki
                    .get(&ctx.source)
                    .and_then(|lki| from_tags(&lki.0.cast_tagged_objects))
            })
    })
}

impl EffectExecutor for VariableCasualtyPlaneswalkerCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The copy's starting loyalty is the sacrificed creature's power as it
        // last existed on the battlefield.
        let Some(sacrificed) = casualty_sacrificed_creature(game, ctx) else {
            return Ok(EffectOutcome::impossible());
        };
        let loyalty = sacrificed.power.unwrap_or(0).max(0) as u32;

        let (source, original_entry) = match departed_source_spell_lki(game, ctx, ctx.source) {
            Some(lki) => lki,
            None => {
                let Some(source) = game.object(ctx.source).cloned() else {
                    return Ok(EffectOutcome::target_invalid());
                };
                (source, resolving_source_stack_entry(ctx))
            }
        };
        let copy_id = create_stack_copy_from_object(
            game,
            &source,
            ctx.source,
            &original_entry,
            ctx.controller,
            &[Supertype::Legendary],
            |copy| {
                copy.base_loyalty = Some(loyalty);
                copy.counters.remove(&CounterType::Loyalty);
            },
            None,
        )?;

        Ok(EffectOutcome::with_objects(vec![copy_id]))
    }
}
