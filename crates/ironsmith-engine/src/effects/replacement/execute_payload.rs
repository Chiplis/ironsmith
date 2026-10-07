//! Execute an instead-payload without losing its event or replacement history.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect_with_outputs};
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
        game,
        parent,
        effects,
        source,
        controller,
        context,
        targets,
        None,
        Vec::new(),
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
        game,
        parent,
        effects,
        source,
        controller,
        context,
        targets,
        None,
        object_tags,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_replacement_payload_with_snapshot(
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
    execute_replacement_payload_with_outputs(
        game,
        parent,
        effects,
        source,
        controller,
        context,
        targets,
        captured_source_snapshot,
        object_tags,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_replacement_payload_with_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    targets: Option<Vec<crate::effects::ResolvedTarget>>,
    captured_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    object_tags: Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    with_replacement_child(
        game,
        parent,
        source,
        controller,
        context,
        targets,
        captured_source_snapshot,
        object_tags,
        |game, child| execute_replacement_program_with_outputs(game, child, effects),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn with_replacement_child<R>(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    targets: Option<Vec<crate::effects::ResolvedTarget>>,
    captured_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    object_tags: Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)>,
    run: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<R, ExecutionError>,
) -> Result<R, ExecutionError> {
    let affected_player = context.affected_player;
    let inherited_replacements = parent.replacement.clone();
    let source_snapshot = captured_source_snapshot.or_else(|| {
        game.object(source)
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
            })
    });
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
    run(game, &mut child)
}

pub(super) fn execute_replacement_program(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<EffectOutcome, ExecutionError> {
    execute_replacement_program_with_outputs(game, child, effects)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// One replacement child loop owns execution order and trigger qualification.
pub(super) fn execute_replacement_program_with_outputs(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
        let mut outcomes: Vec<EffectOutcome> = Vec::new();
        let mut outputs =
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved());
        for (index, effect) in effects.iter().enumerate() {
            if child.resolution_stopped() { break; }
            let result = execute_effect_with_outputs(game, effect, child)?;
            outcomes.push(result.outcome.clone());
            outputs.retain_owned_child(result);
            if child.decision_maker.awaiting_choice() {
                break;
            }
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                child,
                effects.get(index + 1),
                outcomes
                    .iter_mut()
                    .flat_map(|outcome| outcome.events.iter_mut()),
            )?;
        }
        Ok(outputs.project_aggregate(EffectOutcome::aggregate(outcomes)))
    })
}

/// Explicit bindings for one captured replacement program. Each child scope
/// receives its own bindings; the interrupted instruction's tags are untouched.
#[derive(Clone)]
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
    F: FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        crate::events::processing::TraitEventResult,
    ) -> Result<EffectOutcome, ExecutionError>,
{
    execute_event_expansion_with_targets(game, parent, result, commit_original, |_, _, _| Ok(None))
}

/// Bind appended programs to targets from each captured event before execution.
pub(crate) fn execute_event_expansion_with_targets<'a, F, T>(
    game: &mut GameState,
    parent: &mut ExecutionContext<'a>,
    result: crate::events::processing::TraitEventResult,
    commit_original: F,
    targets_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where
    F: FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        crate::events::processing::TraitEventResult,
    ) -> Result<EffectOutcome, ExecutionError>,
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, ExecutionError>,
{
    execute_event_expansion_with_bindings(
        game,
        parent,
        result,
        commit_original,
        |game, context, receipt| {
            Ok(ReplacementProgramBindings {
                targets: targets_for_program(game, context, receipt)?,
                object_tags: Vec::new(),
            })
        },
    )
}

pub(crate) fn execute_event_expansion_with_bindings<'a, F, T>(
    game: &mut GameState,
    parent: &mut ExecutionContext<'a>,
    result: crate::events::processing::TraitEventResult,
    commit_original: F,
    bindings_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where
    F: FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        crate::events::processing::TraitEventResult,
    ) -> Result<EffectOutcome, ExecutionError>,
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    execute_event_expansion_with_outputs(
        game,
        parent,
        result,
        |game, parent, original| {
            commit_original(game, parent, original)
                .map(crate::effects::CompletedEffectOutputs::aggregate_only)
        },
        bindings_for_program,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Original and appended receipts pass through one expansion rollback owner.
pub(crate) fn execute_event_expansion_with_outputs<'a, F, T>(
    game: &mut GameState,
    parent: &mut ExecutionContext<'a>,
    result: crate::events::processing::TraitEventResult,
    commit_original: F,
    bindings_for_program: T,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>
where
    F: FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        crate::events::processing::TraitEventResult,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    let game_checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(parent);
    let (original, programs) = result.into_expansion();
    let result = (|| {
        let original_outcome = commit_original(game, parent, original)?;
        if parent.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let completed = complete_deferred_replacement_programs_with_bindings(
            game,
            parent,
            original_outcome.outcome.clone(),
            programs,
            bindings_for_program,
        )?;
        if parent.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        Ok(original_outcome.append_batch_program_outputs(completed))
    })();
    if result.is_err() || parent.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            game_checkpoint,
            result.is_ok() && parent.decision_maker.awaiting_choice(),
        );
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
    complete_deferred_replacement_programs(game, parent, original_outcome, programs)
        .map(CompletedReplacementPrograms::into_outcome)
}

pub(crate) fn complete_deferred_replacement_programs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
) -> Result<CompletedReplacementPrograms, ExecutionError> {
    complete_deferred_replacement_programs_with_targets(
        game,
        parent,
        original_outcome,
        programs,
        |_, _, _| Ok(None),
    )
}

pub(crate) fn complete_deferred_replacement_programs_with_targets<T>(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    targets_for_program: T,
) -> Result<CompletedReplacementPrograms, ExecutionError>
where
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, ExecutionError>,
{
    complete_deferred_replacement_programs_with_bindings(
        game,
        parent,
        original_outcome,
        programs,
        |game, context, receipt| {
            Ok(ReplacementProgramBindings {
                targets: targets_for_program(game, context, receipt)?,
                object_tags: Vec::new(),
            })
        },
    )
}

/// Completed programs retain one outcome per input program in execution order.
/// Keeping these receipts separate lets action owners retain participant identity
/// before choosing the enclosing instruction's result projection.
pub(crate) struct CompletedReplacementPrograms {
    original: EffectOutcome,
    outcomes: Vec<crate::effects::CompletedEffectOutputs>,
}
impl CompletedReplacementPrograms {
    pub(crate) fn into_outputs(
        self,
    ) -> (EffectOutcome, Vec<crate::effects::CompletedEffectOutputs>) {
        (self.original, self.outcomes)
    }
    /// Legacy action projections consume the same program receipts once.
    pub(crate) fn into_parts(self) -> (EffectOutcome, Vec<EffectOutcome>) {
        let (original, outputs) = self.into_outputs();
        (
            original,
            outputs
                .into_iter()
                .map(crate::effects::CompletedEffectOutputs::into_outcome)
                .collect(),
        )
    }
    pub(crate) fn into_outcome(self) -> EffectOutcome {
        let (original, outcomes) = self.into_parts();
        EffectOutcome::aggregate_replacement_outcomes(original, outcomes)
    }
}

pub(crate) fn execute_deferred_replacement_programs_with_bindings<T>(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    bindings_for_program: T,
) -> Result<EffectOutcome, ExecutionError>
where
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    complete_deferred_replacement_programs_with_bindings(
        game,
        parent,
        original_outcome,
        programs,
        bindings_for_program,
    )
    .map(CompletedReplacementPrograms::into_outcome)
}

pub(crate) fn complete_deferred_replacement_programs_with_bindings<T>(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    mut original_outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    bindings_for_program: T,
) -> Result<CompletedReplacementPrograms, ExecutionError>
where
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    let game_checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(parent);
    let result = (|| {
        if parent.decision_maker.awaiting_choice() {
            return Ok(CompletedReplacementPrograms {
                original: EffectOutcome::count(0),
                outcomes: Vec::new(),
            });
        }
        let mut outcomes: Vec<crate::effects::CompletedEffectOutputs> = Vec::new();
        for program in programs {
            // The original event has already happened. Its event-time
            // qualifications must be captured before an added instruction can
            // remove a qualifying permanent or change another participant.
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                parent,
                program.effects.first(),
                original_outcome.events.iter_mut().chain(
                    outcomes
                        .iter_mut()
                        .flat_map(|outcome| outcome.outcome.events.iter_mut()),
                ),
            )?;
            let bindings = bindings_for_program(game, &program.context, &original_outcome)?;
            let outcome = execute_replacement_payload_with_outputs(
                game,
                parent,
                &program.effects,
                program.source,
                program.controller,
                &program.context,
                bindings.targets,
                program.source_snapshot,
                bindings.object_tags,
            )?;
            if parent.decision_maker.awaiting_choice() {
                return Ok(CompletedReplacementPrograms {
                    original: EffectOutcome::count(0),
                    outcomes: Vec::new(),
                });
            }
            outcomes.push(outcome);
        }
        for outcome in &mut outcomes {
            outcome.synchronize_observations();
        }
        Ok(CompletedReplacementPrograms {
            original: original_outcome,
            outcomes,
        })
    })();
    if result.is_err() || parent.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            game_checkpoint,
            result.is_ok() && parent.decision_maker.awaiting_choice(),
        );
        context_checkpoint.restore(parent);
    }
    result
}
