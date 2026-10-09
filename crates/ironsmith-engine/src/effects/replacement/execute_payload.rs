//! Execute an instead-payload without losing its event or replacement history.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{ExecutionContext, ExecutionError};
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
    BoundReplacementProgram::acquire(
        game,
        parent,
        ReplacementProgramSchedule::Ordered(
            crate::effects::composition::OrderedProgramCursor::replacement(
                std::borrow::Cow::Borrowed(effects),
            ),
        ),
        source,
        controller,
        context,
        ReplacementProgramBindings {
            targets,
            object_tags,
        },
        captured_source_snapshot,
    )?
    .execute_with_outputs(game, parent)
}

/// Execute one selected replacement original with its authored primary result.
/// Callers own proposal selection, trigger boundaries and suspension; this owner
/// preserves the payload's actual packet while projecting the replaced action.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_replacement_original_payload_with_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    bindings: ReplacementProgramBindings,
    captured_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    original: EffectOutcome,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    BoundReplacementOriginalProgram::acquire(
        game,
        parent,
        std::borrow::Cow::Borrowed(effects),
        source,
        controller,
        context,
        bindings,
        captured_source_snapshot,
        original,
    )?
    .execute_with_outputs(game, parent)
}

/// Project a replaced action over its actual completed subtree. Fresh execution
/// and retained resumption share this owner; neither appends a prefix again.
pub(crate) fn project_replacement_original_outputs(
    original: EffectOutcome,
    payload: crate::effects::CompletedEffectOutputs,
) -> crate::effects::CompletedEffectOutputs {
    let aggregate =
        EffectOutcome::aggregate_replacement_outcomes(original, [payload.outcome.clone()]);
    payload.project_aggregate(aggregate)
}

/// The bound original program owns its actual acquired child context. The
/// existing atomic and draw-aware schedules consume the same value; future
/// authored operands remain unevaluated until their instruction executes.
pub(crate) struct BoundReplacementOriginalProgram<'effects> {
    program: BoundReplacementProgram<'effects>,
    original: EffectOutcome,
}

/// The acquired context and actual cursor have one owner for original and
/// appended payloads. Acquisition does not execute or pre-evaluate a child.
struct BoundReplacementProgram<'effects> {
    frame: crate::effects::composition::CapturedProgramFrame<ReplacementProgramSchedule<'effects>>,
}

enum ReplacementProgramSchedule<'effects> {
    Ordered(crate::effects::composition::OrderedProgramCursor<'effects>),
    DrawContinuation(std::borrow::Cow<'effects, [Effect]>),
}

impl<'effects> ReplacementProgramSchedule<'effects> {
    fn into_cursor(self) -> Box<dyn crate::effects::ActionProgramCursor + 'effects> {
        let cursor = match self {
            Self::Ordered(cursor) => cursor,
            Self::DrawContinuation(effects) => {
                crate::effects::composition::OrderedProgramCursor::replacement(effects)
            }
        };
        Box::new(cursor)
    }

    fn execute_with_outputs(
        self,
        game: &mut GameState,
        child: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_replacement_cursor_with_outputs(game, child, self.into_cursor())
    }

    fn commit_original_with_outputs(
        self,
        game: &mut GameState,
        child: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        match self {
            Self::DrawContinuation(effects) => {
                super::draw_continuation::prepare_bound_original_program_draw_with_outputs(
                    game, child, &effects, original,
                )
            }
            ordered => {
                let cursor = crate::effects::composition::projected_program_cursor(
                    ordered.into_cursor(),
                    move |payload| project_replacement_original_outputs(original, payload),
                );
                execute_replacement_cursor_with_outputs(game, child, cursor)
                    .map(crate::effects::SimultaneousEffectCommit::finished)
            }
        }
    }
}

impl<'effects> BoundReplacementProgram<'effects> {
    #[allow(clippy::too_many_arguments)]
    fn acquire(
        game: &mut GameState,
        parent: &mut ExecutionContext,
        schedule: ReplacementProgramSchedule<'effects>,
        source: ObjectId,
        controller: PlayerId,
        context: &ReplacementEventContext,
        bindings: ReplacementProgramBindings,
        captured_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Result<Self, ExecutionError> {
        with_replacement_child(
            game,
            parent,
            source,
            controller,
            context,
            bindings.targets,
            captured_source_snapshot,
            bindings.object_tags,
            |_, child| {
                Ok(Self {
                    frame: crate::effects::composition::CapturedProgramFrame::capture(
                        schedule, child,
                    ),
                })
            },
        )
    }

    fn run<T>(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
        run: impl FnOnce(
            &mut GameState,
            &mut ExecutionContext,
            ReplacementProgramSchedule<'effects>,
        ) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        self.frame.run(game, parent, run)
    }

    fn execute_with_outputs(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.run(game, parent, |game, child, schedule| {
            schedule.execute_with_outputs(game, child)
        })
    }
}

impl<'effects> BoundReplacementOriginalProgram<'effects> {
    #[allow(clippy::too_many_arguments)]
    fn acquire(
        game: &mut GameState,
        parent: &mut ExecutionContext,
        effects: std::borrow::Cow<'effects, [Effect]>,
        source: ObjectId,
        controller: PlayerId,
        context: &ReplacementEventContext,
        bindings: ReplacementProgramBindings,
        captured_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
        original: EffectOutcome,
    ) -> Result<Self, ExecutionError> {
        let schedule =
            if super::draw_continuation::original_program_uses_draw_continuation(&effects) {
                ReplacementProgramSchedule::DrawContinuation(effects)
            } else {
                ReplacementProgramSchedule::Ordered(
                    crate::effects::composition::OrderedProgramCursor::replacement(effects),
                )
            };
        let program = BoundReplacementProgram::acquire(
            game,
            parent,
            schedule,
            source,
            controller,
            context,
            bindings,
            captured_source_snapshot,
        )?;
        Ok(Self { program, original })
    }

    pub(crate) fn execute_with_outputs(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let frame = self.program.frame.map(|schedule| {
            crate::effects::composition::projected_program_cursor(
                schedule.into_cursor(),
                move |payload| project_replacement_original_outputs(self.original, payload),
            )
        });
        frame.run(game, parent, |game, child, cursor| {
            execute_replacement_cursor_with_outputs(game, child, cursor)
        })
    }

    pub(crate) fn commit_original_with_outputs(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.program.run(game, parent, |game, child, schedule| {
            schedule.commit_original_with_outputs(game, child, self.original)
        })
    }
}

/// Freeze the existing live/LKI/parent precedence before sibling originals.
pub(crate) fn capture_replacement_source_snapshot(
    game: &GameState,
    parent: &ExecutionContext,
    source: ObjectId,
) -> Option<crate::snapshot::ObjectSnapshot> {
    game.object(source)
        .filter(|_| !game.is_phased_out(source))
        .map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .or_else(|| game.source_last_known_snapshot(source).cloned())
        .or_else(|| {
            parent
                .source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == source)
                .cloned()
        })
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
    let source_snapshot = captured_source_snapshot
        .or_else(|| capture_replacement_source_snapshot(game, parent, source));
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
    execute_replacement_cursor_with_outputs(
        game,
        child,
        Box::new(
            crate::effects::composition::OrderedProgramCursor::replacement(
                std::borrow::Cow::Borrowed(effects),
            ),
        ),
    )
}

fn execute_replacement_cursor_with_outputs<'cursor>(
    game: &mut GameState,
    child: &mut ExecutionContext,
    cursor: Box<dyn crate::effects::ActionProgramCursor + 'cursor>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
        crate::effects::composition::execute_observed_replacement_cursor_with_outputs(
            game, child, cursor,
        )
    })
}

/// Explicit bindings for one captured replacement program. Each child scope
/// receives its own bindings; the interrupted instruction's tags are untouched.
#[derive(Debug, Clone)]
pub(crate) struct ReplacementProgramBindings {
    pub targets: Option<Vec<crate::effects::ResolvedTarget>>,
    pub object_tags: Vec<(String, Vec<crate::snapshot::ObjectSnapshot>)>,
}

/// An original replacement programme selected before any original commits.
/// Its execution scope is immutable input, not a completed output or addition.
#[derive(Debug)]
pub(crate) struct PreparedReplacementOriginal {
    pub program: crate::events::processing::PreparedReplacementProgram,
    pub scope: crate::effects::ReplacementExecutionContext,
    pub original: EffectOutcome,
    pub bindings: ReplacementProgramBindings,
}

impl PreparedReplacementOriginal {
    /// Acquire the actual original program without executing a child. This is
    /// the ownership handoff for original coordinators: source, event history,
    /// bindings and selected program state travel together to the consumer.
    /// Acquisition stays at the caller's original boundary; it does not freeze
    /// future authored operands or change the program's scheduling eligibility.
    pub(crate) fn acquire_bound_original(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<BoundReplacementOriginalProgram<'static>, ExecutionError> {
        let inherited = std::mem::replace(&mut ctx.replacement, self.scope);
        let result = (|| {
            let mut original = self.original;
            // Qualify pending observations before the first instruction can
            // change their sources; simultaneous owners hold this boundary.
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                self.program.effects.first(),
                original.events.iter_mut(),
            )?;
            BoundReplacementOriginalProgram::acquire(
                game,
                ctx,
                std::borrow::Cow::Owned(self.program.effects),
                self.program.source,
                self.program.controller,
                &self.program.context,
                self.bindings,
                self.program.source_snapshot,
                original,
            )
        })();
        ctx.replacement = inherited;
        result
    }

    pub(crate) fn commit_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.acquire_bound_original(game, ctx)?
            .execute_with_outputs(game, ctx)
    }

    /// Return the committed prefix and its actual draw/tail continuation.
    /// Unsupported programs keep their existing atomic execution contract.
    pub(crate) fn commit_original_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.acquire_bound_original(game, ctx)?
            .commit_original_with_outputs(game, ctx)
    }
}

/// Commit an already selected replacement original in its caller-owned scope.
/// Event families retain snapshot acquisition and recipient binding policies;
/// the bound owner selects native continuation versus ordered execution once.
/// Outer additions and nominal cost acknowledgement remain with their owners.
pub(crate) fn commit_bound_replacement_program_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    program: crate::events::processing::PreparedReplacementProgram,
    bindings: ReplacementProgramBindings,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    BoundReplacementOriginalProgram::acquire(
        game,
        ctx,
        std::borrow::Cow::Owned(program.effects),
        program.source,
        program.controller,
        &program.context,
        bindings,
        program.source_snapshot,
        replaced_original_outcome(),
    )?
    .commit_original_with_outputs(game, ctx)
}

fn replaced_original_outcome() -> EffectOutcome {
    let mut original = EffectOutcome::replaced();
    original.set_value(crate::effect::OutcomeValue::Count(0));
    original
}

/// Commit a retained original replacement program in its captured event scope.
/// The action adapter supplies exact subject bindings; this owner retains the
/// committed prefix and draw continuation through the common payload runner.
pub(crate) fn commit_replacement_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    program: crate::events::processing::PreparedReplacementProgram,
    bindings: impl FnOnce(
        &ReplacementEventContext,
    ) -> Result<ReplacementProgramBindings, ExecutionError>,
    action_name: &'static str,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    if program
        .effects
        .iter()
        .any(crate::effects::replacement::replacement_effect_contains_draw)
        && !program
            .effects
            .iter()
            .all(crate::effects::replacement::replacement_effect_supported)
    {
        return Err(ExecutionError::Impossible(format!(
            "{action_name} replacement original has no native draw-continuation owner"
        )));
    }
    let bindings = bindings(&program.context)?;
    let original = replaced_original_outcome();
    PreparedReplacementOriginal {
        program,
        scope: ctx.replacement.clone(),
        original,
        bindings,
    }
    .commit_original_with_outputs(game, ctx)
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
    let (original, programs) = result.into_expansion();
    crate::effects::composition::execute_result_transaction(game, parent, |game, parent| {
        let original_outcome = commit_original(game, parent, original)?;
        crate::effects::replacement::complete_replacement_programs_with_original_outputs(
            game,
            parent,
            original_outcome,
            |game, parent, original| {
                complete_deferred_replacement_programs_with_bindings(
                    game,
                    parent,
                    original,
                    programs,
                    bindings_for_program,
                )
            },
        )
    })
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
    original_outcome: EffectOutcome,
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
    complete_replacement_programs_with_inputs(
        game,
        parent,
        original_outcome,
        programs
            .into_iter()
            .map(|program| (program, None))
            .collect(),
        bindings_for_program,
    )
}

/// Complete appended replacement programs against the actual original packet.
/// The caller supplies its existing bound or lazy-binding executor; this owner
/// retains the packet, handles suspension and appends its actual child outputs.
pub(crate) fn complete_replacement_programs_with_original_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    outputs: crate::effects::CompletedEffectOutputs,
    complete_programs: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext,
        EffectOutcome,
    ) -> Result<CompletedReplacementPrograms, ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if parent.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let completed = complete_programs(game, parent, outputs.outcome.clone())?;
    if parent.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    Ok(outputs.append_batch_program_outputs(completed))
}

/// Execute captured bindings through the same replacement-program transaction.
pub(crate) fn complete_bound_replacement_programs_with_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    original_outcome: EffectOutcome,
    programs: Vec<(
        crate::events::processing::PreparedReplacementProgram,
        ReplacementProgramBindings,
    )>,
) -> Result<CompletedReplacementPrograms, ExecutionError> {
    complete_replacement_programs_with_inputs(
        game,
        parent,
        original_outcome,
        programs
            .into_iter()
            .map(|(program, bindings)| (program, Some(bindings)))
            .collect(),
        |_, _, _| {
            Err(ExecutionError::InternalError(
                "bound replacement program lost its captured bindings".into(),
            ))
        },
    )
}

fn complete_replacement_programs_with_inputs<T>(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    mut original_outcome: EffectOutcome,
    programs: Vec<(
        crate::events::processing::PreparedReplacementProgram,
        Option<ReplacementProgramBindings>,
    )>,
    bindings_for_program: T,
) -> Result<CompletedReplacementPrograms, ExecutionError>
where
    T: Fn(
        &GameState,
        &ReplacementEventContext,
        &EffectOutcome,
    ) -> Result<ReplacementProgramBindings, ExecutionError>,
{
    crate::effects::composition::execute_result_transaction(game, parent, |game, parent| {
        if parent.decision_maker.awaiting_choice() {
            return Ok(CompletedReplacementPrograms {
                original: EffectOutcome::count(0),
                outcomes: Vec::new(),
            });
        }
        let mut outcomes: Vec<crate::effects::CompletedEffectOutputs> = Vec::new();
        for (program, captured_bindings) in programs {
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
            let bindings = match captured_bindings {
                Some(bindings) => bindings,
                None => bindings_for_program(game, &program.context, &original_outcome)?,
            };
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
    })
}
