//! CR 121.7 replacement-program continuations. A selected branch and every
//! enclosing scope are retained; resumption never reruns the original prefix.
use crate::effect::{Effect, EffectId, EffectOutcome, ExecutionFact, OutcomeValue};
use crate::effects::{
    ExecutionContext, ExecutionContextCheckpoint, ExecutionError, SimultaneousEffectCommit,
    SimultaneousEffectCompletion,
};
use crate::events::processing::ReplacementEventContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

trait Resume: Send {
    /// Return the completed subtree, including its prefix exactly once. The
    /// leaf restores the captured execution state and enclosing scopes unwind.
    fn resume(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}
struct Prepared {
    prefix: EffectOutcome,
    resume: Option<Box<dyn Resume>>,
}
impl Prepared {
    fn finished(prefix: EffectOutcome) -> Self {
        Self {
            prefix,
            resume: None,
        }
    }
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
            Self::Sequence { .. } => {
                let Some(terminal) = outcomes.last() else {
                    return EffectOutcome::count(0);
                };
                EffectOutcome::with_details(
                    terminal.status,
                    terminal.value.clone(),
                    outcomes
                        .iter()
                        .flat_map(|outcome| outcome.events.iter().cloned())
                        .collect(),
                    outcomes
                        .iter()
                        .flat_map(|outcome| outcome.execution_facts.iter().cloned())
                        .collect(),
                )
            }
            Self::Optional { .. } => {
                EffectOutcome::aggregate(outcomes).with_execution_fact(ExecutionFact::Accepted)
            }
            Self::Aggregate if outcomes.is_empty() => EffectOutcome::count(0),
            Self::Aggregate => EffectOutcome::aggregate(outcomes),
        }
    }
}

fn contains_draw(effect: &Effect) -> bool {
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
fn supported(effect: &Effect) -> bool {
    if !contains_draw(effect)
        || effect
            .downcast_ref::<crate::effects::DrawCardsEffect>()
            .is_some()
    {
        return true;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().all(supported);
    }
    if let Some(optional) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return optional.effects.iter().all(supported);
    }
    if let Some(condition) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return condition.then.iter().chain(&condition.else_).all(supported);
    }
    if let Some(condition) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return condition
            .if_true
            .iter()
            .chain(&condition.if_false)
            .all(supported);
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
        return effect.0.transparent_child_effect().is_some_and(supported);
    }
    false
}

struct DrawLeaf {
    context: ExecutionContextCheckpoint,
    effect: Effect,
}
impl Resume for DrawLeaf {
    fn resume(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.context.restore(ctx);
        crate::effects::execute_effect(game, &self.effect, ctx)
    }
}
struct ProgramFrame {
    before: Vec<EffectOutcome>,
    first: Box<dyn Resume>,
    first_effect: Effect,
    tail: Vec<Effect>,
    mode: Mode,
}
impl Resume for ProgramFrame {
    fn resume(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut outcomes = self.before;
        let mut first = self.first.resume(game, ctx)?;
        self.mode.adjust(&self.first_effect, &mut first);
        let stop = self.mode.stops(&first);
        outcomes.push(first);
        if stop || ctx.decision_maker.awaiting_choice() {
            return Ok(self.mode.finish(outcomes));
        }
        for (index, effect) in self.tail.iter().enumerate() {
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                Some(effect),
                outcomes
                    .iter_mut()
                    .flat_map(|outcome| outcome.events.iter_mut()),
            );
            let result = crate::effects::execute_effect(game, effect, ctx);
            let mut outcome = match result {
                Err(ExecutionError::InvalidTarget)
                    if matches!(self.mode, Mode::Sequence { coordinated: true }) =>
                {
                    EffectOutcome::target_invalid()
                }
                other => other?,
            };
            self.mode.adjust(effect, &mut outcome);
            let stop = self.mode.stops(&outcome);
            outcomes.push(outcome);
            if stop || ctx.decision_maker.awaiting_choice() {
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
                );
            }
        }
        Ok(self.mode.finish(outcomes))
    }
}

enum Scope {
    Result(EffectId),
    Player(Option<PlayerId>),
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
    inner: Box<dyn Resume>,
}
impl Resume for ScopeFrame {
    fn resume(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = self.inner.resume(game, ctx)?;
        self.scope.leave(game, ctx, &outcome);
        Ok(outcome)
    }
}
fn scope_result(
    mut prepared: Prepared,
    scope: Scope,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Prepared {
    if let Some(inner) = prepared.resume.take() {
        prepared.resume = Some(Box::new(ScopeFrame { scope, inner }));
    } else {
        scope.leave(game, ctx, &prepared.prefix);
    }
    prepared
}

struct BranchesFrame {
    before: Vec<EffectOutcome>,
    first: Box<dyn Resume>,
    rest: Vec<crate::effects::PreparedIfBranch>,
}
impl Resume for BranchesFrame {
    fn resume(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut outcomes = self.before;
        outcomes.push(self.first.resume(game, ctx)?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            outcomes
                .iter_mut()
                .flat_map(|outcome| outcome.events.iter_mut()),
        );
        outcomes.push(crate::effects::execute_if_branches(game, ctx, &self.rest)?);
        Ok(EffectOutcome::aggregate(outcomes))
    }
}
fn prepare_branches(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    branches: Vec<crate::effects::PreparedIfBranch>,
) -> Result<Prepared, ExecutionError> {
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
                let mut prefix = outcomes.clone();
                prefix.push(prepared.prefix);
                let mut rest = Vec::new();
                if repetition + 1 < branch.repetitions {
                    let mut remaining = branch.clone();
                    remaining.repetitions -= repetition + 1;
                    rest.push(remaining);
                }
                rest.extend_from_slice(&branches[index + 1..]);
                return Ok(Prepared {
                    prefix: EffectOutcome::aggregate(prefix),
                    resume: Some(Box::new(BranchesFrame {
                        before: outcomes,
                        first,
                        rest,
                    })),
                });
            }
            outcomes.push(prepared.prefix);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Prepared::finished(EffectOutcome::count(0)));
            }
        }
    }
    Ok(Prepared::finished(if outcomes.is_empty() {
        EffectOutcome::count(0)
    } else {
        EffectOutcome::aggregate(outcomes)
    }))
}

fn prepare_program(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    mode: Mode,
    first_guard: Option<crate::effects::context::OptionalIdentityGuard>,
) -> Result<Prepared, ExecutionError> {
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
                Prepared::finished(EffectOutcome::target_invalid())
            }
            other => other?,
        };
        if let Some(scope) = guard_scope {
            prepared = scope_result(prepared, scope, game, ctx);
        }
        if let Some(first) = prepared.resume {
            let mut prefix = outcomes.clone();
            mode.adjust(effect, &mut prepared.prefix);
            prefix.push(prepared.prefix);
            return Ok(Prepared {
                prefix: mode.finish(prefix),
                resume: Some(Box::new(ProgramFrame {
                    before: outcomes,
                    first,
                    first_effect: effect.clone(),
                    tail: effects[index + 1..].to_vec(),
                    mode,
                })),
            });
        }
        mode.adjust(effect, &mut prepared.prefix);
        let stop = mode.stops(&prepared.prefix);
        outcomes.push(prepared.prefix);
        if stop || ctx.decision_maker.awaiting_choice() {
            break;
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            effects.get(index + 1),
            outcomes
                .iter_mut()
                .flat_map(|outcome| outcome.events.iter_mut()),
        );
    }
    Ok(Prepared::finished(mode.finish(outcomes)))
}
fn prepare_effect(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &Effect,
) -> Result<Prepared, ExecutionError> {
    if game.turn_store.end_turn_procedure_pending
        || game.turn_store.end_combat_phase_procedure_pending
    {
        return crate::effects::execute_effect(game, effect, ctx).map(Prepared::finished);
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        // A zero instruction has no draw boundary. In particular its suffix
        // must not be delayed past another original merely because the AST
        // contains a draw-shaped node.
        if crate::effects::resolve_value(game, &draw.count, ctx)? <= 0 {
            return crate::effects::execute_effect(game, effect, ctx).map(Prepared::finished);
        }
        return Ok(Prepared {
            prefix: EffectOutcome::count(0),
            resume: Some(Box::new(DrawLeaf {
                context: ExecutionContextCheckpoint::capture(ctx),
                effect: effect.clone(),
            })),
        });
    }
    if !contains_draw(effect) {
        return crate::effects::execute_effect(game, effect, ctx).map(Prepared::finished);
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
        let Some(branch) = optional.prepare_optional_execution(game, ctx)? else {
            return Ok(Prepared::finished(EffectOutcome::declined()));
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
            return Ok(Prepared::finished(EffectOutcome::target_invalid()));
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

struct DrawContinuation {
    resume: Box<dyn Resume>,
    source: ObjectId,
    controller: PlayerId,
}
impl SimultaneousEffectCompletion for DrawContinuation {
    fn freeze(&mut self, _game: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        parent: &mut ExecutionContext,
        _original_prefix: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut child =
            ExecutionContext::new(self.source, self.controller, &mut *parent.decision_maker);
        let mut payload =
            crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
                let mut outcome = self.resume.resume(game, &mut child)?;
                crate::effects::runtime::capture_triggers_before_added_program(
                    game,
                    &child,
                    None,
                    outcome.events.iter_mut(),
                );
                Ok::<_, ExecutionError>(outcome)
            })?;
        let mut original = EffectOutcome::replaced();
        original.set_value(OutcomeValue::Count(0));
        // The resumed subtree includes its captured prefix once; do not append
        // the prefix receipt again or numeric event evidence would be doubled.
        Ok(EffectOutcome::aggregate_replacement_outcomes(
            original,
            [payload],
        ))
    }
}

pub(crate) fn prepare_draw_continuation(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
) -> Result<Option<SimultaneousEffectCommit>, ExecutionError> {
    if !effects.iter().any(contains_draw) || !effects.iter().all(supported) {
        return Ok(None);
    }
    super::execute_payload::with_replacement_child(
        game,
        parent,
        source,
        controller,
        context,
        None,
        None,
        Vec::new(),
        |game, child| {
            if !child.target_assignments.is_empty() {
                return Ok(None);
            }
            let prepared =
                crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
                    prepare_program(game, child, effects, Mode::Aggregate, None)
                })?;
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            Ok(Some(SimultaneousEffectCommit {
                outcome: EffectOutcome::aggregate_replacement_outcomes(original, [prepared.prefix]),
                completion: prepared.resume.map(|resume| {
                    Box::new(DrawContinuation {
                        resume,
                        source,
                        controller,
                    }) as Box<dyn SimultaneousEffectCompletion>
                }),
            }))
        },
    )
}
