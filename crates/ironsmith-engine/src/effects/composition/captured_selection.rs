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
    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() == 1 {
        return Ok(Some(candidates[0]));
    }
    let spec = crate::decisions::specs::ChooseObjectsSpec::new(
        ctx.source,
        prompt,
        candidates.to_vec(),
        1,
        Some(1),
    );
    let selected: Vec<ObjectId> = crate::decisions::make_decision(
        game,
        ctx.decision_maker,
        chooser,
        Some(ctx.source),
        spec,
    );
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    if strict && (selected.len() != 1 || !candidates.contains(&selected[0])) {
        return Err(ExecutionError::InvalidTarget);
    }
    Ok(
        crate::effects::helpers::normalize_object_selection(selected, candidates, 1)
            .first()
            .copied()
            .or(Some(candidates[0])),
    )
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
    let least = values.iter().map(|(_, value)| *value).min();
    Ok(values
        .into_iter()
        .filter_map(|(id, value)| (Some(value) == least).then_some(id))
        .collect())
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
