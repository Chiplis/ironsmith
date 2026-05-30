//! Cascade keyword effect implementation.
//!
//! Exiles cards from the top of your library until a nonland card with lesser
//! mana value is exiled, lets you cast it without paying its mana cost, then
//! puts all other exiled cards on the bottom of your library in random order.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::{ManaCost, ManaSymbol};
use crate::tag::TagKey;
use crate::target::PlayerFilter;

use super::runtime_helpers::{
    cast_effect_driven_spell_without_paying, effect_driven_cast_options_for_card,
    with_spell_cast_event,
};

/// Effect that resolves a single cascade trigger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CascadeEffect;

impl CascadeEffect {
    /// Create a new cascade effect.
    pub fn new() -> Self {
        Self
    }
}

fn mana_value_on_stack(cost: Option<&ManaCost>, x_value: Option<u32>) -> u32 {
    let Some(cost) = cost else {
        return 0;
    };
    let x = x_value.unwrap_or(0);
    let x_pips = cost
        .pips()
        .iter()
        .filter(|pip| pip.iter().any(|symbol| matches!(symbol, ManaSymbol::X)))
        .count() as u32;
    cost.mana_value() + x_pips.saturating_mul(x)
}

impl EffectExecutor for CascadeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (source_mana_value, source_name) = if let Some(source_obj) = game.object(ctx.source) {
            (
                mana_value_on_stack(
                    source_obj.mana_cost.as_ref(),
                    ctx.x_value.or(source_obj.x_value),
                ),
                source_obj.name.clone(),
            )
        } else if let Some(snapshot) = ctx.source_snapshot.as_ref() {
            (
                mana_value_on_stack(
                    snapshot.mana_cost.as_ref(),
                    ctx.x_value.or(snapshot.x_value),
                ),
                snapshot.name.clone(),
            )
        } else {
            return Ok(EffectOutcome::target_invalid());
        };
        let all_tag = TagKey::from("__cascade_all");
        let match_tag = TagKey::from("__cascade_match");
        execute_library_consult(
            game,
            ctx,
            ctx.controller,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            Some(&all_tag),
            Some(&match_tag),
            |card, _| {
                if card.is_land() {
                    return false;
                }
                card.mana_cost.as_ref().map_or(0, ManaCost::mana_value) < source_mana_value
            },
        )?;

        let mut casted_card = None;
        if let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned() {
            let mut candidate_id = candidate_snapshot.object_id;
            if game.object(candidate_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(candidate_snapshot.stable_id) {
                    candidate_id = found;
                } else {
                    return Ok(EffectOutcome::count(0));
                }
            }
            let Some(candidate_obj) = game.object(candidate_id) else {
                return Ok(EffectOutcome::count(0));
            };
            let candidate_name = candidate_obj.name.clone();

            let choice_ctx = crate::decisions::context::BooleanContext::new(
                ctx.controller,
                Some(candidate_id),
                format!("Cast {candidate_name} without paying its mana cost?"),
            )
            .with_source_name(&source_name);
            let should_cast = ctx.decision_maker.decide_boolean(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            if should_cast {
                let filter = crate::target::ObjectFilter::nonland().with_mana_value(
                    crate::filter::Comparison::LessThan(source_mana_value as i32),
                );
                let options = effect_driven_cast_options_for_card(
                    game,
                    ctx.controller,
                    ctx,
                    candidate_id,
                    candidate_obj.zone,
                    &filter,
                );
                let option = if options.len() == 1 {
                    options[0].clone()
                } else if options.len() > 1 {
                    let choices = options
                        .iter()
                        .cloned()
                        .map(|option| (option.label.clone(), option))
                        .collect::<Vec<_>>();
                    let Some(choice) = crate::decisions::ask_choose_one(
                        game,
                        ctx.decision_maker,
                        ctx.controller,
                        ctx.source,
                        &choices,
                    ) else {
                        return Ok(EffectOutcome::count(0));
                    };
                    choice
                } else {
                    return Ok(EffectOutcome::count(0));
                };

                if let Some(result) =
                    cast_effect_driven_spell_without_paying(game, ctx, ctx.controller, &option)?
                {
                    casted_card = Some((candidate_id, result.new_id, result.from_zone));
                }
            }
        }
        let keep_tagged = casted_card.as_ref().map(|_| match_tag.clone());
        crate::effects::execute_effect(
            game,
            &Effect::put_tagged_remainder_on_library_bottom(
                all_tag,
                keep_tagged,
                LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
            ctx,
        )?;

        if let Some((_, casted_id, from_zone)) = casted_card {
            Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![casted_id]),
                game,
                casted_id,
                ctx.controller,
                from_zone,
                ctx.provenance,
            ))
        } else {
            Ok(EffectOutcome::count(0))
        }
    }
}
