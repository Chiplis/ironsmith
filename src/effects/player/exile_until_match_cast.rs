//! Exile cards from the top of a library until one matches a filter, then offer
//! that card to be cast and put the rest on the bottom in random order.

use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

use super::runtime_helpers::with_spell_cast_event;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilMatchCastEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub caster: PlayerFilter,
    pub without_paying_mana_cost: bool,
}

impl ExileUntilMatchCastEffect {
    pub fn new(
        player: PlayerFilter,
        filter: ObjectFilter,
        caster: PlayerFilter,
        without_paying_mana_cost: bool,
    ) -> Self {
        Self {
            player,
            filter,
            caster,
            without_paying_mana_cost,
        }
    }
}

impl EffectExecutor for ExileUntilMatchCastEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let caster_id = resolve_player_filter(game, &self.caster, ctx)?;
        let all_tag = TagKey::from("__exile_until_match_cast_all");
        let match_tag = TagKey::from("__exile_until_match_cast_match");
        let filter_ctx = ctx.filter_context(game);
        execute_library_consult(
            game,
            ctx,
            player_id,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            Some(&all_tag),
            Some(&match_tag),
            |object, game| self.filter.matches(object, &filter_ctx, game),
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
            let prompt = if self.without_paying_mana_cost {
                format!("Cast {candidate_name} without paying its mana cost?")
            } else {
                format!("Cast {candidate_name}?")
            };
            let choice_ctx = crate::decisions::context::BooleanContext::new(
                caster_id,
                Some(candidate_id),
                prompt,
            );
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
                        controller: caster_id,
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
                PlayerFilter::Specific(caster_id),
            ),
            ctx,
        )?;

        if let Some((_, casted_id, from_zone)) = casted_card {
            Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![casted_id]),
                game,
                casted_id,
                caster_id,
                from_zone,
                ctx.provenance,
            ))
        } else {
            Ok(EffectOutcome::count(0))
        }
    }
}
