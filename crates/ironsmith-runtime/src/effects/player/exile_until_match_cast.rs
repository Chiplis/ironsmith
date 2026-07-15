//! Exile cards from the top of a library until one matches a filter, then offer
//! that card to be cast and put the rest on the bottom in random order.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

use super::runtime_helpers::{EffectDrivenCastOption, with_spell_cast_event};

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

            let candidate_name = candidate_obj.name.to_string();
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
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            if should_cast {
                let from_zone = candidate_obj.zone;
                let option = EffectDrivenCastOption {
                    object_id: candidate_id,
                    from_zone,
                    casting_method: crate::alternative_cast::CastingMethod::PlayFrom {
                        source: ctx.source,
                        zone: from_zone,
                        use_alternative: None,
                    },
                    label: format!("Cast {candidate_name}"),
                };
                let result = crate::game_loop::cast_spell_from_resolving_effect(
                    game,
                    option.object_id,
                    option.from_zone,
                    caster_id,
                    &option.casting_method,
                    self.without_paying_mana_cost,
                    None,
                    ctx.provenance,
                    &mut ctx.decision_maker,
                )
                .map_err(|error| ExecutionError::Impossible(error.to_string()))?;
                if let Some(new_id) = result {
                    casted_card = Some((candidate_id, new_id, from_zone));
                } else if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
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
