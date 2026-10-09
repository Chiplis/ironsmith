//! CR 121.7 replacement-program continuations. A selected branch and every
//! enclosing scope are retained; resumption never reruns the original prefix.
use crate::effect::{Effect, EffectId, EffectOutcome, ExecutionFact, OutcomeValue};
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionContextCheckpoint, ExecutionError,
    SimultaneousEffectCommit, SimultaneousEffectCompletion,
};
use crate::events::processing::ReplacementEventContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

pub(crate) trait ReplacementResume: Send {
    /// Freeze only already committed originals, before any sibling addition.
    /// Future authored instructions stay unevaluated until resumption.
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError>;
    /// Inherit the outer observer's actual receipts; retaining a prefix does
    /// not itself establish that its triggers were matched.
    fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]);

    /// Dispatch body for `resume_replacement_child_with_outputs`. Return the completed
    /// subtree, including its prefix exactly once; the gateway owns rollback
    /// before any enclosing scope catches an error or observes suspension.
    fn resume_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.resume_inner(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::aggregate_only)
    }

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}
pub(crate) struct PreparedReplacementChild {
    pub(crate) prefix: CompletedEffectOutputs,
    pub(crate) resume: Option<Box<dyn ReplacementResume>>,
}
impl PreparedReplacementChild {
    fn finished(prefix: EffectOutcome) -> Self {
        Self {
            prefix: CompletedEffectOutputs::aggregate_only(prefix),
            resume: None,
        }
    }
}
impl PreparedReplacementChild {
    pub(crate) fn finished_with_outputs(prefix: CompletedEffectOutputs) -> Self {
        Self {
            prefix,
            resume: None,
        }
    }
}

/// Own child packets once while preserving the program's existing aggregate projection.
fn compose_prefix_outputs(
    aggregate: EffectOutcome,
    children: Vec<CompletedEffectOutputs>,
) -> CompletedEffectOutputs {
    let complete = !children.is_empty() && children.iter().all(|child| child.projections_complete);
    let mut outputs = CompletedEffectOutputs::aggregate_only(aggregate);
    outputs.projections_complete = complete;
    outputs.retain_batch_children(children);
    outputs
}

/// Retained frames keep their owned children. Their paused prefix is an
/// alternative view of those same facts, with no new chronological events.
fn compose_retained_prefix_outputs(
    aggregate: EffectOutcome,
    before: &[CompletedEffectOutputs],
    selected: &CompletedEffectOutputs,
) -> CompletedEffectOutputs {
    compose_prefix_outputs(
        aggregate,
        before
            .iter()
            .chain(std::iter::once(selected))
            .map(CompletedEffectOutputs::clone_projection)
            .collect(),
    )
}

#[derive(Clone)]
enum Mode {
    Aggregate,
    Sequence { coordinated: bool },
    Optional { has_action: bool },
}
impl Mode {
    fn adjust(&self, effect: &Effect, outcome: &mut EffectOutcome) {
        if matches!(self, Self::Optional { has_action: true })
            && crate::effects::is_object_selection(effect)
        {
            outcome.set_value(OutcomeValue::None);
        }
    }
    fn stops(&self, outcome: &EffectOutcome) -> bool {
        matches!(self, Self::Sequence { coordinated: false }) && outcome.status.is_failure()
    }
    fn finish(&self, outcomes: Vec<EffectOutcome>) -> EffectOutcome {
        match self {
            Self::Sequence { .. } => EffectOutcome::aggregate_terminal(outcomes),
            Self::Optional { .. } => {
                EffectOutcome::aggregate(outcomes).with_execution_fact(ExecutionFact::Accepted)
            }
            Self::Aggregate if outcomes.is_empty() => EffectOutcome::count(0),
            Self::Aggregate => EffectOutcome::aggregate(outcomes),
        }
    }
}

fn life_action(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::PayLifeEffect>()
            .is_some()
}

fn contains_draw(effect: &Effect) -> bool {
    // A nested life event may be replaced by a draw at runtime.
    if life_action(effect) || effect.0.supports_replacement_draw_continuation() {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some()
    {
        return true;
    }
    let mut found = false;
    effect
        .0
        .visit_child_effects(&mut |child| found |= contains_draw(child));
    found
}
pub(crate) fn replacement_effect_supported(effect: &Effect) -> bool {
    if life_action(effect)
        || effect.0.supports_replacement_draw_continuation()
        || !contains_draw(effect)
        || effect
            .downcast_ref::<crate::effects::DrawCardsEffect>()
            .is_some()
    {
        return true;
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
        return repeat.effects.iter().all(replacement_effect_supported);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().all(replacement_effect_supported);
    }
    if let Some(optional) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return optional.effects.iter().all(replacement_effect_supported);
    }
    if let Some(condition) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return condition
            .then
            .iter()
            .chain(&condition.else_)
            .all(replacement_effect_supported);
    }
    if let Some(condition) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return condition
            .if_true
            .iter()
            .chain(&condition.if_false)
            .all(replacement_effect_supported);
    }
    if effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            .is_some()
    {
        return effect
            .0
            .transparent_child_effect()
            .is_some_and(replacement_effect_supported);
    }
    false
}

/// Complete non-draw replacement prefixes in place. The semantic owner decides
/// whether its retained completion has actually reached a draw; syntax alone
/// cannot detect a draw created by an event replacement.
pub(crate) fn prepare_committed_draw_boundary(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: SimultaneousEffectCommit<CompletedEffectOutputs>,
) -> Result<PreparedReplacementChild, ExecutionError> {
    let receipt = prepare_committed_original_draw_with_outputs(game, ctx, receipt)?;
    retain_draw_boundary(receipt, ctx)
}

fn prepare_committed_original_draw_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: SimultaneousEffectCommit<CompletedEffectOutputs>,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    let mut original = receipt.outcome;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SimultaneousEffectCommit::finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    if ctx.resolution_stopped() {
        return Ok(SimultaneousEffectCommit::finished(original));
    }
    let Some(mut completion) = receipt.completion else {
        return Ok(SimultaneousEffectCommit::finished(original));
    };
    crate::effects::composition::prepare_standalone_original_completion(
        game,
        ctx,
        &mut original.outcome,
        completion.as_mut(),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SimultaneousEffectCommit::finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    original.synchronize_observations();
    let prepared = completion.prepare_draw_boundary_from_outputs(game, ctx, original)?;
    Ok(prepared)
}

/// Explicit native owners opt in; capability is never inferred merely from
/// having a prepared proposal or from a preview of deferred child programs.
pub(crate) fn prepare_native_draw_continuation_with_outputs(
    effect: &dyn crate::effects::EffectExecutor,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    prepare_native_proposal_draw_continuation_with_outputs(
        effect.result_action(),
        game,
        ctx,
        |game, ctx| effect.prepare_simultaneous_player_action(game, ctx),
    )
}

/// Native selection can need a mutable world (random choices and hidden pools).
/// Construct its proposal inside the same resource transaction as immutable
/// proposals; recording, original admission and draw preparation have one owner.
pub(crate) fn prepare_native_proposal_draw_continuation_with_outputs<'a>(
    result_action: Option<crate::effect::PriorEffectAction>,
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    prepare: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    )
        -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError>,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    crate::effects::tokens::execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || {
            SimultaneousEffectCommit::finished(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ))
        },
        |game, ctx| {
            let proposal = prepare(game, ctx)?;
            let mut proposal =
                crate::effects::outcome_recording::record_proposal(proposal, result_action);
            proposal.prepare_selection(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            proposal.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            let opened = proposal.has_simultaneous_originals() && game.open_simultaneous_action();
            let result = (|| {
                proposal.seal_original(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(SimultaneousEffectCommit::finished(
                        CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                    ));
                }
                let (mut receipt, observations) =
                    crate::effects::with_action_observations(game, |game| {
                        proposal.commit_original_with_outputs(game, ctx)
                    })?;
                crate::effects::composition::original_observations::retain_original_observations(
                    std::iter::once(&mut receipt),
                    observations,
                );
                Ok::<_, ExecutionError>(receipt)
            })();
            game.close_simultaneous_action(opened);
            prepare_committed_original_draw_with_outputs(game, ctx, result?)
        },
    )
}

pub(crate) fn retain_draw_boundary(
    committed: SimultaneousEffectCommit<CompletedEffectOutputs>,
    ctx: &ExecutionContext,
) -> Result<PreparedReplacementChild, ExecutionError> {
    let Some(completion) = committed.completion else {
        return Ok(PreparedReplacementChild::finished_with_outputs(
            committed.outcome,
        ));
    };
    let prefix = committed.outcome.clone_projection();
    Ok(PreparedReplacementChild {
        prefix,
        resume: Some(Box::new(OriginalActionFrame {
            original: committed.outcome,
            completion,
            context: ExecutionContextCheckpoint::capture(ctx),
            frozen: false,
        })),
    })
}

struct OriginalActionFrame {
    original: crate::effects::CompletedEffectOutputs,
    completion: Box<dyn SimultaneousEffectCompletion>,
    context: ExecutionContextCheckpoint,
    frozen: bool,
}
impl ReplacementResume for OriginalActionFrame {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if !self.frozen {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                &mut self.original.outcome,
                Vec::new(),
            );
            self.original.synchronize_observations();
            self.completion.freeze(game)?;
            self.frozen = true;
        }
        Ok(())
    }
    fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        crate::effects::composition::inherit_original_observations(
            &mut self.original.outcome,
            observed,
        );
    }

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn resume_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.freeze(game)?;
        self.context.restore(ctx);
        crate::effects::composition::observe_original_completion(
            game,
            ctx,
            self.completion.as_mut(),
            &mut self.original.outcome,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::composition::complete_committed_original_with_outputs(
            game,
            ctx,
            SimultaneousEffectCommit {
                outcome: self.original,
                completion: Some(self.completion),
            },
        )
    }
}

struct DrawLeaf {
    context: ExecutionContextCheckpoint,
    effect: Effect,
    prepared: crate::effects::cards::PreparedDrawInstruction,
}
impl ReplacementResume for DrawLeaf {
    fn freeze(&mut self, _game: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn observe_prefix(&mut self, _observed: &[crate::triggers::TriggerEvent]) {}

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn resume_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self {
            context,
            effect,
            prepared,
        } = *self;
        context.restore(ctx);
        // A prepared draw's quantity and actor were acquired when reached.
        let mut prepared = Some(prepared);
        crate::effects::runtime::prepare_effect_original_with_outputs(
            game,
            &effect,
            ctx,
            |_, game, ctx| {
                let prepared = prepared.take().ok_or_else(|| {
                    ExecutionError::InternalError(
                        "prepared draw was consumed before chooser replay".into(),
                    )
                })?;
                crate::effects::cards::execute_prepared_draw_instruction(prepared, game, ctx)
                    .map(CompletedEffectOutputs::aggregate_only)
                    .map(SimultaneousEffectCommit::finished)
            },
        )
        .map(|receipt| receipt.outcome)
    }
}
struct ProgramFrame {
    before: Vec<CompletedEffectOutputs>,
    first: Box<dyn ReplacementResume>,
    first_effect: Effect,
    tail: Vec<Effect>,
    mode: Mode,
}
impl ReplacementResume for ProgramFrame {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        for outcome in &mut self.before {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                &mut outcome.outcome,
                Vec::new(),
            );
        }
        self.first.freeze(game)
    }
    fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        for outcome in &mut self.before {
            crate::effects::composition::inherit_original_observations(
                &mut outcome.outcome,
                observed,
            );
        }
        self.first.observe_prefix(observed);
    }

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn resume_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut outcomes = self
            .before
            .iter()
            .map(|outputs| outputs.outcome.clone())
            .collect::<Vec<_>>();
        let first = resume_replacement_child_with_outputs(game, ctx, self.first)?;
        let aggregate = first.outcome.clone();
        let mut children = self.before;
        children.push(first);
        let mut outputs = compose_prefix_outputs(aggregate, children);
        let mut first = outputs.outcome.clone();
        self.mode.adjust(&self.first_effect, &mut first);
        let stop = self.mode.stops(&first);
        outcomes.push(first);
        if stop || ctx.resolution_stopped() || ctx.decision_maker.awaiting_choice() {
            return Ok(outputs.project_aggregate(self.mode.finish(outcomes)));
        }
        for (index, effect) in self.tail.iter().enumerate() {
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                Some(effect),
                outcomes
                    .iter_mut()
                    .flat_map(|outcome| outcome.events.iter_mut()),
            )?;
            let result = crate::effects::execute_effect_with_outputs(game, effect, ctx);
            let child = match result {
                Err(ExecutionError::InvalidTarget)
                    if matches!(self.mode, Mode::Sequence { coordinated: true }) =>
                {
                    crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    )
                }
                other => other?,
            };
            let mut outcome = child.outcome.clone();
            outputs = outputs.append_owned_child(child);
            self.mode.adjust(effect, &mut outcome);
            let stop = self.mode.stops(&outcome);
            outcomes.push(outcome);
            if stop || ctx.resolution_stopped() || ctx.decision_maker.awaiting_choice() {
                break;
            }
            if index + 1 == self.tail.len() {
                crate::effects::runtime::capture_triggers_before_added_program(
                    game,
                    ctx,
                    None,
                    outcomes
                        .iter_mut()
                        .flat_map(|outcome| outcome.events.iter_mut()),
                )?;
            }
        }
        Ok(outputs.project_aggregate(self.mode.finish(outcomes)))
    }
}

enum Scope {
    Result(EffectId),
    Player(Option<PlayerId>),
    SharedOperations(crate::effects::composition::RepetitionScope),
    Optional {
        player: Option<PlayerId>,
        optional_action: bool,
    },
    Source {
        source: ObjectId,
        snapshot: Option<ObjectSnapshot>,
    },
    Tagged {
        effect: crate::effects::TaggedEffect,
        runtime: crate::effects::TaggedRuntimeState,
    },
    IdentityGuard(Option<crate::effects::context::OptionalIdentityGuard>),
}
impl Scope {
    fn leave(self, game: &mut GameState, ctx: &mut ExecutionContext, outcome: &EffectOutcome) {
        match self {
            Self::Player(player) => ctx.iteration.iterated_player = player,
            Self::SharedOperations(operations) => operations.leave(ctx),
            Self::Result(id) => {
                ctx.effect_outcomes
                    .entry(id)
                    .or_insert_with(|| outcome.clone());
            }
            Self::Optional {
                player,
                optional_action,
            } => {
                ctx.iteration.iterated_player = player;
                ctx.optional_action = optional_action;
            }
            Self::Source { source, snapshot } => {
                ctx.source = source;
                ctx.source_snapshot = snapshot;
            }
            Self::Tagged { effect, runtime } => {
                crate::effects::apply_outcome_tags(&effect, game, ctx, outcome, runtime)
            }
            Self::IdentityGuard(guard) => ctx.optional_identity_guard = guard,
        }
    }
}
struct ScopeFrame {
    scope: Scope,
    inner: Box<dyn ReplacementResume>,
}
impl ReplacementResume for ScopeFrame {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }
    fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        self.inner.observe_prefix(observed);
    }

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn resume_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let outputs = resume_replacement_child_with_outputs(game, ctx, self.inner)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        self.scope.leave(game, ctx, &outputs.outcome);
        Ok(outputs)
    }
}
fn scope_result(
    mut prepared: PreparedReplacementChild,
    scope: Scope,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> PreparedReplacementChild {
    if ctx.decision_maker.awaiting_choice() {
        return PreparedReplacementChild::finished(EffectOutcome::count(0));
    }
    if let Some(inner) = prepared.resume.take() {
        prepared.resume = Some(Box::new(ScopeFrame { scope, inner }));
    } else {
        scope.leave(game, ctx, &prepared.prefix.outcome);
    }
    prepared
}

fn finish_repetitions(outcomes: Vec<EffectOutcome>) -> EffectOutcome {
    crate::effects::composition::finish_repeated_sequence_outcomes(outcomes)
}

struct RepetitionFrame {
    before: Vec<CompletedEffectOutputs>,
    first: Box<dyn ReplacementResume>,
    effects: Vec<Effect>,
    remaining: usize,
}
impl ReplacementResume for RepetitionFrame {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        for outcome in &mut self.before {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                &mut outcome.outcome,
                Vec::new(),
            );
        }
        self.first.freeze(game)
    }
    fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        for outcome in &mut self.before {
            crate::effects::composition::inherit_original_observations(
                &mut outcome.outcome,
                observed,
            );
        }
        self.first.observe_prefix(observed);
    }

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn resume_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut outcomes = self
            .before
            .iter()
            .map(|outputs| outputs.outcome.clone())
            .collect::<Vec<_>>();
        let first = resume_replacement_child_with_outputs(game, ctx, self.first)?;
        let aggregate = first.outcome.clone();
        let mut children = self.before;
        children.push(first);
        let mut outputs = compose_prefix_outputs(aggregate, children);
        let current = outputs.outcome.clone();
        let stop = current.status.is_failure();
        outcomes.push(current);
        if stop || ctx.resolution_stopped() || ctx.decision_maker.awaiting_choice() {
            return Ok(outputs.project_aggregate(finish_repetitions(outcomes)));
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            outcomes
                .iter_mut()
                .flat_map(|outcome| outcome.events.iter_mut()),
        )?;
        if self.remaining > 0 {
            use crate::effects::EffectExecutor;
            let repeated =
                crate::effects::RepeatEffectsEffect::new(self.remaining as i32, self.effects);
            let child = repeated.execute_child_with_outputs(game, ctx)?;
            outcomes.push(child.outcome.clone());
            outputs = outputs.append_owned_child(child);
        }
        Ok(outputs.project_aggregate(finish_repetitions(outcomes)))
    }
}
fn prepare_repetitions(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &crate::effects::RepeatEffectsEffect,
) -> Result<PreparedReplacementChild, ExecutionError> {
    let count = crate::effects::composition::resolve_repeat_count(game, &effect.count, ctx)?;
    let mut outcomes = Vec::new();
    for index in 0..count {
        let operations = crate::effects::composition::RepetitionScope::enter(ctx);
        let prepared = prepare_program(
            game,
            ctx,
            &effect.effects,
            Mode::Sequence { coordinated: false },
            None,
        )?;
        let prepared = scope_result(prepared, Scope::SharedOperations(operations), game, ctx);
        if let Some(first) = prepared.resume {
            let mut prefix = outcomes
                .iter()
                .map(|outputs: &CompletedEffectOutputs| outputs.outcome.clone())
                .collect::<Vec<_>>();
            prefix.push(prepared.prefix.outcome.clone());
            return Ok(PreparedReplacementChild {
                prefix: compose_retained_prefix_outputs(
                    finish_repetitions(prefix),
                    &outcomes,
                    &prepared.prefix,
                ),
                resume: Some(Box::new(RepetitionFrame {
                    before: outcomes,
                    first,
                    effects: effect.effects.clone(),
                    remaining: count - index - 1,
                })),
            });
        }
        let stop = prepared.prefix.outcome.status.is_failure();
        outcomes.push(prepared.prefix);
        if stop || ctx.resolution_stopped() || ctx.decision_maker.awaiting_choice() {
            break;
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            outcomes
                .iter_mut()
                .flat_map(|outcome| outcome.outcome.events.iter_mut()),
        )?;
    }
    let aggregate = finish_repetitions(
        outcomes
            .iter()
            .map(|outputs| outputs.outcome.clone())
            .collect(),
    );
    Ok(PreparedReplacementChild::finished_with_outputs(
        compose_prefix_outputs(aggregate, outcomes),
    ))
}

struct BranchesFrame {
    before: Vec<CompletedEffectOutputs>,
    first: Box<dyn ReplacementResume>,
    rest: Vec<crate::effects::PreparedIfBranch>,
}
impl ReplacementResume for BranchesFrame {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        for outcome in &mut self.before {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                &mut outcome.outcome,
                Vec::new(),
            );
        }
        self.first.freeze(game)
    }
    fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        for outcome in &mut self.before {
            crate::effects::composition::inherit_original_observations(
                &mut outcome.outcome,
                observed,
            );
        }
        self.first.observe_prefix(observed);
    }

    fn resume_inner(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn resume_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut outcomes = self
            .before
            .iter()
            .map(|outputs| outputs.outcome.clone())
            .collect::<Vec<_>>();
        let first = resume_replacement_child_with_outputs(game, ctx, self.first)?;
        let aggregate = first.outcome.clone();
        let mut children = self.before;
        children.push(first);
        let mut outputs = compose_prefix_outputs(aggregate, children);
        outcomes.push(outputs.outcome.clone());
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            outcomes
                .iter_mut()
                .flat_map(|outcome| outcome.events.iter_mut()),
        )?;
        let child = crate::effects::execute_if_branches_with_outputs(game, ctx, &self.rest)?;
        outcomes.push(child.outcome.clone());
        // An empty tail executes no additional owner. Keep the original
        // neutral aggregate entry without downgrading the resumed child's
        // coverage solely because the tail has no projections.
        if !self.rest.is_empty() {
            outputs = outputs.append_owned_child(child);
        }
        Ok(outputs.project_aggregate(EffectOutcome::aggregate(outcomes)))
    }
}
fn prepare_branches(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    branches: Vec<crate::effects::PreparedIfBranch>,
) -> Result<PreparedReplacementChild, ExecutionError> {
    let mut outcomes = Vec::new();
    for (index, branch) in branches.iter().enumerate() {
        for repetition in 0..branch.repetitions {
            let previous = ctx.iteration.iterated_player;
            if let Some(player) = branch.player {
                ctx.iteration.iterated_player = Some(player);
            }
            let prepared = prepare_program(game, ctx, &branch.effects, Mode::Aggregate, None)?;
            let prepared = scope_result(prepared, Scope::Player(previous), game, ctx);
            if let Some(first) = prepared.resume {
                let mut prefix = outcomes
                    .iter()
                    .map(|outputs: &CompletedEffectOutputs| outputs.outcome.clone())
                    .collect::<Vec<_>>();
                prefix.push(prepared.prefix.outcome.clone());
                let mut rest = Vec::new();
                if repetition + 1 < branch.repetitions {
                    let mut remaining = branch.clone();
                    remaining.repetitions -= repetition + 1;
                    rest.push(remaining);
                }
                rest.extend_from_slice(&branches[index + 1..]);
                return Ok(PreparedReplacementChild {
                    prefix: compose_retained_prefix_outputs(
                        EffectOutcome::aggregate(prefix),
                        &outcomes,
                        &prepared.prefix,
                    ),
                    resume: Some(Box::new(BranchesFrame {
                        before: outcomes,
                        first,
                        rest,
                    })),
                });
            }
            outcomes.push(prepared.prefix);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(PreparedReplacementChild::finished(EffectOutcome::count(0)));
            }
        }
    }
    let aggregate = if outcomes.is_empty() {
        EffectOutcome::count(0)
    } else {
        EffectOutcome::aggregate(outcomes.iter().map(|outputs| outputs.outcome.clone()))
    };
    Ok(PreparedReplacementChild::finished_with_outputs(
        compose_prefix_outputs(aggregate, outcomes),
    ))
}

fn prepare_program(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    mode: Mode,
    first_guard: Option<crate::effects::context::OptionalIdentityGuard>,
) -> Result<PreparedReplacementChild, ExecutionError> {
    let mut outcomes = Vec::new();
    for (index, effect) in effects.iter().enumerate() {
        let guard_scope = if index == 0 && first_guard.is_some() {
            Some(Scope::IdentityGuard(std::mem::replace(
                &mut ctx.optional_identity_guard,
                first_guard.clone(),
            )))
        } else {
            None
        };
        let result = prepare_effect(game, ctx, effect);
        let mut prepared = match result {
            Err(ExecutionError::InvalidTarget)
                if matches!(mode, Mode::Sequence { coordinated: true }) =>
            {
                PreparedReplacementChild::finished(EffectOutcome::target_invalid())
            }
            other => other?,
        };
        if let Some(scope) = guard_scope {
            prepared = scope_result(prepared, scope, game, ctx);
        }
        if let Some(first) = prepared.resume {
            let mut prefix = outcomes
                .iter()
                .map(|outputs: &CompletedEffectOutputs| outputs.outcome.clone())
                .collect::<Vec<_>>();
            mode.adjust(effect, &mut prepared.prefix.outcome);
            prefix.push(prepared.prefix.outcome.clone());
            return Ok(PreparedReplacementChild {
                prefix: compose_retained_prefix_outputs(
                    mode.finish(prefix),
                    &outcomes,
                    &prepared.prefix,
                ),
                resume: Some(Box::new(ProgramFrame {
                    before: outcomes,
                    first,
                    first_effect: effect.clone(),
                    tail: effects[index + 1..].to_vec(),
                    mode,
                })),
            });
        }
        mode.adjust(effect, &mut prepared.prefix.outcome);
        let stop = mode.stops(&prepared.prefix.outcome);
        outcomes.push(prepared.prefix);
        if stop || ctx.resolution_stopped() || ctx.decision_maker.awaiting_choice() {
            break;
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            effects.get(index + 1),
            outcomes
                .iter_mut()
                .flat_map(|outcome| outcome.outcome.events.iter_mut()),
        )?;
    }
    let aggregate = mode.finish(
        outcomes
            .iter()
            .map(|outputs| outputs.outcome.clone())
            .collect(),
    );
    Ok(PreparedReplacementChild::finished_with_outputs(
        compose_prefix_outputs(aggregate, outcomes),
    ))
}
fn prepare_effect(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &Effect,
) -> Result<PreparedReplacementChild, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || PreparedReplacementChild::finished(EffectOutcome::count(0)),
        |game, ctx| prepare_effect_inner(game, ctx, effect),
    )
}

fn prepare_effect_inner(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &Effect,
) -> Result<PreparedReplacementChild, ExecutionError> {
    if ctx.resolution_stopped()
        || game.turn_store.end_turn_procedure_pending
        || game.turn_store.end_combat_phase_procedure_pending
    {
        return crate::effects::execute_effect_with_outputs(game, effect, ctx)
            .map(PreparedReplacementChild::finished_with_outputs);
    }
    if effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some()
    {
        let mut retained = None;
        let committed = crate::effects::runtime::prepare_effect_original_with_outputs(
            game,
            effect,
            ctx,
            |effect, game, ctx| {
                let draw = effect
                    .downcast_ref::<crate::effects::DrawCardsEffect>()
                    .expect("native draw instruction");
                let prepared = crate::effects::cards::prepare_draw_instruction(draw, game, ctx)?;
                // Dynamic zero has no boundary; its suffix remains an immediate prefix.
                if prepared.requested_count == 0 {
                    return crate::effects::cards::execute_prepared_draw_instruction(
                        prepared, game, ctx,
                    )
                    .map(CompletedEffectOutputs::aggregate_only)
                    .map(SimultaneousEffectCommit::finished);
                }
                retained = Some(prepared);
                Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ))
            },
        )?;
        if ctx.decision_maker.awaiting_choice() {
            retained = None;
        }
        let resume = retained.map(|prepared| {
            Box::new(DrawLeaf {
                context: ExecutionContextCheckpoint::capture(ctx),
                effect: effect.clone(),
                prepared,
            }) as Box<dyn ReplacementResume>
        });
        return Ok(PreparedReplacementChild {
            prefix: committed.outcome,
            resume,
        });
    }
    if effect.0.supports_replacement_draw_continuation() {
        let committed = crate::effects::runtime::prepare_effect_draw_continuation_with_outputs(
            game, effect, ctx,
        )?;
        return retain_draw_boundary(committed, ctx);
    }
    if life_action(effect) {
        let mut proposal = effect.prepare_simultaneous_player_action(game, ctx)?;
        proposal.prepare_selection(game, ctx)?;
        proposal.prepare_original(game, ctx)?;
        proposal.seal_original(game, ctx)?;
        let committed = proposal.commit_original_with_outputs(game, ctx)?;
        return prepare_committed_draw_boundary(game, ctx, committed);
    }

    if !contains_draw(effect) {
        return crate::effects::execute_effect_with_outputs(game, effect, ctx)
            .map(PreparedReplacementChild::finished_with_outputs);
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
        return prepare_repetitions(game, ctx, repeat);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return prepare_program(
            game,
            ctx,
            &sequence.effects,
            Mode::Sequence {
                coordinated: sequence.surface.is_coordinated(),
            },
            None,
        );
    }
    if let Some(optional) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if optional.pay_as_cost {
            return crate::effects::execute_effect_with_outputs(game, effect, ctx)
                .map(PreparedReplacementChild::finished_with_outputs);
        }
        let Some(branch) = optional.prepare_optional_execution(game, ctx)? else {
            return Ok(PreparedReplacementChild::finished(EffectOutcome::declined()));
        };
        let previous_optional = std::mem::replace(&mut ctx.optional_action, true);
        let prepared = prepare_program(
            game,
            ctx,
            &optional.effects,
            Mode::Optional {
                has_action: optional
                    .effects
                    .iter()
                    .any(|effect| !crate::effects::is_object_selection(effect)),
            },
            None,
        )?;
        return Ok(scope_result(
            prepared,
            Scope::Optional {
                player: branch.previous_iterated_player,
                optional_action: previous_optional,
            },
            game,
            ctx,
        ));
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::IfEffect>() {
        let branches = crate::effects::prepare_if_branches(conditional, game, ctx);
        return prepare_branches(game, ctx, branches);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        let (branch, guard) = crate::effects::prepare_conditional_branch(conditional, game, ctx)?;
        return prepare_program(game, ctx, &branch, Mode::Aggregate, guard);
    }
    if let Some(annotation) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        ctx.effect_outcomes.remove(&annotation.id);
        let prepared = prepare_effect(game, ctx, &annotation.effect)?;
        return Ok(scope_result(
            prepared,
            Scope::Result(annotation.id),
            game,
            ctx,
        ));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let runtime = crate::effects::capture_tagged_runtime_state(game, &tagged.effect, ctx);
        let prepared = prepare_effect(game, ctx, &tagged.effect)?;
        return Ok(scope_result(
            prepared,
            Scope::Tagged {
                effect: tagged.clone(),
                runtime,
            },
            game,
            ctx,
        ));
    }
    if let Some(rebound) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        let Some((source, snapshot)) = crate::effects::resolve_source_binding(rebound, game, ctx)
        else {
            return Ok(PreparedReplacementChild::finished(
                EffectOutcome::target_invalid(),
            ));
        };
        let scope = Scope::Source {
            source: ctx.source,
            snapshot: ctx.source_snapshot.clone(),
        };
        ctx.source = source;
        ctx.source_snapshot = snapshot;
        let prepared = prepare_effect(game, ctx, &rebound.effect)?;
        return Ok(scope_result(prepared, scope, game, ctx));
    }
    Err(ExecutionError::InternalError(
        "unsupported replacement continuation escaped capability check".into(),
    ))
}

/// A selected replacement program is one original subtree of its expansion.
/// An added program's internal draw belongs to the enclosing additions phase.
enum DrawProgramRole {
    SelectedOriginal(EffectOutcome),
    AddedProgram,
}

struct DrawContinuation {
    role: DrawProgramRole,
    resume: Box<dyn ReplacementResume>,
    source: ObjectId,
    controller: PlayerId,
}
impl SimultaneousEffectCompletion for DrawContinuation {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        match &self.role {
            DrawProgramRole::SelectedOriginal(_) => crate::effects::OriginalPhaseStatus::Retained,
            DrawProgramRole::AddedProgram => crate::effects::OriginalPhaseStatus::Complete,
        }
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.complete_original_phase_from_outputs(
            game,
            ctx,
            CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        if !matches!(&self.role, DrawProgramRole::SelectedOriginal(_)) {
            return Err(ExecutionError::InternalError(
                "added draw program cannot execute as a replacement original".into(),
            ));
        }
        // Only this selected authored subtree is the enclosing expansion's
        // original. Its captured native instruction order stays atomic; this
        // owner has no external added-program queue to advance here.
        crate::effects::composition::complete_authored_original_subtree_with_outputs(
            game, ctx, self, original,
        )
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit {
            outcome: CompletedEffectOutputs::aggregate_only(original),
            completion: Some(self),
        })
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.resume.freeze(game)
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        parent: &mut ExecutionContext,
        original_prefix: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, parent, original_prefix)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        parent: &mut ExecutionContext,
        original_prefix: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.resume.observe_prefix(&original_prefix.events);
        let mut child =
            ExecutionContext::new(self.source, self.controller, &mut *parent.decision_maker);
        let resume = self.resume;
        let outputs =
            crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
                let mut outputs = resume_replacement_child_with_outputs(game, &mut child, resume)?;
                crate::effects::runtime::capture_triggers_before_added_program(
                    game,
                    &child,
                    None,
                    outputs.outcome.events.iter_mut(),
                )?;
                Ok::<_, ExecutionError>(outputs)
            })?;
        let DrawProgramRole::SelectedOriginal(mut original) = self.role else {
            return Ok(outputs);
        };
        crate::effects::composition::inherit_original_observations(
            &mut original,
            &original_prefix.events,
        );
        // The resumed subtree includes its captured prefix once; do not append
        // the prefix receipt again or numeric event evidence would be doubled.
        Ok(super::project_replacement_original_outputs(
            original, outputs,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_draw_continuation_with_bindings_and_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    captured_source_snapshot: Option<ObjectSnapshot>,
    bindings: super::ReplacementProgramBindings,
) -> Result<Option<SimultaneousEffectCommit<CompletedEffectOutputs>>, ExecutionError> {
    let mut original = EffectOutcome::replaced();
    original.set_value(OutcomeValue::Count(0));
    prepare_draw_continuation_with_original_and_outputs(
        game,
        parent,
        effects,
        source,
        controller,
        context,
        captured_source_snapshot,
        bindings,
        original,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_draw_continuation_with_original_and_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    captured_source_snapshot: Option<ObjectSnapshot>,
    bindings: super::ReplacementProgramBindings,
    original: EffectOutcome,
) -> Result<Option<SimultaneousEffectCommit<CompletedEffectOutputs>>, ExecutionError> {
    if !original_program_uses_draw_continuation(effects) {
        return Ok(None);
    }
    super::execute_payload::with_replacement_child(
        game,
        parent,
        source,
        controller,
        context,
        bindings.targets,
        captured_source_snapshot,
        bindings.object_tags,
        |game, child| {
            prepare_bound_original_program_draw_with_outputs(game, child, effects, original)
                .map(Some)
        },
    )
}

/// Eligibility is a property of the authored program, not a grouping proof.
/// Preserve the established native draw schedule and atomic fallback contract.
pub(super) fn original_program_uses_draw_continuation(effects: &[Effect]) -> bool {
    effects.iter().any(contains_draw) && effects.iter().all(replacement_effect_supported)
}

/// Execute the selected original in its already acquired replacement scope.
/// This entry does not reacquire source, targets, tags or interrupted outcomes.
pub(super) fn prepare_bound_original_program_draw_with_outputs(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
    original: EffectOutcome,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    prepare_bound_program_draw_with_outputs(
        game,
        child,
        effects,
        DrawProgramRole::SelectedOriginal(original),
    )
}

/// One prefix/continuation owner for both original replacements and additions.
/// Public adapters own eligibility, bindings and the authored result frame.
fn prepare_bound_program_draw_with_outputs(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
    role: DrawProgramRole,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    let source = child.source;
    let controller = child.controller;
    let prepared = crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
        prepare_program(game, child, effects, Mode::Aggregate, None)
    })?;
    let outcome = match &role {
        DrawProgramRole::SelectedOriginal(original) => {
            super::project_replacement_original_outputs(original.clone(), prepared.prefix)
        }
        DrawProgramRole::AddedProgram => prepared.prefix,
    };
    Ok(SimultaneousEffectCommit {
        outcome,
        completion: prepared.resume.map(|resume| {
            Box::new(DrawContinuation {
                role,
                resume,
                source,
                controller,
            }) as Box<dyn SimultaneousEffectCompletion>
        }),
    })
}

/// Retain one already-bound program's own result, without inventing a
/// replaced-original summary. Added programs and prevention queues use this.
pub(crate) fn prepare_scoped_program_draw_boundary_with_outputs(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    if !effects.iter().all(replacement_effect_supported) {
        return Err(ExecutionError::Impossible(
            "program has no native draw-continuation owner".into(),
        ));
    }
    prepare_bound_program_draw_with_outputs(game, child, effects, DrawProgramRole::AddedProgram)
}

/// Continue an already established replacement scope, preserving all captured
/// source, target, local result and acquisition bindings.
pub(crate) fn prepare_scoped_draw_continuation_with_outputs(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<Option<SimultaneousEffectCommit<CompletedEffectOutputs>>, ExecutionError> {
    if !original_program_uses_draw_continuation(effects) {
        return Ok(None);
    }
    let mut original = EffectOutcome::replaced();
    original.set_value(OutcomeValue::Count(0));
    prepare_bound_original_program_draw_with_outputs(game, child, effects, original).map(Some)
}

pub(crate) fn prepare_scoped_draw_continuation(
    game: &mut GameState,
    child: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<Option<SimultaneousEffectCommit>, ExecutionError> {
    prepare_scoped_draw_continuation_with_outputs(game, child, effects)
        .map(|prepared| prepared.map(SimultaneousEffectCommit::into_aggregate))
}

/// The only dispatch boundary for a captured continuation subtree. Failure
/// and suspension unwind its context before an enclosing scope can catch the
/// error or decide whether the next authored instruction should run.
pub(crate) fn resume_replacement_child_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    continuation: Box<dyn ReplacementResume>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| continuation.resume_outputs(game, ctx),
    )
}

pub(crate) fn prepare_replacement_child(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &Effect,
) -> Result<PreparedReplacementChild, ExecutionError> {
    prepare_effect(game, ctx, effect)
}
pub(crate) fn replacement_effect_contains_draw(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some()
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= replacement_effect_contains_draw(child));
    found
}
