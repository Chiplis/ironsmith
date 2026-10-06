//! Exile cards from the top of a library until one matches a filter, then
//! grant temporary play permission for that exiled card until end of turn.

use crate::effects::CompletedEffectOutputs;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryConsultMode, LibraryConsultStopRule, execute_library_consult_with_outputs,
};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilMatchGrantPlayEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub caster: PlayerFilter,
}

impl ExileUntilMatchGrantPlayEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, caster: PlayerFilter) -> Self {
        Self {
            player,
            filter,
            caster,
        }
    }
}

impl EffectExecutor for ExileUntilMatchGrantPlayEffect {
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
        let mut consultation = None;
        let instruction = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player_id = resolve_player_filter(game, &self.player, ctx)?;
                let caster_id = resolve_player_filter(game, &self.caster, ctx)?;
                let match_tag = TagKey::from("__exile_until_match_grant_play_match");
                let filter_ctx = ctx.filter_context(game);
                consultation = Some(execute_library_consult_with_outputs(
                    game,
                    ctx,
                    player_id,
                    LibraryConsultMode::Exile,
                    LibraryConsultStopRule::FirstMatch,
                    None,
                    Some(&match_tag),
                    |object, game| self.filter.matches(object, &filter_ctx, game),
                )?);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned() else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let candidate_id = candidate_snapshot.object_id;
                if !game
                    .object(candidate_id)
                    .is_some_and(|object| object.zone == Zone::Exile)
                {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                game.effect_store.grant_registry.grant_to_card(
                    candidate_id,
                    Zone::Exile,
                    caster_id,
                    Grantable::PlayFrom,
                    GrantSource::Effect {
                        source_id: ctx.source,
                        expires_end_of_turn: game.turn.turn_number,
                    },
                );

                Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::with_objects(vec![candidate_id]),
                ))
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return instruction
                .map(|_| CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
        }
        instruction.map(|outputs| {
            if let Some(consult) = consultation {
                let observations = consult.attach_to_outputs(EffectOutcome::resolved());
                let primary_status = outputs.outcome.status;
                let primary_value = outputs.outcome.value.clone();
                let mut combined = EffectOutcome::aggregate([
                    observations.outcome.clone(),
                    outputs.outcome.clone(),
                ]);
                combined.status = primary_status;
                combined.value = primary_value;
                let mut outputs = outputs.project_aggregate(combined);
                outputs.retain_batch_children([observations]);
                outputs
            } else {
                outputs
            }
        })
    }
}
