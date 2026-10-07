use crate::effect::{Effect, EffectOutcome, OutcomeStatus, OutcomeValue};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::Comparison;
use crate::game_state::GameState;
use crate::resolve_value;

pub type RepeatEffectsEffect = ironsmith_core::RepeatEffectsEffect<Effect>;

impl EffectExecutor for RepeatEffectsEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        self.effects.iter().all(crate::effects::replacement::replacement_effect_supported)
    }
    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

    fn supports_prepared_action_program(&self) -> bool {
        self.effects
            .iter()
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(Some(select_repetition_cursor(self, game, ctx)?))
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_repeated_program(self, game, ctx),
        )
    }
}

/// Repetition counts are fixed when the authored program is entered. Draw
/// continuations use this same selector rather than freezing later inputs.
pub(crate) fn resolve_repeat_count(
    game: &GameState,
    count: &crate::effect::Value,
    ctx: &ExecutionContext,
) -> Result<usize, ExecutionError> {
    Ok(resolve_value(game, count, ctx)?.max(0) as usize)
}

/// Result ownership is independent of the domain that schedules a repeated
/// program. Keep each child's primary result and actual observations, while
/// projecting the authored union and first terminal failure onto the parent.
pub(crate) fn finish_repeated_sequence_outcomes(outcomes: Vec<EffectOutcome>) -> EffectOutcome {
    let mut objects = Vec::new();
    for outcome in &outcomes {
        for object in outcome.objects().into_iter().flatten() {
            if !objects.contains(object) {
                objects.push(*object);
            }
        }
    }
    let failed = outcomes.iter().find(|outcome| outcome.status.is_failure());
    let status = failed.map_or(OutcomeStatus::Succeeded, |outcome| outcome.status);
    let value = if objects.is_empty() {
        failed.map_or(OutcomeValue::None, |outcome| outcome.value.clone())
    } else {
        OutcomeValue::Objects(objects)
    };
    EffectOutcome::aggregate_with_primary_result(
        EffectOutcome::with_details(status, value, Vec::new(), Vec::new()),
        outcomes,
    )
}

/// Each occurrence has a fresh team-operation set while retaining all claims
/// made within that occurrence. Ordinary/staged programs and draw continuations
/// restore the enclosing set through this same scope owner.
pub(crate) struct RepetitionScope(std::collections::HashSet<(usize, usize, &'static str)>);
impl RepetitionScope {
    pub(crate) fn enter(ctx: &mut ExecutionContext) -> Self {
        Self(std::mem::take(&mut ctx.shared_team_structure_operations))
    }
    pub(crate) fn leave(self, ctx: &mut ExecutionContext) {
        ctx.shared_team_structure_operations = self.0;
    }
}

/// A repetition yields a complete authored Sequence, so its target/result
/// scopes remain owned by Sequence rather than by a flattened Repeat loop.
enum RepetitionPlan {
    Sequence {
        body: Effect,
        count: usize,
    },
    DistinctPowers {
        choice: crate::effects::ChooseObjectsEffect,
        powers: Vec<i32>,
    },
    MultipliedCreation {
        instruction: Effect,
        count: usize,
    },
}
impl RepetitionPlan {
    fn len(&self) -> usize {
        match self {
            Self::Sequence { count, .. } | Self::MultipliedCreation { count, .. } => *count,
            Self::DistinctPowers { powers, .. } => powers.len(),
        }
    }
    fn resets_team_operations(&self) -> bool {
        !matches!(self, Self::DistinctPowers { .. })
    }
    fn child(&self, index: usize) -> (Effect, Vec<usize>) {
        match self {
            Self::Sequence { body, .. } => (body.clone(), vec![0, index]),
            Self::DistinctPowers { choice, powers } => {
                let mut choice = choice.clone();
                choice.filter.power = Some(Comparison::Equal(powers[index]));
                (
                    Effect::new(SequenceEffect::new(vec![Effect::new(choice)])),
                    vec![1, index],
                )
            }
            Self::MultipliedCreation { instruction, .. } => (instruction.clone(), vec![2]),
        }
    }
}

fn select_repetition_cursor(
    effect: &RepeatEffectsEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Box<dyn crate::effects::ActionProgramCursor>, ExecutionError> {
    // One choice per captured effective power class, retaining the union under
    // the authored tag only when selection finishes or a child fails.
    let plan = if let crate::effect::Value::DistinctPowers(filter) = effect.count.unhinted()
        && let [child] = effect.effects.as_slice()
        && let Some(choice) = child.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choice.count.is_single()
        && &choice.filter == filter
    {
        let powers = crate::effects::helpers::distinct_power_values_for_filter(game, filter, ctx);
        ctx.clear_object_tag(&choice.tag);
        RepetitionPlan::DistinctPowers {
            choice: choice.clone(),
            powers,
        }
    } else {
        let count = resolve_repeat_count(game, &effect.count, ctx)?;
        // A vote-count creation is one request multiplied before replacement
        // processing. Other repetitions retain separate action identities.
        if matches!(effect.count.unhinted(), crate::effect::Value::VoteCount(_))
            && let [child] = effect.effects.as_slice()
            && let Some(creation) = child.downcast_ref::<crate::effects::CreateTokenEffect>()
        {
            RepetitionPlan::MultipliedCreation {
                instruction: crate::effects::tokens::multiplied_token_instruction(
                    creation,
                    count as u32,
                ),
                count: usize::from(count > 0),
            }
        } else {
            RepetitionPlan::Sequence {
                body: Effect::new(SequenceEffect::new(effect.effects.clone())),
                count,
            }
        }
    };
    let mut outputs =
        crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
    outputs.projections_complete = plan.len() > 0;
    Ok(Box::new(RepetitionCursor {
        plan,
        next: 0,
        child_pending: false,
        completed: None,
        stopped: false,
        previous_operations: None,
        outputs,
        outcomes: Vec::new(),
        events: Vec::new(),
        reported_cursor: 0,
        selected: Vec::new(),
    }))
}

struct RepetitionCursor {
    plan: RepetitionPlan,
    next: usize,
    child_pending: bool,
    completed: Option<EffectOutcome>,
    stopped: bool,
    previous_operations: Option<RepetitionScope>,
    outputs: crate::effects::CompletedEffectOutputs,
    outcomes: Vec<EffectOutcome>,
    events: Vec<crate::events::RawEvent>,
    reported_cursor: usize,
    selected: Vec<crate::snapshot::ObjectSnapshot>,
}
impl std::fmt::Debug for RepetitionCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepetitionCursor")
            .field("next", &self.next)
            .field("count", &self.plan.len())
            .field("stopped", &self.stopped)
            .finish_non_exhaustive()
    }
}
impl crate::effects::ActionProgramCursor for RepetitionCursor {
    fn finish_stopped(mut self: Box<Self>, _game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        self.child_pending = false;
        self.completed = None;
        if let Some(operations) = self.previous_operations.take() { operations.leave(ctx); }
        self.finish()
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<crate::effects::ProgramAction>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if let Some(outcome) = self.completed.take() {
            if let Some(operations) = self.previous_operations.take() {
                operations.leave(ctx);
            }
            if let RepetitionPlan::DistinctPowers { choice, .. } = &self.plan {
                if let Some(current) = ctx.get_tagged_all(&choice.tag) {
                    for snapshot in current {
                        if !self
                            .selected
                            .iter()
                            .any(|existing| existing.object_id == snapshot.object_id)
                        {
                            self.selected.push(snapshot.clone());
                        }
                    }
                }
            }
            self.stopped = outcome.status.is_failure();
        }
        self.stopped |= ctx.resolution_stopped();
        if self.stopped || self.next >= self.plan.len() {
            if let RepetitionPlan::DistinctPowers { choice, .. } = &self.plan {
                ctx.set_tagged_objects(choice.tag.clone(), self.selected.clone());
            }
            return Ok(None);
        }
        if self.child_pending {
            return Err(ExecutionError::InternalError(
                "repetition advanced before its child completed".into(),
            ));
        }
        if matches!(self.plan, RepetitionPlan::Sequence { .. })
            && self.next > 0
            && crate::effects::match_triggers_at_instruction_boundary(
                game,
                ctx,
                None,
                self.events[self.reported_cursor..].iter(),
            )?
        {
            self.reported_cursor = self.events.len();
        }
        if self.plan.resets_team_operations() {
            self.previous_operations = Some(RepetitionScope::enter(ctx));
        }
        let (effect, identity) = self.plan.child(self.next);
        self.next += 1;
        self.child_pending = true;
        Ok(Some(crate::effects::ProgramAction {
            native: None,
            effect,
            identity,
            scope: crate::effects::ProgramActionScope::default(),
        }))
    }
    fn accept_action(
        &mut self,
        child: crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        if !self.child_pending || self.completed.is_some() {
            return Err(ExecutionError::InternalError(
                "unexpected repetition child acknowledgement".into(),
            ));
        }
        self.child_pending = false;
        let outcome = child.outcome.clone();
        self.events.extend(outcome.events.clone());
        self.outcomes.push(outcome.clone());
        self.completed = Some(outcome);
        self.outputs.retain_owned_child(child);
        Ok(())
    }
    fn ends_action_unit(&self) -> bool {
        self.child_pending
    }
    fn finish(self: Box<Self>) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        if self.child_pending || self.completed.is_some() || self.previous_operations.is_some() {
            return Err(ExecutionError::InternalError(
                "unfinished repetition program".into(),
            ));
        }
        let primary = match &self.plan {
            RepetitionPlan::Sequence { .. } => None,
            RepetitionPlan::DistinctPowers { .. } => Some(
                self.outcomes
                    .iter()
                    .find(|outcome| outcome.status.is_failure())
                    .map_or_else(EffectOutcome::resolved, |outcome| {
                        EffectOutcome::with_details(
                            outcome.status,
                            outcome.value.clone(),
                            Vec::new(),
                            Vec::new(),
                        )
                    }),
            ),
            RepetitionPlan::MultipliedCreation { .. } => Some(self.outcomes.first().map_or_else(
                EffectOutcome::resolved,
                |outcome| {
                    let objects = outcome.objects().unwrap_or_default().to_vec();
                    EffectOutcome::with_details(
                        outcome.status,
                        if objects.is_empty() {
                            OutcomeValue::None
                        } else {
                            OutcomeValue::Objects(objects)
                        },
                        Vec::new(),
                        Vec::new(),
                    )
                },
            )),
        };
        let aggregate = if let Some(primary) = primary {
            EffectOutcome::aggregate_with_primary_result(primary, self.outcomes)
        } else {
            finish_repeated_sequence_outcomes(self.outcomes)
        };
        Ok(crate::effects::ProgramCompletion::new(
            self.outputs.project_aggregate(aggregate),
        ))
    }
}

fn execute_repeated_program(
    effect: &RepeatEffectsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let cursor = select_repetition_cursor(effect, game, ctx)?;
    super::action_program::execute_action_program_with_outputs(
        cursor,
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )
}
