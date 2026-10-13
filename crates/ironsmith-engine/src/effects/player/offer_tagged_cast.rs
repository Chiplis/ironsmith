//! Offer a free cast of one exact retained card, constraining the resulting
//! spell (including alternate faces), rather than only the original card.
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone)]
pub(crate) struct OfferTaggedCast {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub zone: Zone,
    pub spell_filter: ObjectFilter,
    pub source_name: Option<String>,
}
impl EffectExecutor for OfferTaggedCast {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        use ironsmith_core::effect_capabilities::LibraryMovementPolicy;
        if self.zone == Zone::Library {
            LibraryMovementPolicy::Moves
        } else {
            LibraryMovementPolicy::Leaf
        }
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player =
                    crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
                let Some(card) = ctx
                    .get_tagged(&self.tag)
                    .and_then(|snapshot| game.object(snapshot.object_id))
                    .filter(|card| card.zone == self.zone)
                else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let id = card.id;
                let mut prompt = crate::decisions::context::BooleanContext::new(
                    player,
                    Some(id),
                    format!("Cast {} without paying its mana cost?", card.name),
                );
                if let Some(name) = &self.source_name {
                    prompt = prompt.with_source_name(name);
                }
                let accepted = ctx.decision_maker.decide_boolean(game, &prompt);
                if ctx.decision_maker.awaiting_choice() || !accepted {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let options = super::runtime_helpers::effect_driven_cast_options_for_card(
                    game,
                    player,
                    ctx.source,
                    id,
                    self.zone,
                    &self.spell_filter,
                );
                let option = super::runtime_helpers::choose_effect_driven_cast_option(
                    game, ctx, player, options,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let Some(option) = option else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let Some(cast) = super::runtime_helpers::cast_effect_driven_spell_without_paying(
                    game, ctx, player, &option,
                )?
                else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                super::runtime_helpers::complete_effect_driven_cast_with_outputs(
                    EffectOutcome::with_objects(vec![cast.new_id]),
                    game,
                    cast,
                    player,
                    ctx.provenance,
                )
            },
        )
    }
}
