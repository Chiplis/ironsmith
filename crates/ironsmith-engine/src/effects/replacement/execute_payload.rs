//! Execute an instead-payload without losing its event or replacement history.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::processing::ReplacementEventContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

pub(crate) fn execute_replacement_payload(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    targets: Option<Vec<crate::effects::ResolvedTarget>>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_replacement_payload_with_snapshot(
        game, parent, effects, source, controller, context, targets, None, Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_replacement_payload_with_object_tags(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    targets: Option<Vec<crate::effects::ResolvedTarget>>,
    object_tags: Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_replacement_payload_with_snapshot(
        game, parent, effects, source, controller, context, targets, None, object_tags,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_replacement_payload_with_snapshot(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    targets: Option<Vec<crate::effects::ResolvedTarget>>,
    captured_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    object_tags: Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)>,
) -> Result<EffectOutcome, ExecutionError> {
    let affected_player = context.affected_player;
    let inherited_replacements = parent.replacement.clone();
    let source_snapshot = captured_source_snapshot.or_else(|| game
        .object(source)
        .filter(|_| !game.is_phased_out(source))
        .map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .or_else(|| {
            game.turn_store
                .turn_history
                .departed_object_snapshot(source)
                .cloned()
        })
        .or_else(|| {
            parent
                .source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == source)
                .cloned()
        }));
    // A replacement has its own source/controller and program scope. Inherit
    // the event history, not the interrupted instruction's local outcomes.
    let mut child = ExecutionContext::new(source, controller, &mut *parent.decision_maker);
    child.source_snapshot = source_snapshot;
    child.replacement = inherited_replacements;
    child.iteration.iterated_player = Some(affected_player);
    child.targets =
        targets.unwrap_or_else(|| vec![crate::effects::ResolvedTarget::Player(affected_player)]);
    for (name, snapshots) in object_tags {
        child.set_tagged_objects(name.as_str(), snapshots);
    }
    context.apply_to(&mut child);
    let mut outcomes = Vec::new();
    for effect in effects {
        outcomes.push(execute_effect(game, effect, &mut child)?);
        if child.decision_maker.awaiting_choice() {
            break;
        }
    }
    Ok(EffectOutcome::aggregate(outcomes))
}


/// Explicit bindings for one captured replacement program. Each child scope
/// receives its own bindings; the interrupted instruction's tags are untouched.
pub(crate) struct ReplacementProgramBindings {
    pub targets: Option<Vec<crate::effects::ResolvedTarget>>,
    pub object_tags: Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)>,
}

/// Commit the retained original proposal, then execute the appended programs.
/// Replacement selection has finished before either phase executes. This
/// composition uses the existing payload executor and keeps primary quantities
/// separate from added actions while retaining all observations and facts.
/// The owning operation must also checkpoint before replacement selection so
/// failure or pending input restores consumed shields and prior preparations.
pub(crate) fn execute_event_expansion<'a, F>(
    game: &mut GameState,
    parent: &mut ExecutionContext<'a>,
    result: crate::events::processing::TraitEventResult,
    commit_original: F,
) -> Result<EffectOutcome, ExecutionError>
where
    F: FnOnce(&mut GameState, &mut ExecutionContext<'a>,
        crate::events::processing::TraitEventResult) -> Result<EffectOutcome, ExecutionError>,
{
    execute_event_expansion_with_targets(game, parent, result, commit_original, |_, _, _| Ok(None))
}

/// Bind appended programs to targets from each captured event before execution.
pub(crate) fn execute_event_expansion_with_targets<'a, F, T>(
    game: &mut GameState, parent: &mut ExecutionContext<'a>,
    result: crate::events::processing::TraitEventResult, commit_original: F, targets_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where
    F: FnOnce(&mut GameState, &mut ExecutionContext<'a>, crate::events::processing::TraitEventResult) -> Result<EffectOutcome, ExecutionError>,
    T: Fn(&GameState, &ReplacementEventContext, &EffectOutcome) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, ExecutionError>,
{
    execute_event_expansion_with_bindings(game, parent, result, commit_original, |game, context, receipt| {
        Ok(ReplacementProgramBindings { targets: targets_for_program(game, context, receipt)?, object_tags: Vec::new() })
    })
}

pub(crate) fn execute_event_expansion_with_bindings<'a, F, T>(
    game: &mut GameState,
    parent: &mut ExecutionContext<'a>,
    result: crate::events::processing::TraitEventResult,
    commit_original: F,
    bindings_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where
    F: FnOnce(&mut GameState, &mut ExecutionContext<'a>,
        crate::events::processing::TraitEventResult) -> Result<EffectOutcome, ExecutionError>,
    T: Fn(&GameState, &ReplacementEventContext, &EffectOutcome) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    let game_checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(parent);
    let (original, programs) = result.into_expansion();
    let result = (|| {
        let original_outcome = commit_original(game, parent, original)?;
        if parent.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        execute_deferred_replacement_programs_with_bindings(
            game, parent, original_outcome, programs, bindings_for_program,
        )
    })();
    if result.is_err() || parent.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(game_checkpoint, result.is_ok() && parent.decision_maker.awaiting_choice());
        context_checkpoint.restore(parent);
    }
    result
}

/// Append captured programs after an already completed original operation.
/// The owner must checkpoint before selecting replacements and committing the
/// original, because this helper's checkpoint starts at the deferred phase.
pub(crate) fn execute_deferred_replacement_programs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_deferred_replacement_programs_with_targets(game, parent, original_outcome, programs, |_, _, _| Ok(None))
}

pub(crate) fn execute_deferred_replacement_programs_with_targets<T>(
    game: &mut GameState, parent: &mut ExecutionContext, original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>, targets_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where T: Fn(&GameState, &ReplacementEventContext, &EffectOutcome) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, ExecutionError>,
{
    execute_deferred_replacement_programs_with_bindings(game, parent, original_outcome, programs, |game, context, receipt| {
        Ok(ReplacementProgramBindings { targets: targets_for_program(game, context, receipt)?, object_tags: Vec::new() })
    })
}

pub(crate) fn execute_deferred_replacement_programs_with_bindings<T>(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    bindings_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where T: Fn(&GameState, &ReplacementEventContext, &EffectOutcome) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    let game_checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(parent);
    let result = (|| {
        if parent.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let mut outcomes = Vec::new();
        for program in programs {
            let bindings = bindings_for_program(game, &program.context, &original_outcome)?;
            let outcome = execute_replacement_payload_with_snapshot(
                game, parent, &program.effects, program.source, program.controller,
                &program.context, bindings.targets, program.source_snapshot, bindings.object_tags,
            )?;
            if parent.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            outcomes.push(outcome);
        }
        Ok(EffectOutcome::aggregate_replacement_outcomes(original_outcome, outcomes))
    })();
    if result.is_err() || parent.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(game_checkpoint, result.is_ok() && parent.decision_maker.awaiting_choice());
        context_checkpoint.restore(parent);
    }
    result
}
