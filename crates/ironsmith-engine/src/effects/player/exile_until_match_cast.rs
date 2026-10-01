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
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let mut consultation = None;
        let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let caster_id = resolve_player_filter(game, &self.caster, ctx)?;
        let all_tag = TagKey::from("__exile_until_match_cast_all");
        let match_tag = TagKey::from("__exile_until_match_cast_match");
        let filter_ctx = ctx.filter_context(game);
        consultation = Some(execute_library_consult(
            game,
            ctx,
            player_id,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            Some(&all_tag),
            Some(&match_tag),
            |object, game| self.filter.matches(object, &filter_ctx, game),
        )?);
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }

        let mut casted_card = None;
        let mut cast_outcome = None;
        if let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned()
            && let Some(candidate_obj) = game.object(candidate_snapshot.object_id)
            && candidate_obj.zone == crate::zone::Zone::Exile
        {
            let candidate_id = candidate_snapshot.object_id;

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
                .map_err(super::runtime_helpers::effect_driven_cast_error)?;
                if let Some(new_id) = result {
                    casted_card = Some((candidate_id, new_id, from_zone));
                    cast_outcome = Some(with_spell_cast_event(
                        EffectOutcome::with_objects(vec![new_id]), game, new_id, caster_id, from_zone, ctx.provenance,
                    ));
                } else if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
            }
        }
        let keep_tagged = casted_card.as_ref().map(|_| match_tag.clone());
        let cleanup = crate::effects::execute_effect(
            game,
            &Effect::put_tagged_remainder_on_library_bottom(
                all_tag,
                keep_tagged,
                LibraryBottomOrder::Random,
                PlayerFilter::Specific(caster_id),
            ),
            ctx,
        )?;

        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let primary = cast_outcome.as_ref().map(|outcome| outcome.value.clone())
            .unwrap_or(crate::effect::OutcomeValue::Count(0));
        let mut phases = Vec::new();
        if let Some(outcome) = cast_outcome { phases.push(outcome); }
        phases.push(cleanup);
        let mut outcome = EffectOutcome::aggregate(phases);
        outcome.status = crate::effect::OutcomeStatus::Succeeded;
        outcome.value = primary;
        Ok(outcome)
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || instruction.is_err() { *game = checkpoint; context_checkpoint.restore(ctx); }
        if pending { return instruction.map(|_| EffectOutcome::count(0)); }
        instruction.map(|outcome| {
            if let Some(consult) = consultation {
                let primary_status = outcome.status;
                let primary_value = outcome.value.clone();
                let mut combined = EffectOutcome::aggregate([consult.attach_to_outcome(EffectOutcome::resolved()), outcome]);
                combined.status = primary_status;
                combined.value = primary_value;
                combined
            } else { outcome }
        })
    }
}
