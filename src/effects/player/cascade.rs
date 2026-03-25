//! Cascade keyword effect implementation.
//!
//! Exiles cards from the top of your library until a nonland card with lesser
//! mana value is exiled, lets you cast it without paying its mana cost, then
//! puts all other exiled cards on the bottom of your library in random order.

use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::mana::{ManaCost, ManaSymbol};
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::runtime_helpers::with_spell_cast_event;

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

            if should_cast {
                let from_zone = candidate_obj.zone;
                let mana_cost = candidate_obj.mana_cost.clone();
                let stable_id = candidate_obj.stable_id;
                let x_value = mana_cost
                    .as_ref()
                    .and_then(|cost| if cost.has_x() { Some(0u32) } else { None });

                if let Some(new_id) = game.move_object_by_effect(candidate_id, Zone::Stack) {
                    if let Some(obj) = game.object_mut(new_id) {
                        obj.x_value = x_value;
                    }

                    let stack_entry = StackEntry {
                        object_id: new_id,
                        controller: ctx.controller,
                        provenance: ctx.provenance,
                        targets: vec![],
                        target_assignments: vec![],
                        x_value,
                        ability_effects: None,
                        is_ability: false,
                        casting_method: CastingMethod::PlayFrom {
                            source: ctx.source,
                            zone: from_zone,
                            use_alternative: None,
                        },
                        optional_costs_paid: OptionalCostsPaid::default(),
                        defending_player: None,
                        chosen_player: None,
                        saga_final_chapter_source: None,
                        source_stable_id: Some(stable_id),
                        source_snapshot: None,
                        source_name: Some(candidate_name),
                        triggering_event: None,
                        intervening_if: None,
                        keyword_payment_contributions: vec![],
                        crew_contributors: vec![],
                        saddle_contributors: vec![],
                        chosen_modes: None,
                        tagged_objects: std::collections::HashMap::new(),
                    };

                    game.push_to_stack(stack_entry);
                    casted_card = Some((candidate_id, new_id, from_zone));
                }
            }
        }
        let keep_tagged = casted_card.as_ref().map(|_| match_tag.clone());
        crate::executor::execute_effect(
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
