//! Captured object choices and bounded repetition with caller-owned policies.
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::{effect::EffectOutcome, game_state::GameState, ids::ObjectId};

pub(crate) fn choose_one(
    game: &GameState,
    ctx: &mut ExecutionContext,
    candidates: &[ObjectId],
    prompt: &str,
    strict: bool,
) -> Result<Option<ObjectId>, ExecutionError> {
    choose_one_for_player(game, ctx, ctx.controller, candidates, prompt, strict)
}

pub(crate) fn choose_one_for_player(
    game: &GameState,
    ctx: &mut ExecutionContext,
    chooser: crate::ids::PlayerId,
    candidates: &[ObjectId],
    prompt: &str,
    strict: bool,
) -> Result<Option<ObjectId>, ExecutionError> {
    choose_captured_object(
        game,
        ctx,
        chooser,
        candidates,
        prompt,
        CapturedChoicePolicy {
            skip_singleton: true,
            prompt_empty: false,
            transport: ObjectChoiceTransport::DecisionSpec,
            answer: if strict {
                ObjectAnswerPolicy::Strict
            } else {
                ObjectAnswerPolicy::FirstValidOrDefault
            },
        },
    )
}

#[derive(Clone, Copy)]
pub(crate) enum ObjectAnswerPolicy {
    Strict,
    First,
    FirstValidOrDefault,
    LastOrDefault,
}
#[derive(Clone, Copy)]
pub(crate) enum ObjectChoiceTransport {
    DecisionSpec,
    Direct { explicit: bool },
}
#[derive(Clone, Copy)]
pub(crate) struct CapturedChoicePolicy {
    pub skip_singleton: bool,
    pub prompt_empty: bool,
    pub transport: ObjectChoiceTransport,
    pub answer: ObjectAnswerPolicy,
}
impl CapturedChoicePolicy {
    pub const EXPLICIT_FIRST: Self = Self {
        skip_singleton: false,
        prompt_empty: false,
        transport: ObjectChoiceTransport::Direct { explicit: true },
        answer: ObjectAnswerPolicy::First,
    };
}
pub(crate) fn select_captured_answer(
    candidates: &[ObjectId],
    selected: &[ObjectId],
    policy: ObjectAnswerPolicy,
) -> Result<Option<ObjectId>, ExecutionError> {
    let valid = |id: &&ObjectId| candidates.contains(id);
    Ok(match policy {
        ObjectAnswerPolicy::Strict => {
            if selected.len() != 1 || !candidates.contains(&selected[0]) {
                return Err(ExecutionError::InvalidTarget);
            }
            selected.first().copied()
        }
        ObjectAnswerPolicy::First => selected.first().filter(valid).copied(),
        ObjectAnswerPolicy::FirstValidOrDefault => {
            selected.iter().find(valid).or(candidates.first()).copied()
        }
        ObjectAnswerPolicy::LastOrDefault => selected
            .last()
            .filter(valid)
            .or(candidates.first())
            .copied(),
    })
}

pub(crate) fn choose_captured_object(
    game: &GameState,
    ctx: &mut ExecutionContext,
    chooser: crate::ids::PlayerId,
    candidates: &[ObjectId],
    prompt: &str,
    policy: CapturedChoicePolicy,
) -> Result<Option<ObjectId>, ExecutionError> {
    if candidates.is_empty() && !policy.prompt_empty {
        return Ok(None);
    }
    if policy.skip_singleton && candidates.len() == 1 {
        return Ok(Some(candidates[0]));
    }
    let selected = match policy.transport {
        ObjectChoiceTransport::DecisionSpec => crate::decisions::make_decision(
            game,
            ctx.decision_maker,
            chooser,
            Some(ctx.source),
            crate::decisions::specs::ChooseObjectsSpec::new(
                ctx.source,
                prompt,
                candidates.to_vec(),
                1,
                Some(1),
            ),
        ),
        ObjectChoiceTransport::Direct { explicit } => {
            let mut menu = crate::decisions::context::SelectObjectsContext::new(
                chooser,
                Some(ctx.source),
                prompt,
                candidates
                    .iter()
                    .filter_map(|id| {
                        game.object(*id).map(|object| {
                            crate::decisions::context::SelectableObject::new(
                                *id,
                                object.name.clone(),
                            )
                        })
                    })
                    .collect(),
                1,
                Some(1),
            );
            if explicit {
                menu = menu.require_explicit_choice();
            }
            ctx.decision_maker.decide_objects(game, &menu)
        }
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    select_captured_answer(candidates, &selected, policy.answer)
}

/// Freeze one checked characteristic frame, select its eligible minimum ties,
/// then bind the exact selected incarnation for the caller's ordinary effect.
pub(crate) fn least_characteristic_candidates(
    game: &GameState,
    ids: &[ObjectId],
    metric: impl Fn(
        &crate::continuous::CalculatedCharacteristics,
    ) -> Result<Option<i32>, ExecutionError>,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let frame = game
        .try_current_characteristics_batch(ids)
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let values = ids
        .iter()
        .map(|id| {
            let chars = frame.get(id).ok_or_else(|| {
                ExecutionError::IncompleteEvidence(format!(
                    "selection has no current characteristics for battlefield object {id:?}"
                ))
            })?;
            Ok(metric(chars)?.map(|value| (*id, value)))
        })
        .collect::<Result<Vec<_>, ExecutionError>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(
        retain_extrema_by_key(values, |(_, value)| Some(*value), false)
            .into_iter()
            .map(|(id, _)| id)
            .collect(),
    )
}

/// Re-evaluate each child selection after the previous child has completed.
/// Reservation belongs to the resource owner, before this loop begins.
pub(crate) fn repeat_collect_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    count: usize,
    mut body: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let mut objects = Vec::new();
    let mut children = Vec::new();
    for _ in 0..count {
        let child = body(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        objects.extend(child.outcome.instruction_result().output_objects());
        children.push(child);
        if ctx.resolution_stopped() {
            break;
        }
    }
    Ok(CompletedEffectOutputs::with_primary_result(
        EffectOutcome::with_objects(objects),
        children,
    ))
}

/// Keep all extrema ties in input order. Missing metrics never win, including
/// an entirely missing round. The metric owns association and aggregation;
/// this reducer never re-reads live objects or mutates tags.
pub(crate) fn retain_extrema_by_key<T, K: Ord + Clone>(
    items: impl IntoIterator<Item = T>,
    mut metric: impl FnMut(&T) -> Option<K>,
    greatest: bool,
) -> Vec<T> {
    let values = items
        .into_iter()
        .filter_map(|item| metric(&item).map(|key| (item, key)))
        .collect::<Vec<_>>();
    let extremum = if greatest {
        values.iter().map(|(_, key)| key).max()
    } else {
        values.iter().map(|(_, key)| key).min()
    }
    .cloned();
    values
        .into_iter()
        .filter_map(|(item, key)| (Some(key) == extremum).then_some(item))
        .collect()
}
