//! Replacement envelope for composed keyword action programs.

use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, KeywordActionEvent};
use crate::game_state::GameState;

#[derive(Clone, Copy)]
pub(crate) enum KeywordActionOutput {
    Body,
    Objects,
}

#[derive(Clone, Copy)]
pub(crate) enum KeywordActionAmount {
    /// One body receives the full magnitude, such as connive N.
    BodyMagnitude,
    /// The event requests N distinct executions of a unit action.
    Repetitions,
}

/// Process the action once, execute its body only when the original proceeds,
/// and bind deferred programs to the completed action's immutable observations.
/// Completion emission stays in the body, at its correct instruction boundary.
pub(crate) fn execute_keyword_action<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    event: Event,
    output: KeywordActionOutput,
    amount: KeywordActionAmount,
    mut body: impl FnMut(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &KeywordActionEvent,
    ) -> Result<EffectOutcome, ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    let kind = crate::events::downcast_event::<KeywordActionEvent>(event.inner())
        .ok_or_else(|| {
            ExecutionError::InternalError("keyword envelope requires an action event".into())
        })?
        .action;
    let processed = process_trait_event_with_execution_context(game, event, ctx)?;
    crate::effects::replacement::execute_event_expansion_with_bindings(
        game,
        ctx,
        processed,
        |game, ctx, original| match original {
            TraitEventResult::Replaced {
                effects,
                source,
                controller,
                context,
                ..
            } => {
                let snapshot = context.event.inner().snapshot().cloned();
                let mut outcome =
                    super::mechanic_actions::execute_keyword_action_replacement_effects(
                        game, ctx, effects, source, controller, &context, snapshot,
                    )?;
                if matches!(output, KeywordActionOutput::Objects) {
                    let objects = outcome
                        .events
                        .iter()
                        .filter_map(|event| event.downcast::<KeywordActionEvent>())
                        .filter(|action| action.action == kind)
                        .map(|action| action.source)
                        .collect();
                    outcome.set_value(OutcomeValue::Objects(objects));
                }
                Ok(outcome)
            }
            TraitEventResult::Prevented => {
                let mut outcome = EffectOutcome::prevented();
                outcome.set_value(OutcomeValue::Count(0));
                Ok(outcome)
            }
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                if ctx.decision_maker.awaiting_choice() {
                    Ok(EffectOutcome::count(0))
                } else {
                    Err(ExecutionError::InternalError(
                        "keyword action suspended without a decision".into(),
                    ))
                }
            }
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                let action = crate::events::downcast_event::<KeywordActionEvent>(event.inner())
                    .filter(|action| action.action == kind)
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "keyword replacement changed action kind".into(),
                        )
                    })?;
                if game.player(action.player).is_none() {
                    return Err(ExecutionError::PlayerNotFound(action.player));
                }
                let repetitions = match amount {
                    KeywordActionAmount::BodyMagnitude => 1,
                    KeywordActionAmount::Repetitions => action.amount,
                };
                let mut children = Vec::new();
                for _ in 0..repetitions {
                    let mut unit = action.clone();
                    if matches!(amount, KeywordActionAmount::Repetitions) {
                        unit.amount = 1;
                    }
                    let controller = ctx.controller;
                    ctx.controller = action.player;
                    let result = body(game, ctx, &unit);
                    ctx.controller = controller;
                    children.push(result?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                }
                if matches!(output, KeywordActionOutput::Objects) {
                    let objects = children
                        .iter()
                        .flat_map(|child| {
                            child
                                .instruction_result()
                                .objects()
                                .unwrap_or(&[])
                                .iter()
                                .copied()
                        })
                        .collect();
                    Ok(EffectOutcome::aggregate_with_primary_result(
                        EffectOutcome::with_objects(objects),
                        children,
                    ))
                } else {
                    Ok(EffectOutcome::aggregate_summing_counts(children))
                }
            }
            TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError(
                "keyword envelope received an unflattened expansion".into(),
            )),
        },
        |_, context, receipt| {
            let captured =
                crate::events::downcast_event::<KeywordActionEvent>(context.event.inner())
                    .filter(|action| action.action == kind)
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "keyword addition lost its action event".into(),
                        )
                    })?;
            let action = receipt
                .events
                .iter()
                .rev()
                .filter_map(|event| event.downcast::<KeywordActionEvent>())
                .find(|action| action.action == kind && action.source == captured.source)
                .unwrap_or(captured);
            let mut tags = action.object_tags.clone();
            if let Some(snapshot) = action.snapshot.as_ref().or(captured.snapshot.as_ref()) {
                tags.insert("it".into(), vec![snapshot.clone()]);
                tags.insert("__it__".into(), vec![snapshot.clone()]);
            }
            Ok(crate::effects::replacement::ReplacementProgramBindings {
                targets: None,
                object_tags: tags
                    .into_iter()
                    .map(|(name, objects)| (name.as_str().to_owned(), objects))
                    .collect(),
            })
        },
    )
}
