//! Prepare performer-bound keyword proposals before any original recipient mutates.
//! The caller supplies a native atomic body, never a repeated outer effect.
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError, SimultaneousEffectCommit};
use crate::events::{Event, KeywordActionEvent};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::game_state::GameState;

pub(crate) fn execute_keyword_recipient_batch<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    mut events: Vec<Event>,
    mut body: impl FnMut(&mut GameState, &mut ExecutionContext<'a>, &KeywordActionEvent)
        -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    super::execute_transaction(game, ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
            let result = (|| {
                let players=events.iter().filter_map(|event|crate::events::downcast_event::<KeywordActionEvent>(event.inner()).map(|action|action.player)).collect();
                let order=super::player_order::selected_players_from(game,players,game.turn.active_player);
                // Stable within each performer; team-aware APNAP for distinct
                // players' simultaneous replacement choices.
                events.sort_by_key(|event|crate::events::downcast_event::<KeywordActionEvent>(event.inner())
                    .and_then(|action|order.iter().position(|player|*player==action.player)).unwrap_or(usize::MAX));
                let mut prepared = Vec::new();
                for event in events {
                    let kind=crate::events::downcast_event::<KeywordActionEvent>(event.inner()).ok_or_else(||
                        ExecutionError::InternalError("keyword recipient requires an action event".into()))?.action;
                    let result = process_trait_event_with_execution_context(game, event, ctx)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                    }
                    prepared.push((kind,result));
                }
                let outcomes = super::execute_simultaneous_originals_with_default_outputs(
                    game, ctx, prepared.len() > 1, |game, ctx| {
                        let mut receipts = Vec::new();
                        for (kind,result) in prepared {
                            parent.restore_ref_preserving_resolution_control(ctx);
                            let (original, programs) = result.into_expansion();
                            let receipt = match original {
                                TraitEventResult::Replaced { effects, source, controller, context, .. } => {
                                    let snapshot = context.event.inner().snapshot().cloned();
                                    let action = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner()).ok_or_else(||
                                        ExecutionError::InternalError("keyword original lost its action".into()))?;
                                    let tags = super::keyword_action::keyword_action_object_bindings(
                                        action.object_tags.clone(), action.snapshot.as_ref());
                                    crate::effects::replacement::commit_bound_replacement_program_original_with_outputs(
                                        game, ctx,
                                        crate::events::processing::PreparedReplacementProgram {
                                            effects, source, controller, context, source_snapshot: snapshot,
                                        },
                                        crate::effects::replacement::ReplacementProgramBindings { targets: None, object_tags: tags },
                                    )?
                                }
                                TraitEventResult::Prevented => SimultaneousEffectCommit::finished(
                                    CompletedEffectOutputs::aggregate_only(EffectOutcome::prevented())),
                                TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                                    let action = crate::events::downcast_event::<KeywordActionEvent>(event.inner()).filter(|action|action.action==kind).ok_or_else(||
                                        ExecutionError::InternalError("keyword recipient lost its action".into()))?;
                                    ctx.controller = action.player;
                                    body(game, ctx, action)?
                                }
                                _ => return Err(ExecutionError::InternalError("keyword recipient preparation is incomplete".into())),
                            };
                            if ctx.decision_maker.awaiting_choice() { return Ok(Vec::new()); }
                            let receipt = crate::effects::replacement::defer_replacement_programs_with_outputs(
                                receipt, programs, |context| {
                                    let action = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner()).ok_or_else(||
                                        ExecutionError::InternalError("keyword addition lost its recipient".into()))?;
                                    Ok(crate::effects::replacement::ReplacementProgramBindings {
                                        targets: None,
                                        object_tags: super::keyword_action::keyword_action_object_bindings(
                                            action.object_tags.clone(), action.snapshot.as_ref()),
                                    })
                                });
                            receipts.push(super::with_original_execution_context(receipt, ctx));
                        }
                        parent.restore_ref_preserving_resolution_control(ctx);
                        Ok(receipts)
                    })?;
                // Each recipient's observations enter the aggregate exactly once.
                Ok(CompletedEffectOutputs::from_children(outcomes, EffectOutcome::aggregate_summing_counts))
            })();
            parent.restore_preserving_resolution_control(ctx);
            result
        })
}
