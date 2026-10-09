//! Authored program cursors retain child actions across participant barriers.

use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionContextCheckpoint, ExecutionError,
    SimultaneousEffectCommit, SimultaneousEffectCompletion, SimultaneousEffectProposal,
};
use crate::game_state::{GameState, TargetAssignment};

/// Context owned by one authored child. Re-enter it for preparation, originals
/// and completion without retaining a borrowed execution context between phases.
#[derive(Debug, Clone, Default)]
pub struct ProgramActionScope {
    /// A selected payment clause retains its payer/cause/reason owner across
    /// every child phase. Absence inherits the enclosing execution purpose.
    pub(crate) payment: Option<crate::costs::PaymentScope>,
    pub(crate) targets: Option<(Vec<crate::effects::ResolvedTarget>, Vec<TargetAssignment>)>,
    pub(crate) mode_label: Option<(crate::ids::ObjectId, String)>,
    pub(crate) iterated_player: Option<Option<crate::ids::PlayerId>>,
    pub(crate) iterated_object: Option<Option<crate::ids::ObjectId>>,
    pub(crate) optional_action: Option<bool>,
    pub(crate) optional_identity_guard:
        Option<Option<crate::effects::context::OptionalIdentityGuard>>,
    pub(crate) public_search_reveal_tag: Option<Option<crate::tag::TagKey>>,
    pub(crate) pending_entry_attachment: Option<Option<crate::target::ChooseSpec>>,
    pub(crate) outer: Option<Box<ProgramActionScope>>,
}

impl ProgramActionScope {
    pub(crate) fn run<T>(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        // Applying these independent overrides has no intermediate action.
        // Select each innermost authored override, then enter the existing
        // temporary-context owner once. Its announced-target preservation and
        // restoration are identical to nested entry, without recursive closure
        // monomorphization or replaying a child.
        let mut effective = ProgramActionScope::default();
        let mut scope = Some(self);
        while let Some(current) = scope {
            if effective.payment.is_none() {
                effective.payment = current.payment.clone();
            }
            if effective.targets.is_none() {
                effective.targets = current.targets.clone();
            }
            if effective.mode_label.is_none() {
                effective.mode_label = current.mode_label.clone();
            }
            if effective.iterated_object.is_none() {
                effective.iterated_object = current.iterated_object;
            }
            if effective.iterated_player.is_none() {
                effective.iterated_player = current.iterated_player;
            }
            if effective.optional_action.is_none() {
                effective.optional_action = current.optional_action;
            }
            if effective.optional_identity_guard.is_none() {
                effective.optional_identity_guard = current.optional_identity_guard.clone();
            }
            if effective.public_search_reveal_tag.is_none() {
                effective.public_search_reveal_tag = current.public_search_reveal_tag.clone();
            }
            if effective.pending_entry_attachment.is_none() {
                effective.pending_entry_attachment = current.pending_entry_attachment.clone();
            }
            scope = current.outer.as_deref();
        }
        effective.run_local(game, ctx, body)
    }

    pub(crate) fn execution_purpose(
        &self,
        inherited: crate::effects::EffectExecutionPurpose,
    ) -> crate::effects::EffectExecutionPurpose {
        let mut scope = Some(self);
        while let Some(current) = scope {
            if current.payment.is_some() {
                return crate::effects::EffectExecutionPurpose::Payment;
            }
            scope = current.outer.as_deref();
        }
        inherited
    }

    /// Add an enclosing scope without discarding scopes already authored by
    /// this child. The iterative execution walk retains innermost precedence.
    pub(super) fn prepend_outer(&mut self, outer: Option<Box<ProgramActionScope>>) {
        let mut scope = self;
        while let Some(ref mut parent) = scope.outer {
            scope = parent;
        }
        scope.outer = outer;
    }

    fn run_local<T>(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        let optional = self
            .optional_action
            .map(|value| std::mem::replace(&mut ctx.optional_action, value));
        let guard = self
            .optional_identity_guard
            .as_ref()
            .map(|value| std::mem::replace(&mut ctx.optional_identity_guard, value.clone()));
        let reveal = self
            .public_search_reveal_tag
            .as_ref()
            .map(|value| std::mem::replace(&mut ctx.public_search_reveal_tag, value.clone()));
        let attachment = self
            .pending_entry_attachment
            .as_ref()
            .map(|value| std::mem::replace(&mut ctx.pending_entry_attachment, value.clone()));
        let label = self
            .mode_label
            .as_ref()
            .map(|label| game.replace_resolving_mode_context(Some(label.clone())));
        let result = if let Some(payment) = &self.payment {
            payment.run(ctx, |ctx| self.run_player_scope(game, ctx, body))
        } else {
            self.run_player_scope(game, ctx, body)
        };
        if let Some(label) = label {
            game.replace_resolving_mode_context(label);
        }
        if let Some(reveal) = reveal {
            ctx.public_search_reveal_tag = reveal;
        }
        if let Some(attachment) = attachment {
            ctx.pending_entry_attachment = attachment;
        }
        if let Some(guard) = guard {
            ctx.optional_identity_guard = guard;
        }
        if let Some(optional) = optional {
            ctx.optional_action = optional;
        }
        result
    }

    fn run_player_scope<T>(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        if let Some(player) = self.iterated_player {
            ctx.with_temp_iterated_player(player, |ctx| self.run_object_scope(game, ctx, body))
        } else {
            self.run_object_scope(game, ctx, body)
        }
    }

    fn run_object_scope<T>(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        if let Some(object) = self.iterated_object {
            ctx.with_temp_iterated_object(object, |ctx| self.run_targets(game, ctx, body))
        } else {
            self.run_targets(game, ctx, body)
        }
    }

    fn run_targets<T>(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        if let Some((targets, assignments)) = &self.targets {
            ctx.with_temp_targets(targets.clone(), |ctx| {
                ctx.with_temp_target_assignments(assignments.clone(), |ctx| body(game, ctx))
            })
        } else {
            body(game, ctx)
        }
    }
}

/// A selected container can forward its opaque physical damage domain.
/// This retains one native cohort instead of splitting it into iteration units.
#[derive(Debug)]
pub(crate) enum NativeProgramAction {
    SharedDamage(Box<dyn SimultaneousEffectProposal>),
    /// An already selected ordinary original. Physical input and rich output
    /// ownership stay with its proposal; payment requests remain TotalCost.
    PreparedOriginal(Box<dyn SimultaneousEffectProposal>),
    /// A domain request, not an already-frozen proposal. Ordinary execution
    /// retains sequential payment timing; staged execution prepares its owner.
    TotalCost {
        cost: crate::cost::TotalCost,
        payer: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    },
}

#[derive(Debug)]
pub struct ProgramAction {
    pub(crate) effect: Effect,
    pub(crate) scope: ProgramActionScope,
    pub(crate) native: Option<NativeProgramAction>,
    /// The authored child path, independent of the affected participant.
    pub identity: Vec<usize>,
}

impl ProgramAction {
    /// Domain purpose governs error acknowledgement even inside an enclosing
    /// action program whose authored children may skip invalid targets.
    fn execution_purpose(
        &self,
        inherited: crate::effects::EffectExecutionPurpose,
    ) -> crate::effects::EffectExecutionPurpose {
        if matches!(self.native, Some(NativeProgramAction::TotalCost { .. })) {
            crate::effects::EffectExecutionPurpose::Payment
        } else {
            self.scope.execution_purpose(inherited)
        }
    }

    pub fn new(effect: Effect) -> Self {
        Self {
            effect,
            native: None,
            scope: ProgramActionScope::default(),
            identity: Vec::new(),
        }
    }
    /// Retain an existing selected proposal or prepare its request in the same
    /// authored scope. Cohort admission, error policy and phase barriers belong
    /// to the enclosing coordinator; this owner does not seal or commit it.
    fn prepare_original_owner(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        purpose: crate::effects::EffectExecutionPurpose,
    ) -> Result<Option<Box<dyn SimultaneousEffectProposal>>, ExecutionError> {
        let purpose = self.execution_purpose(purpose);
        match self.native.take() {
            Some(NativeProgramAction::SharedDamage(proposal))
            | Some(NativeProgramAction::PreparedOriginal(proposal)) => Ok(Some(proposal)),
            request => self.scope.run(game, ctx, |game, ctx| match request {
                Some(NativeProgramAction::TotalCost {
                    cost,
                    payer,
                    reason,
                }) => {
                    crate::costs::prepare_total_cost_program_action(&cost, game, ctx, payer, reason)
                }
                None => super::prepared_branch::prepare_action_for_purpose(
                    &self.effect,
                    purpose,
                    game,
                    ctx,
                ),
                Some(NativeProgramAction::SharedDamage(_))
                | Some(NativeProgramAction::PreparedOriginal(_)) => unreachable!(),
            }),
        }
    }

    /// Execute this selected child once. The cursor retains its authored identity
    /// and acknowledgement; this owner consumes the native proposal in its scope.
    pub(super) fn execute_with_outputs(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        purpose: crate::effects::EffectExecutionPurpose,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let purpose = self.execution_purpose(purpose);
        let native = self.native.take();
        self.scope.run(game, ctx, |game, ctx| {
            execute_program_child_with_outputs(&self.effect, native, game, ctx, purpose)
        })
    }

    /// Retain this child's actual draw/tail continuation. Deferral is a native
    /// draw boundary, not permission to regroup unrelated authored instructions.
    pub(super) fn prepare_draw_boundary(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::replacement::PreparedReplacementChild, ExecutionError> {
        let purpose = self.execution_purpose(crate::effects::EffectExecutionPurpose::Action);
        let native = self.native.take();
        let action = &*self;
        self.scope.run(game, ctx, |game, ctx| match native {
            Some(NativeProgramAction::SharedDamage(mut proposal)) => {
                    proposal.prepare_selection(game, ctx)?;
                    proposal.prepare_original(game, ctx)?;
                    let inputs = proposal.damage_action_inputs().ok_or_else(|| {
                        ExecutionError::InternalError(
                            "prepared iteration damage lost its shared inputs".into(),
                        )
                    })?;
                    let opened = game.open_simultaneous_action();
                    let receipt = (|| {
                        let owner = inputs.seal(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(SimultaneousEffectCommit::finished(
                                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                            ));
                        }
                        owner.commit_original_with_outputs(game, ctx)
                    })();
                    game.close_simultaneous_action(opened);
                    let receipt = super::adapt_original_outcome_with_outputs(
                        receipt?,
                        Box::new(ProgramDamageBinding(proposal)),
                        game,
                        ctx,
                    )?;
                    crate::effects::replacement::prepare_committed_draw_boundary(game, ctx, receipt)
            }
            Some(NativeProgramAction::PreparedOriginal(proposal)) => {
                    let receipt = crate::effects::replacement::prepare_native_proposal_draw_continuation_with_outputs(
                        action.effect.0.result_action(), game, ctx, |_, _| Ok(proposal),
                    )?;
                    crate::effects::replacement::retain_prepared_draw_boundary(receipt, ctx)
            }
            None if matches!(purpose, crate::effects::EffectExecutionPurpose::Action) => {
                crate::effects::replacement::prepare_replacement_child(game, ctx, &action.effect)
            }
            native => execute_program_child_with_outputs(
                &action.effect, native, game, ctx, purpose,
            ).map(crate::effects::replacement::PreparedReplacementChild::finished_with_outputs),
        })
    }
}

/// One physical dispatcher for an authored child. Bare ordered/replacement
/// programs inherit their caller's context; selected cursors enter their actual
/// ProgramActionScope before reaching the same domain owners here.
pub(super) fn execute_program_child_with_outputs(
    effect: &Effect,
    native: Option<NativeProgramAction>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    match native {
        Some(NativeProgramAction::SharedDamage(proposal)) => {
            crate::effects::damage::complete_prepared_damage_action(game, ctx, proposal)
        }
        Some(NativeProgramAction::PreparedOriginal(proposal)) => {
            super::complete_prepared_original_with_outputs(proposal, game, ctx, false)
        }
        Some(NativeProgramAction::TotalCost {
            cost,
            payer,
            reason,
        }) => crate::costs::execute_total_cost_program_action(&cost, game, ctx, payer, reason),
        None => purpose.execute(game, effect, ctx),
    }
}

/// Bind the authored program only after its one physical damage owner
/// completes. The view changes the primary result without duplicating history.
struct ProgramDamageBinding(Box<dyn crate::effects::SimultaneousEffectProposal>);
impl super::OriginalOutcomeAdapter for ProgramDamageBinding {
    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<EffectOutcome, ExecutionError>,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.finish_with_outputs(
            game,
            ctx,
            result.map(CompletedEffectOutputs::aggregate_only),
        )
        .map(CompletedEffectOutputs::into_outcome)
    }
    fn finish_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<CompletedEffectOutputs, ExecutionError>,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let mut outputs = result?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(outputs);
        }
        let binding = self.0.bind_damage_action(game, ctx, &outputs)?;
        let primary = binding.transfer_owned_outputs(&mut outputs);
        let observations = outputs.outcome.clone();
        Ok(outputs.project_aggregate(primary.with_authoritative_observations(observations)))
    }
}

pub struct ProgramCompletion {
    pub(crate) outputs: CompletedEffectOutputs,
    /// Genuine parent acknowledgements, separate from child observations.
    pub(crate) facts: Vec<ExecutionFact>,
}

impl ProgramCompletion {
    pub fn new(outputs: CompletedEffectOutputs) -> Self {
        Self {
            outputs,
            facts: Vec::new(),
        }
    }
}

/// A selected program yields only its next authored child. Future inputs are
/// read after the current unit completes, never frozen speculatively up front.
/// A selected program can declare a captured claim before physical originals.
/// The owner captures its required context at selection; the coordinator runs
/// each declaration once after participant offers, before their body actions.
pub struct ProgramPreparation {
    action: Box<dyn FnOnce(&mut GameState) -> Result<(), ExecutionError> + Send>,
}
impl ProgramPreparation {
    pub fn new(
        action: impl FnOnce(&mut GameState) -> Result<(), ExecutionError> + Send + 'static,
    ) -> Self {
        Self {
            action: Box::new(action),
        }
    }
    pub(super) fn prepare(self, game: &mut GameState) -> Result<(), ExecutionError> {
        (self.action)(game)
    }
}

/// One standalone execution selection. A borrowed definition remains with its
/// cursor; an owned action retains native state and authored scope. Neither
/// shape declares shared-action eligibility or freezes future operands.
pub struct ProgramInstructionSelection<'cursor> {
    instruction: Option<ExecutableProgramInstruction<'cursor>>,
    preparations: Vec<ProgramPreparation>,
    dispatch_while_pending: bool,
}

enum ExecutableProgramInstruction<'cursor> {
    Selected(ProgramAction),
    Borrowed(&'cursor Effect),
}

impl<'cursor> ProgramInstructionSelection<'cursor> {
    pub fn new(action: Option<ProgramAction>, preparations: Vec<ProgramPreparation>) -> Self {
        Self {
            instruction: action.map(ExecutableProgramInstruction::Selected),
            preparations,
            dispatch_while_pending: false,
        }
    }

    pub(super) fn borrowed(
        instruction: Option<&'cursor Effect>,
        dispatch_while_pending: bool,
    ) -> Self {
        Self {
            instruction: instruction.map(ExecutableProgramInstruction::Borrowed),
            preparations: Vec::new(),
            dispatch_while_pending,
        }
    }
}

impl ExecutableProgramInstruction<'_> {
    fn execute(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        purpose: crate::effects::EffectExecutionPurpose,
    ) -> (
        crate::effects::EffectExecutionPurpose,
        Result<CompletedEffectOutputs, ExecutionError>,
    ) {
        match self {
            Self::Selected(mut action) => {
                let action_purpose = action.execution_purpose(purpose);
                (
                    action_purpose,
                    action.execute_with_outputs(game, ctx, purpose),
                )
            }
            Self::Borrowed(effect) => (
                purpose,
                execute_program_child_with_outputs(effect, None, game, ctx, purpose),
            ),
        }
    }
}

pub trait ActionProgramCursor: std::fmt::Debug + Send {
    /// Drain selected declarations after next_action. None plus declarations
    /// pauses traversal; prepare them and retry instead of finishing the cursor.
    fn take_preparations(&mut self) -> Vec<ProgramPreparation> {
        Vec::new()
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError>;
    /// Select a standalone instruction without requiring a definition clone.
    /// Staged coordinators keep using next_action's owned declarations. Pending
    /// selection must not drain declarations that the caller has not admitted.
    fn select_execution_instruction(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ProgramInstructionSelection<'_>, ExecutionError> {
        let next = self.next_action(game, ctx)?;
        let preparations = if ctx.decision_maker.awaiting_choice() {
            Vec::new()
        } else {
            self.take_preparations()
        };
        Ok(ProgramInstructionSelection::new(next, preparations))
    }
    fn accept_action(&mut self, outputs: CompletedEffectOutputs) -> Result<(), ExecutionError>;
    /// Acknowledge the actual child packet at its owning context boundary.
    /// Cursors with post-child observation obligations perform them here before
    /// any next declaration is selected; legacy cursors retain pure acknowledgement.
    fn accept_action_with_context(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        outputs: CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.accept_action(outputs)
    }

    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError>;
    /// The authored parent owns the packet exposed on suspension. Modal
    /// transactions use a neutral packet; Sequence retains its partial prefix.
    fn finish_pending(self: Box<Self>) -> Result<CompletedEffectOutputs, ExecutionError> {
        Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ))
    }
    /// Stop a successfully interrupted program without selecting another child.
    /// Native owners unwind reserved scopes and retain actual prefix packets.
    fn finish_stopped(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        self.finish_pending().map(ProgramCompletion::new)
    }
    fn ends_action_unit(&self) -> bool {
        false
    }
    fn continues_past_illegal_targets(&self) -> bool {
        false
    }
}

/// A completed selection can own a genuine no-action/invalid result without
/// yielding a dummy child or manufacturing a physical original receipt.
pub(super) fn finished_program_cursor(
    outputs: CompletedEffectOutputs,
) -> Box<dyn ActionProgramCursor> {
    finished_program_completion_cursor(ProgramCompletion::new(outputs))
}

pub(super) fn finished_program_completion_cursor(
    completed: ProgramCompletion,
) -> Box<dyn ActionProgramCursor> {
    Box::new(FinishedProgramCursor { completed })
}

struct FinishedProgramCursor {
    completed: ProgramCompletion,
}
impl std::fmt::Debug for FinishedProgramCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FinishedProgramCursor")
            .finish_non_exhaustive()
    }
}
impl ActionProgramCursor for FinishedProgramCursor {
    fn finish_stopped(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        Ok(self.completed)
    }
    fn next_action(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        Ok(None)
    }
    fn accept_action(&mut self, _outputs: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        Err(ExecutionError::InternalError(
            "finished program has no child to acknowledge".into(),
        ))
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        Ok(self.completed)
    }
}

/// Retain a pure enclosing result projection beside the actual selected cursor.
/// Every terminal path consumes the real child packet once; selection, context,
/// authored identity, declarations and contextual acknowledgement stay with it.
pub(crate) fn projected_program_cursor<'cursor>(
    inner: Box<dyn ActionProgramCursor + 'cursor>,
    project: impl FnOnce(CompletedEffectOutputs) -> CompletedEffectOutputs + Send + 'cursor,
) -> Box<dyn ActionProgramCursor + 'cursor> {
    Box::new(ProjectedProgramCursor {
        inner,
        project: Box::new(project),
    })
}

struct ProjectedProgramCursor<'cursor> {
    inner: Box<dyn ActionProgramCursor + 'cursor>,
    project: Box<dyn FnOnce(CompletedEffectOutputs) -> CompletedEffectOutputs + Send + 'cursor>,
}

impl std::fmt::Debug for ProjectedProgramCursor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectedProgramCursor")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl ActionProgramCursor for ProjectedProgramCursor<'_> {
    fn take_preparations(&mut self) -> Vec<ProgramPreparation> {
        self.inner.take_preparations()
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        self.inner.next_action(game, ctx)
    }
    fn select_execution_instruction(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ProgramInstructionSelection<'_>, ExecutionError> {
        self.inner.select_execution_instruction(game, ctx)
    }
    fn accept_action(&mut self, outputs: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        self.inner.accept_action(outputs)
    }
    fn accept_action_with_context(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outputs: CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.inner.accept_action_with_context(game, ctx, outputs)
    }
    fn ends_action_unit(&self) -> bool {
        self.inner.ends_action_unit()
    }
    fn continues_past_illegal_targets(&self) -> bool {
        self.inner.continues_past_illegal_targets()
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        let Self { inner, project } = *self;
        let mut completed = inner.finish()?;
        completed.outputs = project(completed.outputs);
        Ok(completed)
    }
    fn finish_pending(self: Box<Self>) -> Result<CompletedEffectOutputs, ExecutionError> {
        let Self { inner, project } = *self;
        inner.finish_pending().map(project)
    }
    fn finish_stopped(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        let Self { inner, project } = *self;
        let mut completed = inner.finish_stopped(game, ctx)?;
        completed.outputs = project(completed.outputs);
        Ok(completed)
    }
}

/// A wrapper retains its actual child program and the same completion
/// adapter used by native proposals. The enclosing transaction owns pending
/// and error rollback; no result is published until the whole child finishes.
pub(super) fn adapted_child_program_cursor(
    child: Effect,
    adapter: Box<dyn super::OriginalOutcomeAdapter>,
) -> Box<dyn ActionProgramCursor> {
    Box::new(AdaptedChildProgramCursor {
        child: Some(child),
        outputs: None,
        adapter: Some(adapter),
    })
}

struct AdaptedChildProgramCursor {
    child: Option<Effect>,
    outputs: Option<CompletedEffectOutputs>,
    adapter: Option<Box<dyn super::OriginalOutcomeAdapter>>,
}
impl std::fmt::Debug for AdaptedChildProgramCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdaptedChildProgramCursor")
            .field("child_pending", &self.child.is_some())
            .finish_non_exhaustive()
    }
}
impl ActionProgramCursor for AdaptedChildProgramCursor {
    fn finish_stopped(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        let outputs = match self.outputs.take() {
            Some(outputs) => match self.adapter.take() {
                Some(adapter) => adapter.finish_with_outputs(game, ctx, Ok(outputs))?,
                None => outputs,
            },
            None => {
                if let Some(adapter) = self.adapter.take() {
                    adapter.cancel(game, ctx);
                }
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))
            }
        };
        Ok(ProgramCompletion::new(outputs))
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if let Some(child) = self.child.take() {
            return Ok(Some(ProgramAction {
                effect: child,
                native: None,
                scope: ProgramActionScope::default(),
                identity: vec![0],
            }));
        }
        if let Some(adapter) = self.adapter.take() {
            let outputs = self.outputs.take().ok_or_else(|| {
                ExecutionError::InternalError(
                    "wrapped program completed without its child packet".into(),
                )
            })?;
            self.outputs = Some(adapter.finish_with_outputs(game, ctx, Ok(outputs))?);
        }
        Ok(None)
    }
    fn accept_action(&mut self, outputs: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        if self.child.is_some() || self.outputs.is_some() || self.adapter.is_none() {
            return Err(ExecutionError::InternalError(
                "unexpected wrapped program acknowledgement".into(),
            ));
        }
        self.outputs = Some(outputs);
        Ok(())
    }
    fn ends_action_unit(&self) -> bool {
        self.child.is_none()
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        if self.child.is_some() || self.adapter.is_some() {
            return Err(ExecutionError::InternalError(
                "unfinished wrapped program".into(),
            ));
        }
        Ok(ProgramCompletion::new(self.outputs.ok_or_else(|| {
            ExecutionError::InternalError("wrapped program lost its completed packet".into())
        })?))
    }
}

/// Entry declarations are metadata, not synthetic child actions. Ordinary
/// execution runs them before the body; simultaneous execution drains them
/// into the common post-selection preparation barrier.
pub(super) fn prepared_program_cursor(
    inner: Box<dyn ActionProgramCursor>,
    preparation: ProgramPreparation,
) -> Box<dyn ActionProgramCursor> {
    Box::new(PreparedProgramCursor {
        inner,
        preparations: vec![preparation],
    })
}
struct PreparedProgramCursor {
    inner: Box<dyn ActionProgramCursor>,
    preparations: Vec<ProgramPreparation>,
}
impl std::fmt::Debug for PreparedProgramCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedProgramCursor")
            .field("preparations", &self.preparations.len())
            .finish_non_exhaustive()
    }
}
impl ActionProgramCursor for PreparedProgramCursor {
    fn finish_stopped(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        self.inner.finish_stopped(game, ctx)
    }
    fn take_preparations(&mut self) -> Vec<ProgramPreparation> {
        let mut preparations = std::mem::take(&mut self.preparations);
        preparations.extend(self.inner.take_preparations());
        preparations
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        if !self.preparations.is_empty() {
            return Ok(None);
        }
        self.inner.next_action(game, ctx)
    }
    fn accept_action(&mut self, outputs: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        self.inner.accept_action(outputs)
    }
    fn accept_action_with_context(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outputs: CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.inner.accept_action_with_context(game, ctx, outputs)
    }

    fn ends_action_unit(&self) -> bool {
        self.inner.ends_action_unit()
    }
    fn continues_past_illegal_targets(&self) -> bool {
        self.inner.continues_past_illegal_targets()
    }
    fn finish_pending(self: Box<Self>) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.inner.finish_pending()
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        if !self.preparations.is_empty() {
            return Err(ExecutionError::InternalError(
                "program completed without draining its declarations".into(),
            ));
        }
        self.inner.finish()
    }
}

/// Forward an actual selected native damage cohort as one authored action.
/// Inputs/proposals belong to existing physical owners; no executor is rerun.
pub(super) fn prepared_damage_program_cursor(
    effect: Effect,
    proposal: Box<dyn SimultaneousEffectProposal>,
) -> Box<dyn ActionProgramCursor> {
    native_program_cursor(effect, NativeProgramAction::SharedDamage(proposal))
}

/// Forward any retained ordinary original through the shared proposal phases.
pub(super) fn prepared_original_program_cursor(
    effect: Effect,
    proposal: Box<dyn SimultaneousEffectProposal>,
) -> Box<dyn ActionProgramCursor> {
    native_program_cursor(effect, NativeProgramAction::PreparedOriginal(proposal))
}

fn native_program_cursor(
    effect: Effect,
    native: NativeProgramAction,
) -> Box<dyn ActionProgramCursor> {
    let mut action = ProgramAction::new(effect);
    action.identity = vec![0];
    action.native = Some(native);
    Box::new(NativeProgramCursor {
        action: Some(action),
        outputs: None,
    })
}
struct NativeProgramCursor {
    action: Option<ProgramAction>,
    outputs: Option<CompletedEffectOutputs>,
}
impl std::fmt::Debug for NativeProgramCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeProgramCursor")
            .field("action_pending", &self.action.is_some())
            .finish_non_exhaustive()
    }
}
impl ActionProgramCursor for NativeProgramCursor {
    fn finish_stopped(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        Ok(ProgramCompletion::new(self.outputs.unwrap_or_else(|| {
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))
        })))
    }
    fn next_action(
        &mut self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(self.action.take())
    }
    fn accept_action(&mut self, outputs: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        if self.action.is_some() || self.outputs.is_some() {
            return Err(ExecutionError::InternalError(
                "unexpected native program acknowledgement".into(),
            ));
        }
        self.outputs = Some(outputs);
        Ok(())
    }
    fn ends_action_unit(&self) -> bool {
        true
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        Ok(ProgramCompletion::new(self.outputs.ok_or_else(|| {
            ExecutionError::InternalError(
                "native program completed without its actual packet".into(),
            )
        })?))
    }
}

pub(crate) struct ProgramParticipant {
    frame: super::CapturedProgramFrame<Option<Box<dyn ActionProgramCursor>>>,
    result: Option<CompletedEffectOutputs>,
}

impl ProgramParticipant {
    pub(crate) fn new(cursor: Box<dyn ActionProgramCursor>, ctx: &ExecutionContext) -> Self {
        Self {
            frame: super::CapturedProgramFrame::capture(
                Some(Box::new(NestedProgramCursor::new(cursor)) as Box<dyn ActionProgramCursor>),
                ctx,
            ),
            result: None,
        }
    }
}

struct NestedProgramParent {
    cursor: Box<dyn ActionProgramCursor>,
    action: ProgramAction,
}

/// Selected containers retain their own result owner while the coordinator
/// visits actual child units. This stack is not an effect-list flattening pass.
struct NestedProgramCursor {
    active: Box<dyn ActionProgramCursor>,
    parents: Vec<NestedProgramParent>,
    completed_facts: Vec<ExecutionFact>,
    preparations: Vec<ProgramPreparation>,
}

impl std::fmt::Debug for NestedProgramCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NestedProgramCursor")
            .field("depth", &self.parents.len())
            .finish_non_exhaustive()
    }
}

impl NestedProgramCursor {
    fn new(active: Box<dyn ActionProgramCursor>) -> Self {
        Self {
            active,
            parents: Vec::new(),
            completed_facts: Vec::new(),
            preparations: Vec::new(),
        }
    }
    fn parent_scope(&self) -> Option<Box<ProgramActionScope>> {
        let mut outer = None;
        for parent in &self.parents {
            let mut scope = parent.action.scope.clone();
            scope.prepend_outer(outer);
            outer = Some(Box::new(scope));
        }
        outer
    }
    fn child_identity(&self, child: &[usize]) -> Vec<usize> {
        if self.parents.is_empty() {
            return child.to_vec();
        }
        if child.is_empty()
            || self
                .parents
                .iter()
                .any(|parent| parent.action.identity.is_empty())
        {
            return Vec::new();
        }
        // Length-delimited authored path segments preserve container boundaries
        // and distinct modal occurrences without aliasing concatenated indices.
        let mut identity = vec![usize::MAX];
        for segment in self
            .parents
            .iter()
            .map(|parent| parent.action.identity.as_slice())
            .chain(std::iter::once(child))
        {
            identity.push(segment.len());
            identity.extend_from_slice(segment);
        }
        identity
    }
}

impl ActionProgramCursor for NestedProgramCursor {
    fn finish_stopped(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ProgramCompletion, ExecutionError> {
        let Self {
            active,
            mut parents,
            mut completed_facts,
            ..
        } = *self;
        let mut completed = active.finish_stopped(game, ctx)?;
        while let Some(mut parent) = parents.pop() {
            completed_facts.extend(completed.facts);
            parent
                .cursor
                .accept_action_with_context(game, ctx, completed.outputs)?;
            let mut outer = None;
            for ancestor in &parents {
                let mut scope = ancestor.action.scope.clone();
                scope.prepend_outer(outer);
                outer = Some(Box::new(scope));
            }
            let mut scope = parent.action.scope;
            scope.prepend_outer(outer);
            completed = scope.run(game, ctx, |game, ctx| {
                parent.cursor.finish_stopped(game, ctx)
            })?;
        }
        completed_facts.extend(completed.facts);
        completed.facts = completed_facts;
        completed.outputs.projections_complete = false;
        Ok(completed)
    }
    fn take_preparations(&mut self) -> Vec<ProgramPreparation> {
        let mut preparations = std::mem::take(&mut self.preparations);
        preparations.extend(self.active.take_preparations());
        preparations
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        loop {
            self.preparations.extend(self.active.take_preparations());
            // A declaration yields a preparation barrier, not completion. Do
            // not enter a body or another offer before this claim is recorded.
            if !self.preparations.is_empty() {
                return Ok(None);
            }
            let scope = ProgramActionScope {
                outer: self.parent_scope(),
                ..Default::default()
            };
            let next = scope.run(game, ctx, |game, ctx| self.active.next_action(game, ctx))?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            if let Some(mut action) = next {
                let inherited_purpose =
                    scope.execution_purpose(crate::effects::EffectExecutionPurpose::Action);
                let purpose = action.execution_purpose(inherited_purpose);
                // Prepared payment decorators own nominal acknowledgement and
                // binding exports; retain that owner rather than descending an
                // ordinary action program and losing its payment lifecycle.
                if action.native.is_none()
                    && matches!(purpose, crate::effects::EffectExecutionPurpose::Action)
                    && action.effect.0.supports_prepared_action_program()
                {
                    let selected = scope.run(game, ctx, |game, ctx| {
                        action.scope.run(game, ctx, |game, ctx| {
                            action.effect.select_prepared_action_program(game, ctx)
                        })
                    })?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    let selected = selected.ok_or_else(|| {
                        ExecutionError::InternalError(
                            "completed nested program selection has no cursor".into(),
                        )
                    })?;
                    let parent = std::mem::replace(&mut self.active, selected);
                    self.parents.push(NestedProgramParent {
                        cursor: parent,
                        action,
                    });
                    continue;
                }
                action.identity = self.child_identity(&action.identity);
                action.scope.prepend_outer(self.parent_scope());
                return Ok(Some(action));
            }
            let Some(parent) = self.parents.pop() else {
                return Ok(None);
            };
            let child = std::mem::replace(&mut self.active, parent.cursor);
            let mut completed = child.finish()?;
            let mut parent_scope = parent.action.scope;
            parent_scope.prepend_outer(self.parent_scope());
            parent_scope.run(game, ctx, |game, ctx| {
                crate::effects::outcome_recording::complete_outcome(
                    game,
                    parent.action.effect.0.result_action(),
                    Some(ctx.controller),
                    &mut completed.outputs.outcome,
                    Vec::new(),
                );
                completed.outputs.synchronize_observations();
                Ok(())
            })?;
            self.completed_facts.extend(completed.facts);
            self.active
                .accept_action_with_context(game, ctx, completed.outputs)?;
        }
    }
    fn accept_action(&mut self, outputs: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        self.active.accept_action(outputs)
    }
    fn accept_action_with_context(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outputs: CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.active.accept_action_with_context(game, ctx, outputs)
    }

    fn ends_action_unit(&self) -> bool {
        self.active.ends_action_unit()
    }
    fn continues_past_illegal_targets(&self) -> bool {
        self.active.continues_past_illegal_targets()
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        if !self.parents.is_empty() {
            return Err(ExecutionError::InternalError(
                "unfinished nested program".into(),
            ));
        }
        let mut completed = self.active.finish()?;
        let mut facts = self.completed_facts;
        facts.extend(completed.facts);
        completed.facts = facts;
        Ok(completed)
    }
}

pub(crate) struct ProgramParticipantResult {
    pub(crate) context: ExecutionContextCheckpoint,
    pub(crate) outputs: CompletedEffectOutputs,
}

pub(crate) struct CompletedActionPrograms {
    pub(crate) participants: Vec<ProgramParticipantResult>,
    /// Physical owners shared by participant binding views, retained once.
    pub(crate) shared: Vec<CompletedEffectOutputs>,
    pub(crate) events: Vec<crate::events::RawEvent>,
    pub(crate) facts: Vec<ExecutionFact>,
}

/// The actual authored declaration and its nominal resource inputs stay with
/// the root through physical commitment, phase completion and acknowledgement.
struct ProgramInstructionDeclaration {
    action: ProgramAction,
    resources: ProgramResourceDeclarations,
}

/// Capture both existing declaration boundaries. Sealing may finish selection
/// of an owner, so the pre-seal set cannot substitute for the sealed set. These
/// values reserve no game state and do not acknowledge a payment.
#[derive(Default)]
struct ProgramResourceDeclarations {
    prepared: Vec<crate::effects::PaymentResourceClaim>,
    sealed: Vec<crate::effects::PaymentResourceClaim>,
}

struct PreparedProgramAction {
    participant: usize,
    declaration: ProgramInstructionDeclaration,
    proposal: Box<dyn SimultaneousEffectProposal>,
}

/// Scoped observers retain the exact original participant frame. Completion
/// itself runs in the participant frame selected by the program coordinator.
struct ProgramOriginalObserver {
    context: ExecutionContextCheckpoint,
    scope: ProgramActionScope,
    inner: Box<dyn SimultaneousEffectCompletion>,
}

impl ProgramOriginalObserver {
    /// Transfer the actual continuation in its captured participant frame.
    /// Every original/draw phase uses this boundary; the request selects only
    /// the child phase, without changing its scalar or retained-packet contract.
    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: crate::effects::composition::CompletionPhase,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let Self {
            context,
            scope,
            inner,
        } = *self;
        let parent = ExecutionContextCheckpoint::capture(ctx);
        context.restore_ref(ctx);
        let result = scope
            .run(game, ctx, |game, ctx| phase.dispatch(inner, game, ctx))
            .map(|mut receipt| {
                receipt.completion = receipt.completion.map(|inner| {
                    Box::new(Self {
                        context: ExecutionContextCheckpoint::capture(ctx),
                        scope,
                        inner,
                    }) as Box<dyn SimultaneousEffectCompletion>
                });
                receipt
            });
        // The participant owns successful prefix result/tag writes. Its caller
        // captures them before another participant runs; failed attempts unwind.
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            parent.restore(ctx);
        }
        result
    }

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        original.dispatch(self.inner, game, ctx)
    }
}

impl SimultaneousEffectCompletion for ProgramOriginalObserver {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutcome(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutputs(original),
        )
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutcome(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutputs(original),
        )
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }
    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        let mut local = self.context.reborrow(&mut *ctx.decision_maker);
        self.scope.run(game, &mut local, |game, ctx| {
            self.inner.observe_original(game, ctx, original)
        })
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.finish(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outcome(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.finish(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outputs(original),
        )
    }
}

struct CompletedProgramOriginal {
    participant: usize,
    scope: ProgramActionScope,
    receipt: SimultaneousEffectCommit<CompletedEffectOutputs>,
    original_bindings: ProgramOriginalBindings,
}

/// Actual sealed physical owners plus their original logical binding views.
/// This value stays inside the containing observation and transaction scope.
struct SealedProgramCohort {
    owners: Vec<PreparedProgramOwner>,
}

struct PreparedProgramOwner {
    participant: usize,
    scope: ProgramActionScope,
    original: SealedProgramOriginal,
}

/// Keep the actual yielded declaration with its sealed proposal until the
/// cohort chooses physical commitment or expansion. A shared damage action
/// retains every logical declaration under its one physical proposal.
enum SealedProgramOriginal {
    Instruction(PreparedProgramAction),
    SharedDamage {
        proposal: Box<dyn SimultaneousEffectProposal>,
        bindings: Vec<PreparedProgramAction>,
    },
}
impl SealedProgramOriginal {
    fn capture_sealed_resource_declarations(&mut self) {
        let actions: &mut [PreparedProgramAction] = match self {
            Self::Instruction(action) => std::slice::from_mut(action),
            Self::SharedDamage { bindings, .. } => bindings,
        };
        for action in actions {
            action.declaration.resources.sealed = action.proposal.declared_payment_resources();
        }
    }

    fn sealed_resource_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::effects::PaymentResourceClaim> {
        let actions: &[PreparedProgramAction] = match self {
            Self::Instruction(action) => std::slice::from_ref(action),
            Self::SharedDamage { bindings, .. } => bindings,
        };
        actions
            .iter()
            .flat_map(|action| &action.declaration.resources.sealed)
    }

    /// Consume the physical proposal once, retaining its actual declarations
    /// through completion and cursor acknowledgement. Shared physical actions
    /// also retain the proposals that publish their logical result views.
    fn into_physical(self) -> (Box<dyn SimultaneousEffectProposal>, ProgramOriginalBindings) {
        match self {
            Self::Instruction(action) => (
                action.proposal,
                ProgramOriginalBindings::Instruction(action.declaration),
            ),
            Self::SharedDamage { proposal, bindings } => {
                (proposal, ProgramOriginalBindings::SharedDamage(bindings))
            }
        }
    }
}

/// The physical proposal and its declaration have different consumers. The
/// declaration's effect, authored scope and identity remain owned until
/// the enclosing cursor acknowledges its actual result; no phase reconstructs
/// them from an output packet or a nominal event.
enum ProgramOriginalBindings {
    Instruction(ProgramInstructionDeclaration),
    SharedDamage(Vec<PreparedProgramAction>),
}

/// Actual owner completion and still-unconsumed logical declarations. This
/// transfer neither repeats the owner nor infers bindings from an aggregate.
struct CompletedProgramOwnerOutputs {
    participant: usize,
    outputs: CompletedEffectOutputs,
    original_bindings: ProgramOriginalBindings,
}

struct CompletedProgramUnit {
    owner: Option<CompletedEffectOutputs>,
    bindings: Vec<(usize, CompletedEffectOutputs)>,
    /// Actual ordinary/shared logical declarations stay alive through every
    /// acknowledgement in this unit, including context capture and unwind.
    declarations: Vec<ProgramInstructionDeclaration>,
}

/// A shared physical action requires the same explicit authored child identity.
/// Empty identities do not prove joinability. Its position is the first
/// contributor's APNAP slot, preserving sealing and one-shot prevention order.
fn program_original_groups(actions: Vec<PreparedProgramAction>) -> Vec<Vec<PreparedProgramAction>> {
    let mut groups: Vec<Vec<PreparedProgramAction>> = Vec::new();
    for action in actions {
        if !action.declaration.action.identity.is_empty()
            && action.proposal.damage_action_inputs().is_some()
        {
            if let Some(group) = groups.iter_mut().find(|group| {
                group[0].declaration.action.identity == action.declaration.action.identity
                    && group[0].proposal.damage_action_inputs().is_some()
            }) {
                group.push(action);
                continue;
            }
        }
        groups.push(vec![action]);
    }
    groups
}

/// The caller owns rollback of program selection and all units. This owner
/// preserves APNAP frames and uses the existing freeze/observe/completion
/// boundary for each wave; it never acknowledges a partially finished parent.
pub(crate) enum ActionProgramsProgress {
    Complete(CompletedActionPrograms),
    Paused {
        prefix: CompletedEffectOutputs,
        continuation: ActionProgramsContinuation,
    },
}

/// The selected cursors and every committed original remain with their owner;
/// resume never reconstructs a program from an aggregate result or rereads inputs.
pub(crate) struct ActionProgramsContinuation {
    participants: Vec<ProgramParticipant>,
    events: Vec<crate::events::RawEvent>,
    facts: Vec<ExecutionFact>,
    shared: Vec<CompletedEffectOutputs>,
    pending_originals: Option<Vec<CompletedProgramOriginal>>,
    /// Alternative views of actual completed children. The selected cursors
    /// remain authoritative; these never replay actions or append history.
    retained_prefix: Vec<CompletedEffectOutputs>,
    capture_between_units: bool,
}
impl ActionProgramsContinuation {
    pub(crate) fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(originals) = &mut self.pending_originals {
            game.freeze_completed_entry_events(
                originals
                    .iter_mut()
                    .flat_map(|original| original.receipt.outcome.outcome.events.iter_mut()),
            )?;
            for original in originals {
                if let Some(completion) = &mut original.receipt.completion {
                    completion.freeze(game)?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        super::inherit_observed_events(&mut self.events, observed);
        if let Some(originals) = &mut self.pending_originals {
            for original in originals {
                super::inherit_original_observations(
                    &mut original.receipt.outcome.outcome,
                    observed,
                );
                original.receipt.outcome.synchronize_observations();
            }
        }
        for outputs in &mut self.shared {
            super::inherit_original_observations(&mut outputs.outcome, observed);
            outputs.synchronize_observations();
        }
        for outputs in &mut self.retained_prefix {
            super::inherit_original_observations(&mut outputs.outcome, observed);
            outputs.synchronize_observations();
        }
    }
    pub(crate) fn resume(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<CompletedActionPrograms>, ExecutionError> {
        match run_action_programs(game, ctx, self, false)? {
            Some(ActionProgramsProgress::Complete(completed)) => Ok(Some(completed)),
            None => Ok(None),
            Some(ActionProgramsProgress::Paused { .. }) => Err(ExecutionError::InternalError(
                "resumed selected programs paused twice".into(),
            )),
        }
    }
}

pub(crate) fn execute_action_programs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: Vec<ProgramParticipant>,
    defer_draws: bool,
) -> Result<Option<ActionProgramsProgress>, ExecutionError> {
    run_action_programs(
        game,
        ctx,
        ActionProgramsContinuation {
            participants,
            events: Vec::new(),
            facts: Vec::new(),
            shared: Vec::new(),
            pending_originals: None,
            retained_prefix: Vec::new(),
            capture_between_units: defer_draws,
        },
        defer_draws,
    )
}

fn finish_stopped_participants(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    facts: &mut Vec<ExecutionFact>,
) -> Result<(), ExecutionError> {
    ctx.stop_resolution();
    for participant in participants {
        let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
        local.stop_resolution();
        if let Some(cursor) = participant.frame.program.take() {
            let completed = cursor.finish_stopped(game, &mut local)?;
            facts.extend(completed.facts);
            participant.result = Some(completed.outputs);
        }
        participant.frame.context = ExecutionContextCheckpoint::capture(&local);
    }
    Ok(())
}

fn run_action_programs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    state: ActionProgramsContinuation,
    defer_draws: bool,
) -> Result<Option<ActionProgramsProgress>, ExecutionError> {
    let ActionProgramsContinuation {
        mut participants,
        mut events,
        mut facts,
        mut shared,
        mut pending_originals,
        mut retained_prefix,
        capture_between_units,
    } = state;
    while participants
        .iter()
        .any(|participant| participant.frame.program.is_some())
    {
        if !events.is_empty()
            && (capture_between_units || game.effect_store.per_event_trigger_matching)
        {
            crate::effects::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                events.iter_mut(),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
        }
        let (mut originals, already_observed) = if let Some(originals) = pending_originals.take() {
            (originals, true)
        } else {
            let mut actions = Vec::new();
            let mut ready = vec![false; participants.len()];
            let mut queued_actions: Vec<Option<ProgramAction>> =
                (0..participants.len()).map(|_| None).collect();
            // Retain this unit's prepared originals while selected declarations
            // pause their bodies. All offers in a wave see the original world;
            // their claims are recorded before even a read-only reveal executes.
            while ready.iter().any(|ready| !ready) {
                let mut preparations = Vec::new();
                for (index, participant) in participants.iter_mut().enumerate() {
                    if ready[index] {
                        continue;
                    }
                    if participant.frame.program.is_none() {
                        ready[index] = true;
                        continue;
                    }
                    let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
                    while let Some(cursor) = &mut participant.frame.program {
                        let next = if let Some(action) = queued_actions[index].take() {
                            Some(action)
                        } else {
                            cursor.next_action(game, &mut local)?
                        };
                        let declared = cursor.take_preparations();
                        if local.decision_maker.awaiting_choice() {
                            return Ok(None);
                        }
                        if !declared.is_empty() {
                            preparations.extend(declared);
                            // Retain a yielded action, if any, while all other actors
                            // select their offers. A None with declarations is a
                            // barrier; retry traversal after preparing those claims.
                            queued_actions[index] = next;
                            break;
                        }
                        let Some(mut action) = next else {
                            let completed = participant
                                .frame
                                .program
                                .take()
                                .expect("active cursor")
                                .finish()?;
                            facts.extend(completed.facts);
                            participant.result = Some(completed.outputs);
                            ready[index] = true;
                            break;
                        };
                        let purpose = action
                            .execution_purpose(crate::effects::EffectExecutionPurpose::Action);
                        if action.native.is_none()
                            && action.effect.0.is_read_only_simultaneous_player_action()
                        {
                            let result = action.execute_with_outputs(game, &mut local, purpose);
                            let outputs = map_program_action_result_for_purpose(
                                cursor.as_ref(),
                                purpose,
                                result,
                            )?;
                            if local.decision_maker.awaiting_choice() {
                                return Ok(None);
                            }
                            events.extend(outputs.outcome.events.clone());
                            facts.extend(outputs.outcome.execution_facts.clone());
                            if capture_between_units {
                                retained_prefix.push(outputs.clone_projection());
                            }
                            cursor.accept_action_with_context(game, &mut local, outputs)?;
                            if local.resolution_stopped() || cursor.ends_action_unit() {
                                ready[index] = true;
                                break;
                            }
                            continue;
                        }
                        let proposal = match action
                            .prepare_original_owner(game, &mut local, purpose)
                        {
                            Ok(Some(proposal)) => proposal,
                            Ok(None) if local.decision_maker.awaiting_choice() => return Ok(None),
                            Ok(None) => {
                                return Err(ExecutionError::InternalError(
                                    "selected program action has no prepared owner".into(),
                                ));
                            }
                            Err(error) => Box::new(FinishedProgramAction {
                                outputs: map_program_action_result_for_purpose(
                                    cursor.as_ref(),
                                    purpose,
                                    Err(error),
                                )?,
                            }),
                        };
                        actions.push(PreparedProgramAction {
                            participant: index,
                            declaration: ProgramInstructionDeclaration {
                                action,
                                resources: ProgramResourceDeclarations::default(),
                            },
                            proposal,
                        });
                        ready[index] = true;
                        break;
                    }
                    if local.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    participant.frame.context = ExecutionContextCheckpoint::capture(&local);
                    let stopped = local.resolution_stopped();
                    drop(local);
                    if stopped {
                        ctx.stop_resolution();
                        break;
                    }
                }
                if ctx.resolution_stopped() {
                    break;
                }
                for preparation in preparations {
                    preparation.prepare(game)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                }
            }
            if ctx.resolution_stopped() {
                finish_stopped_participants(game, ctx, &mut participants, &mut facts)?;
                break;
            }
            // A paused earlier participant can become ready after a later one.
            // Restore APNAP order before original preparation/sealing/cohorting.
            actions.sort_by_key(|action| action.participant);
            if actions.is_empty() {
                continue;
            }
            for action in &mut actions {
                let participant = &mut participants[action.participant];
                let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
                action
                    .declaration
                    .action
                    .scope
                    .run(game, &mut local, |game, ctx| {
                        action.proposal.prepare_selection(game, ctx)
                    })?;
                if local.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                participant.frame.context = ExecutionContextCheckpoint::capture(&local);
                let stopped = local.resolution_stopped();
                drop(local);
                if stopped {
                    ctx.stop_resolution();
                    break;
                }
            }
            if ctx.resolution_stopped() {
                finish_stopped_participants(game, ctx, &mut participants, &mut facts)?;
                break;
            }
            for action in &mut actions {
                let participant = &mut participants[action.participant];
                let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
                action
                    .declaration
                    .action
                    .scope
                    .run(game, &mut local, |game, ctx| {
                        action.proposal.prepare_original(game, ctx)
                    })?;
                if local.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                participant.frame.context = ExecutionContextCheckpoint::capture(&local);
                let stopped = local.resolution_stopped();
                drop(local);
                if stopped {
                    ctx.stop_resolution();
                    break;
                }
            }
            if ctx.resolution_stopped() {
                finish_stopped_participants(game, ctx, &mut participants, &mut facts)?;
                break;
            }
            // Capture every proposal at the original budget boundary before
            // checking any claim. Keep actual claims with their declarations.
            for action in &mut actions {
                action.declaration.resources.prepared =
                    action.proposal.declared_payment_resources();
            }
            if !crate::effects::can_pay_declared_resource_claims(
                game,
                actions
                    .iter()
                    .flat_map(|action| &action.declaration.resources.prepared),
            ) {
                return Err(ExecutionError::Impossible(
                    "program unit exceeds shared resources".into(),
                ));
            }
            let simultaneous = actions.len() > 1
                || actions
                    .iter()
                    .any(|action| action.proposal.has_simultaneous_originals());
            let opened = simultaneous && game.open_simultaneous_action();
            let pinned = simultaneous
                && crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
            let originals = commit_program_originals(game, ctx, &mut participants, actions);
            crate::effects::helpers::end_simultaneous_zone_change_lookback(game, pinned);
            game.close_simultaneous_action(opened);
            let originals = originals?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            (originals, false)
        };
        if !already_observed {
            originals = super::simultaneous::prepare_simultaneous_originals_with_participants(
                game,
                ctx,
                originals,
                |original| &mut original.receipt,
                |game, ctx, originals| {
                    game.observe_prepared_life_payment_originals(
                        ctx,
                        originals.iter_mut().flat_map(|original| {
                            original.receipt.outcome.outcome.events.iter_mut()
                        }),
                    )
                },
            )?;
        }
        if defer_draws {
            let mut boundary = false;
            for original in &mut originals {
                if boundary {
                    break;
                }
                let Some(completion) = original.receipt.completion.take() else {
                    continue;
                };
                let participant = &mut participants[original.participant];
                let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
                let prior = std::mem::replace(
                    &mut original.receipt.outcome,
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()),
                );
                let prepared = original.scope.run(game, &mut local, |game, ctx| {
                    completion.prepare_draw_boundary_from_outputs(game, ctx, prior)
                })?;
                participant.frame.context = ExecutionContextCheckpoint::capture(&local);
                original.receipt = prepared;
                boundary = original.receipt.completion.is_some();
                if local.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                let stopped = local.resolution_stopped();
                drop(local);
                if stopped {
                    ctx.stop_resolution();
                    boundary = false;
                    break;
                }
            }
            if boundary {
                let mut prefix = EffectOutcome::resolved()
                    .with_events(events.clone())
                    .with_execution_facts(facts.clone());
                for original in &originals {
                    prefix = EffectOutcome::aggregate([
                        prefix,
                        original.receipt.outcome.outcome.clone(),
                    ]);
                }
                let mut prefix_outputs = CompletedEffectOutputs::aggregate_only(prefix);
                prefix_outputs.retain_batch_children(
                    retained_prefix
                        .iter()
                        .map(CompletedEffectOutputs::clone_projection),
                );
                prefix_outputs.retain_batch_children(
                    shared.iter().map(CompletedEffectOutputs::clone_projection),
                );
                prefix_outputs.retain_batch_children(
                    participants
                        .iter()
                        .filter_map(|participant| participant.result.as_ref())
                        .map(CompletedEffectOutputs::clone_projection),
                );
                prefix_outputs.retain_batch_children(
                    originals
                        .iter()
                        .map(|original| original.receipt.outcome.clone_projection()),
                );
                return Ok(Some(ActionProgramsProgress::Paused {
                    prefix: prefix_outputs,
                    continuation: ActionProgramsContinuation {
                        participants,
                        events,
                        facts,
                        shared,
                        pending_originals: Some(originals),
                        retained_prefix,
                        capture_between_units,
                    },
                }));
            }
        }
        if super::simultaneous::original_cohort_phase_status_with_participants(
            &mut originals,
            |original| &mut original.receipt,
        ) != crate::effects::OriginalPhaseStatus::Combined
        {
            let Some(phased) =
                super::simultaneous::complete_original_cohort_phase_with_participants(
                    game,
                    ctx,
                    originals,
                    |original| &mut original.receipt,
                    |game, ctx, original| {
                        phase_program_original(game, ctx, &mut participants, original)
                    },
                )?
            else {
                return Ok(None);
            };
            originals = phased;
        }
        let mut completed = Vec::with_capacity(originals.len());
        for original in originals {
            completed.push(complete_program_original(
                game,
                ctx,
                &mut participants,
                original,
            )?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        for unit in completed {
            let declarations = unit.declarations;
            let has_shared_owner = unit.owner.is_some();
            if let Some(owner) = unit.owner {
                events.extend(owner.outcome.events.clone());
                facts.extend(owner.outcome.execution_facts.clone());
                shared.push(owner);
            } else {
                for (_, outputs) in &unit.bindings {
                    events.extend(outputs.outcome.events.clone());
                    facts.extend(outputs.outcome.execution_facts.clone());
                }
            }
            for (index, outputs) in unit.bindings {
                if capture_between_units && !has_shared_owner {
                    retained_prefix.push(outputs.clone_projection());
                }
                let participant = &mut participants[index];
                let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
                let result = participant
                    .frame
                    .program
                    .as_mut()
                    .expect("unfinished program")
                    .accept_action_with_context(game, &mut local, outputs);
                participant.frame.context = ExecutionContextCheckpoint::capture(&local);
                let stopped = local.resolution_stopped();
                drop(local);
                if stopped {
                    ctx.stop_resolution();
                }
                result?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
            }
            // Acknowledgement consumed each packet in its participant frame.
            // Only now can its actual selected declaration leave this owner.
            drop(declarations);
        }
        if ctx.resolution_stopped() {
            finish_stopped_participants(game, ctx, &mut participants, &mut facts)?;
            break;
        }
    }
    Ok(Some(ActionProgramsProgress::Complete(
        CompletedActionPrograms {
            participants: participants
                .into_iter()
                .map(|participant| {
                    Ok(ProgramParticipantResult {
                        context: participant.frame.context,
                        outputs: participant.result.ok_or_else(|| {
                            ExecutionError::InternalError("program lost completed outputs".into())
                        })?,
                    })
                })
                .collect::<Result<_, ExecutionError>>()?,
            events,
            facts,
            shared,
        },
    )))
}

fn commit_program_originals(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    actions: Vec<PreparedProgramAction>,
) -> Result<Vec<CompletedProgramOriginal>, ExecutionError> {
    let (mut originals, observations) = crate::effects::with_action_observations(game, |game| {
        commit_program_originals_inner(game, ctx, participants, actions)
    })?;
    super::original_observations::retain_original_observations(
        originals.iter_mut().map(|original| &mut original.receipt),
        observations,
    );
    Ok(originals)
}

fn commit_program_originals_inner(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    actions: Vec<PreparedProgramAction>,
) -> Result<Vec<CompletedProgramOriginal>, ExecutionError> {
    let Some(cohort) = seal_program_originals(game, ctx, participants, actions)? else {
        return Ok(Vec::new());
    };
    commit_sealed_program_originals(game, ctx, participants, cohort)
}

/// Seal the entire cohort and validate its nominal resource claims before any
/// physical original commits. Suspension returns no admitted cohort. The outer
/// observation/transaction scope owns this phase and the following commitment.
fn seal_program_originals(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    actions: Vec<PreparedProgramAction>,
) -> Result<Option<SealedProgramCohort>, ExecutionError> {
    let mut owners = Vec::new();
    for mut group in program_original_groups(actions) {
        let index = group[0].participant;
        let scope = group[0].declaration.action.scope.clone();
        let participant = &mut participants[index];
        let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
        let shared_damage = !group[0].declaration.action.identity.is_empty()
            && group[0].proposal.damage_action_inputs().is_some();
        let original = if shared_damage {
            let inputs = crate::effects::damage::DamageActionInputs::collect(
                group
                    .iter()
                    .map(|action| action.proposal.damage_action_inputs()),
            )
            .ok_or_else(|| {
                ExecutionError::InternalError(
                    "program damage cohort lost its captured inputs".into(),
                )
            })?;
            // Logical proposals remain unsealed: the physical owner consumes
            // replacement and prevention resources exactly once for the cohort.
            let proposal = scope.run(game, &mut local, |game, ctx| inputs.seal(game, ctx))?;
            SealedProgramOriginal::SharedDamage {
                proposal,
                bindings: group,
            }
        } else {
            let mut action = group.pop().expect("nonempty original group");
            scope.run(game, &mut local, |game, ctx| {
                action.proposal.seal_original(game, ctx)
            })?;
            SealedProgramOriginal::Instruction(action)
        };
        if local.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        participant.frame.context = ExecutionContextCheckpoint::capture(&local);
        let stopped = local.resolution_stopped();
        drop(local);
        if stopped {
            ctx.stop_resolution();
        }
        owners.push(PreparedProgramOwner {
            participant: index,
            scope,
            original,
        });
    }
    // Preserve the separate post-seal declaration boundary and logical owner
    // order, including unsealed binding proposals under a shared damage owner.
    for owner in &mut owners {
        owner.original.capture_sealed_resource_declarations();
    }
    if !crate::effects::can_pay_declared_resource_claims(
        game,
        owners
            .iter()
            .flat_map(|owner| owner.original.sealed_resource_declarations()),
    ) {
        return Err(ExecutionError::Impossible(
            "sealed program unit exceeds shared resources".into(),
        ));
    }
    Ok(Some(SealedProgramCohort { owners }))
}

/// Consume the admitted physical owners once, retaining their actual packets
/// and participant bindings. Selection, sealing and affordability belong to
/// the preceding cohort owner; this driver does not repeat those phases.
fn commit_sealed_program_originals(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    cohort: SealedProgramCohort,
) -> Result<Vec<CompletedProgramOriginal>, ExecutionError> {
    let mut originals = Vec::new();
    for owner in cohort.owners {
        let (proposal, original_bindings) = owner.original.into_physical();
        let participant = &mut participants[owner.participant];
        let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
        let mut receipt = owner.scope.run(game, &mut local, |game, ctx| {
            proposal.commit_original_with_outputs(game, ctx)
        })?;
        participant.frame.context = ExecutionContextCheckpoint::capture(&local);
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(ProgramOriginalObserver {
                context: ExecutionContextCheckpoint::capture(&local),
                scope: owner.scope.clone(),
                inner,
            }));
        }
        originals.push(CompletedProgramOriginal {
            participant: owner.participant,
            scope: owner.scope,
            receipt,
            original_bindings,
        });
        if local.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        let stopped = local.resolution_stopped();
        drop(local);
        if stopped {
            ctx.stop_resolution();
        }
    }
    Ok(originals)
}

/// Advance the retained original in its actual participant frame. Capturing
/// that frame here preserves writes from an original with no remaining child.
fn phase_program_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    original: CompletedProgramOriginal,
) -> Result<CompletedProgramOriginal, ExecutionError> {
    let CompletedProgramOriginal {
        participant: index,
        scope,
        receipt,
        original_bindings,
    } = original;
    let participant = &mut participants[index];
    let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
    let mut receipt = receipt;
    if local.resolution_stopped() {
        receipt.completion = None;
    } else {
        receipt = scope.run(game, &mut local, |game, ctx| {
            super::simultaneous::complete_retained_original_phase_with_outputs(game, ctx, receipt)
        })?;
    }
    participant.frame.context = ExecutionContextCheckpoint::capture(&local);
    let stopped = local.resolution_stopped();
    drop(local);
    if stopped {
        ctx.stop_resolution();
    }
    Ok(CompletedProgramOriginal {
        participant: index,
        scope,
        receipt,
        original_bindings,
    })
}

fn complete_program_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    original: CompletedProgramOriginal,
) -> Result<CompletedProgramUnit, ExecutionError> {
    let Some(completed) = complete_program_original_outputs(game, ctx, participants, original)?
    else {
        return Ok(CompletedProgramUnit {
            owner: None,
            bindings: Vec::new(),
            declarations: Vec::new(),
        });
    };
    bind_program_original_outputs(game, ctx, participants, completed)
}

/// Finish one actual owner in its retained participant frame. Result bindings
/// stay with the completed packet until their enclosing parent is ready.
fn complete_program_original_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    original: CompletedProgramOriginal,
) -> Result<Option<CompletedProgramOwnerOutputs>, ExecutionError> {
    let stopped = ctx.resolution_stopped();
    let participant = &mut participants[original.participant];
    let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
    if stopped {
        local.stop_resolution();
    }
    let outputs = if stopped {
        original.receipt.outcome
    } else {
        original.scope.run(game, &mut local, |game, ctx| {
            super::simultaneous::complete_committed_original_with_outputs(
                game,
                ctx,
                original.receipt,
            )
        })?
    };
    participant.frame.context = ExecutionContextCheckpoint::capture(&local);
    if local.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let stopped = local.resolution_stopped();
    drop(local);
    if stopped {
        ctx.stop_resolution();
    }
    Ok(Some(CompletedProgramOwnerOutputs {
        participant: original.participant,
        outputs,
        original_bindings: original.original_bindings,
    }))
}

/// Bind the actual completed packet once. A shared damage owner retains its
/// one physical output while logical declarations contribute only their views.
fn bind_program_original_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    participants: &mut [ProgramParticipant],
    completed: CompletedProgramOwnerOutputs,
) -> Result<CompletedProgramUnit, ExecutionError> {
    let CompletedProgramOwnerOutputs {
        participant: participant_index,
        mut outputs,
        original_bindings,
    } = completed;
    let bindings = match original_bindings {
        ProgramOriginalBindings::Instruction(declaration) => {
            return Ok(CompletedProgramUnit {
                owner: None,
                bindings: vec![(participant_index, outputs)],
                declarations: vec![declaration],
            });
        }
        ProgramOriginalBindings::SharedDamage(bindings) => bindings,
    };
    // Per-participant wrappers publish their genuine result/tag/source bindings
    // from this receipt. They do not commit a second physical damage action.
    let mut rows = Vec::new();
    let mut owned_bindings = Vec::new();
    let mut declarations = Vec::new();
    for action in bindings {
        let participant = &mut participants[action.participant];
        let mut local = participant.frame.context.reborrow(&mut *ctx.decision_maker);
        let binding = action
            .declaration
            .action
            .scope
            .run(game, &mut local, |game, ctx| {
                action.proposal.bind_damage_action(game, ctx, &outputs)
            })?;
        participant.frame.context = ExecutionContextCheckpoint::capture(&local);
        if local.decision_maker.awaiting_choice() {
            return Ok(CompletedProgramUnit {
                owner: None,
                bindings: Vec::new(),
                declarations: Vec::new(),
            });
        }
        rows.push((
            action.participant,
            CompletedEffectOutputs::aggregate_only(binding.outcome.clone()),
        ));
        owned_bindings.push(binding);
        declarations.push(action.declaration);
    }
    crate::effects::DamageActionBinding::from_bindings(owned_bindings, |_| EffectOutcome::count(0))
        .transfer_owned_outputs(&mut outputs);
    Ok(CompletedProgramUnit {
        owner: Some(outputs),
        bindings: rows,
        declarations,
    })
}

fn map_program_action_result_for_purpose(
    cursor: &dyn ActionProgramCursor,
    purpose: crate::effects::EffectExecutionPurpose,
    result: Result<CompletedEffectOutputs, ExecutionError>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    match purpose {
        crate::effects::EffectExecutionPurpose::Action => map_program_action_result(cursor, result),
        crate::effects::EffectExecutionPurpose::Payment => result,
    }
}

fn map_program_action_result(
    cursor: &dyn ActionProgramCursor,
    result: Result<CompletedEffectOutputs, ExecutionError>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    match result {
        Err(ExecutionError::InvalidTarget) if cursor.continues_past_illegal_targets() => Ok(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()),
        ),
        other => other,
    }
}

struct FinishedProgramAction {
    outputs: CompletedEffectOutputs,
}
impl std::fmt::Debug for FinishedProgramAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FinishedProgramAction")
            .finish_non_exhaustive()
    }
}
impl SimultaneousEffectProposal for FinishedProgramAction {
    fn commit_original_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit::finished(self.outputs))
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }
    fn commit(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(self.outputs.into_outcome())
    }
}

/// Ordinary execution uses the same selected cursor and authored scopes. The
/// caller chooses its existing Action/Payment gateway and owns rollback.
pub(super) fn execute_action_program_with_outputs<'cursor>(
    mut cursor: Box<dyn ActionProgramCursor + 'cursor>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    match run_program_cursor(cursor.as_mut(), game, ctx, purpose)? {
        ProgramExecutionEnd::Finished => Ok(cursor.finish()?.outputs),
        ProgramExecutionEnd::Pending => cursor.finish_pending(),
        ProgramExecutionEnd::Stopped => cursor
            .finish_stopped(game, ctx)
            .map(|completed| completed.outputs),
    }
}

/// Traversal and physical child execution have one owner. The caller consumes
/// its actual cursor using its existing result/pending/stopped projection.
pub(super) enum ProgramExecutionEnd {
    Finished,
    Pending,
    Stopped,
}

pub(super) fn run_program_cursor(
    cursor: &mut dyn ActionProgramCursor,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<ProgramExecutionEnd, ExecutionError> {
    loop {
        if ctx.resolution_stopped() {
            return Ok(ProgramExecutionEnd::Stopped);
        }
        let (action_purpose, result) = {
            let selected = cursor.select_execution_instruction(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() && !selected.dispatch_while_pending {
                drop(selected);
                return Ok(ProgramExecutionEnd::Pending);
            }
            let ProgramInstructionSelection {
                instruction,
                preparations,
                ..
            } = selected;
            let declared = !preparations.is_empty();
            for preparation in preparations {
                preparation.prepare(game)?;
            }
            let Some(instruction) = instruction else {
                if declared {
                    continue;
                }
                break;
            };
            instruction.execute(game, ctx, purpose)
        };
        let outputs = map_program_action_result_for_purpose(cursor, action_purpose, result)?;
        cursor.accept_action_with_context(game, ctx, outputs)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(ProgramExecutionEnd::Pending);
        }
    }
    Ok(ProgramExecutionEnd::Finished)
}

/// Existing prepared leaf capabilities determine staged coverage. Containers
/// need a cursor, or an explicit native lifecycle, before they can join. Damage
/// contributions compose their shared physical owner. Sequential draws retain
/// their existing owner until the coordinator forwards that domain contract;
/// this capability query does not force them into a sequential fallback.
pub(super) fn action_program_child_is_prepared(effect: &Effect) -> bool {
    if effect.0.is_read_only_simultaneous_player_action()
        || effect.0.supports_prepared_action_program()
    {
        return true;
    }
    if effect.0.requires_sequential_player_actions()
        || crate::effects::replacement::replacement_effect_contains_draw(effect)
    {
        return false;
    }
    if let Some(child) = effect.0.transparent_child_effect() {
        return effect.0.supports_simultaneous_player_action()
            && action_program_child_is_prepared(child);
    }
    let mut has_children = false;
    effect.0.visit_child_effects(&mut |_| has_children = true);
    !has_children && effect.0.supports_simultaneous_player_action()
}
