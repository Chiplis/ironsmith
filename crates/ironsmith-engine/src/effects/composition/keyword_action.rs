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

/// Preserve explicit subjects, including an explicitly empty subject, and
/// populate the two internal aliases consistently. A source snapshot is only
/// a fallback for actions whose performer/source is also their subject.
pub(super) fn keyword_action_object_bindings(
    mut tags: std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    fallback: Option<&crate::snapshot::ObjectSnapshot>,
) -> Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)> {
    let subject = tags
        .get("__it__")
        .or_else(|| tags.get("it"))
        .cloned()
        .or_else(|| fallback.map(|snapshot| vec![snapshot.clone()]));
    if let Some(subject) = subject {
        tags.entry("it".into()).or_insert_with(|| subject.clone());
        tags.entry("__it__".into()).or_insert(subject);
    }
    tags.into_iter()
        .map(|(name, objects)| (name.as_str().to_owned(), objects))
        .collect()
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
    execute_keyword_action_with_outputs(game, ctx, event, output, amount, |game, ctx, action| {
        body(game, ctx, action).map(crate::effects::CompletedEffectOutputs::aggregate_only)
    })
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Retain the body owner's projections through keyword replacement expansion.
pub(crate) fn execute_keyword_action_with_outputs<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    event: Event,
    output: KeywordActionOutput,
    amount: KeywordActionAmount,
    mut body: impl FnMut(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &KeywordActionEvent,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    super::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let kind = crate::events::downcast_event::<KeywordActionEvent>(event.inner())
                .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "keyword envelope requires an action event".into(),
                    )
                })?
                .action;
            let processed = process_trait_event_with_execution_context(game, event, ctx)?;
            crate::effects::replacement::execute_event_expansion_with_outputs(
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
                        let outcome =
                            super::mechanic_actions::execute_keyword_action_replacement_effects_with_outputs(
                                game, ctx, effects, source, controller, &context, snapshot,
                            )?;
                        // A replacement program is observable work, but its keyword
                        // completions are not original executions of this proposal.
                        let mut original = EffectOutcome::replaced();
                        original.set_value(if matches!(output, KeywordActionOutput::Objects) {
                            OutcomeValue::Objects(Vec::new())
                        } else {
                            OutcomeValue::Count(0)
                        });
                        let aggregate = EffectOutcome::aggregate_replacement_outcomes(
                            original,
                            [outcome.outcome.clone()],
                        );
                        Ok(outcome.project_aggregate(aggregate))
                    }
                    TraitEventResult::Prevented => {
                        let mut outcome = EffectOutcome::prevented();
                        outcome.set_value(OutcomeValue::Count(0));
                        Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            outcome,
                        ))
                    }
                    TraitEventResult::NeedsChoice { .. }
                    | TraitEventResult::NeedsInteraction { .. } => {
                        if ctx.decision_maker.awaiting_choice() {
                            Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ))
                        } else {
                            Err(ExecutionError::InternalError(
                                "keyword action suspended without a decision".into(),
                            ))
                        }
                    }
                    TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                        let action =
                            crate::events::downcast_event::<KeywordActionEvent>(event.inner())
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
                        let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        );
                        for _ in 0..repetitions {
                            let mut unit = action.clone();
                            if matches!(amount, KeywordActionAmount::Repetitions) {
                                unit.amount = 1;
                            }
                            let controller = ctx.controller;
                            ctx.controller = action.player;
                            let result = body(game, ctx, &unit);
                            ctx.controller = controller;
                            let child = result?;
                            children.push(child.outcome.clone());
                            outputs.retain_owned_child(child);
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::count(0),
                                ));
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
                            Ok(outputs.project_aggregate(
                                EffectOutcome::aggregate_with_primary_result(
                                    EffectOutcome::with_objects(objects),
                                    children,
                                ),
                            ))
                        } else {
                            Ok(
                                outputs.project_aggregate(EffectOutcome::aggregate_summing_counts(
                                    children,
                                )),
                            )
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
                        .instruction_result()
                        .events
                        .iter()
                        .rev()
                        .filter_map(|event| event.downcast::<KeywordActionEvent>())
                        .find(|action| action.action == kind && action.source == captured.source)
                        .unwrap_or(captured);
                    let mut tags = captured.object_tags.clone();
                    tags.extend(action.object_tags.clone());
                    let object_tags = keyword_action_object_bindings(
                        tags,
                        action.snapshot.as_ref().or(captured.snapshot.as_ref()),
                    );
                    Ok(crate::effects::replacement::ReplacementProgramBindings {
                        targets: None,
                        object_tags,
                    })
                },
            )
        },
    )
}
