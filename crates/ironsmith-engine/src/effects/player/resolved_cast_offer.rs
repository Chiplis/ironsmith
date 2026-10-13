//! Resolved cast offers: candidate discovery and hidden opening belong to callers.
use super::runtime_helpers::*;
use crate::decisions::context::BooleanContext;
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ObjectFilter;
use crate::zone::Zone;

pub(super) enum CastOfferCandidates {
    Exact {
        object: ObjectId,
        zone: Zone,
        spell_filter: ObjectFilter,
    },
    Resolved(Vec<EffectDrivenCastOption>),
}

pub(super) struct ResolvedCastOffer {
    pub player: PlayerId,
    pub candidates: CastOfferCandidates,
    pub prompt: Option<BooleanContext>,
    pub payment: EffectDrivenCastPayment,
    pub failed_cast: EffectOutcome,
}

impl ResolvedCastOffer {
    pub(super) fn execute(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let neutral = || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
        if let Some(prompt) = self.prompt {
            let accepted = ctx.decision_maker.decide_boolean(game, &prompt);
            if ctx.decision_maker.awaiting_choice() || !accepted {
                return Ok(neutral());
            }
        }
        let options = match self.candidates {
            CastOfferCandidates::Exact {
                object,
                zone,
                spell_filter,
            } => {
                if game.object(object).is_none_or(|card| card.zone != zone) {
                    return Ok(neutral());
                }
                effect_driven_cast_options_for_card_in_context(
                    game,
                    self.player,
                    ctx.source,
                    object,
                    zone,
                    &spell_filter,
                    self.payment,
                    &ctx.filter_context(game).with_caster(Some(self.player)),
                )
            }
            CastOfferCandidates::Resolved(options) => options,
        };
        let option = choose_effect_driven_cast_option(game, ctx, self.player, options);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(neutral());
        }
        let Some(option) = option else {
            return Ok(neutral());
        };
        let cast =
            cast_effect_driven_spell_with_payment(game, ctx, self.player, &option, self.payment)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(neutral());
        }
        let Some(cast) = cast else {
            return Ok(CompletedEffectOutputs::aggregate_only(self.failed_cast));
        };
        complete_effect_driven_cast_with_outputs(
            EffectOutcome::with_objects(vec![cast.new_id]),
            game,
            cast,
            self.player,
            ctx.provenance,
        )
    }
}
