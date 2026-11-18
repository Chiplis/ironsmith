//! Selected iteration frames retain bindings and delegate genuine child actions.

use super::prepared_iteration::{IterationScope, IterationTagScope};
use crate::effect::{Effect, EffectId, EffectOutcome};
use crate::effects::{
    ActionProgramCursor, CompletedEffectOutputs, ExecutionContext, ExecutionError, ProgramAction,
    ProgramActionScope, ProgramCompletion,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

pub(super) struct IterationInput {
    pub object: Option<ObjectId>,
    pub player: PlayerId,
    pub tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
}

/// Selection and result policies stay with their authored iteration family.
/// Traversal, actual child ownership and binding lifetime have one owner.
pub(super) trait SelectedIterationPlan: Send {
    fn len(&self) -> usize;
    fn effects(&self) -> &[Effect];
    /// Per-announcement target view for an iteration. Ordinary object/player
    /// iteration inherits the enclosing target set unchanged.
    fn target_scope(&self, _index: usize) -> Option<(
        Vec<crate::effects::ResolvedTarget>,
        Vec<crate::game_state::TargetAssignment>,
    )> { None }
    fn select(
        &mut self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        index: usize,
    ) -> Result<IterationInput, ExecutionError>;
    fn root_tags(&self) -> Vec<(TagKey, Option<Vec<ObjectSnapshot>>)> {
        Vec::new()
    }
    fn result_slot(&self) -> Option<EffectId> {
        None
    }
    fn entry_attachment_hints(&self) -> bool {
        false
    }
    fn project(
        &self,
        outcomes: Vec<EffectOutcome>,
        _ranges: &[(PlayerId, usize, usize)],
    ) -> EffectOutcome {
        EffectOutcome::aggregate_summing_counts(outcomes)
    }
}

struct IterationResultScope {
    id: EffectId,
    previous: Option<EffectOutcome>,
}
impl IterationResultScope {
    fn enter(ctx: &mut ExecutionContext, id: EffectId) -> Self {
        Self {
            id,
            previous: ctx.effect_outcomes.remove(&id),
        }
    }
    fn leave(self, ctx: &mut ExecutionContext) {
        ctx.effect_outcomes.remove(&self.id);
        if let Some(previous) = self.previous {
            ctx.store_outcome(self.id, previous);
        }
    }
}

pub(super) fn selected_iteration_cursor(
    plan: Box<dyn SelectedIterationPlan>,
    ctx: &mut ExecutionContext,
) -> Box<dyn ActionProgramCursor> {
    if plan.len() == 0 {
        return super::action_program::finished_program_cursor(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        );
    }
    let mut outputs = CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
    outputs.projections_complete = !plan.effects().is_empty();
    let unit_ends =
        super::action_units::partition_action_units(plan.effects(), |_| None, |_, _| true)
            .into_iter()
            .filter_map(|unit| unit.last().copied())
            .collect();
    let tags = Some(IterationTagScope::enter(ctx, plan.root_tags()));
    let result = plan
        .result_slot()
        .map(|id| IterationResultScope::enter(ctx, id));
    Box::new(IterationProgramCursor {
        plan,
        iteration: 0,
        child: 0,
        active: None,
        tags,
        result,
        child_pending: false,
        ranges: Vec::new(),
        unit_ends,
        ends_unit: false,
        outputs,
        outcomes: Vec::new(),
    })
}
struct ActiveIteration {
    scope: IterationScope,
    object: Option<ObjectId>,
    player: PlayerId,
    range_start: usize,
}

struct IterationProgramCursor {
    plan: Box<dyn SelectedIterationPlan>,
    iteration: usize,
    child: usize,
    active: Option<ActiveIteration>,
    tags: Option<IterationTagScope>,
    result: Option<IterationResultScope>,
    child_pending: bool,
    ranges: Vec<(PlayerId, usize, usize)>,
    unit_ends: Vec<usize>,
    ends_unit: bool,
    outputs: CompletedEffectOutputs,
    outcomes: Vec<EffectOutcome>,
}
impl std::fmt::Debug for IterationProgramCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IterationProgramCursor")
            .field("iteration", &self.iteration)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}
impl IterationProgramCursor {
    fn leave_iteration(&mut self, ctx: &mut ExecutionContext) {
        if let Some(active) = self.active.take() {
            active.scope.leave(ctx);
            self.ranges.push((active.player, active.range_start, self.outcomes.len()));
        }
    }

    fn leave_plan(&mut self, ctx: &mut ExecutionContext) {
        self.leave_iteration(ctx);
        if let Some(tags) = self.tags.take() {
            tags.leave(ctx);
        }
        if let Some(result) = self.result.take() {
            result.leave(ctx);
        }
    }
}

impl ActionProgramCursor for IterationProgramCursor {
    fn finish_stopped(mut self: Box<Self>, _game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<ProgramCompletion, ExecutionError> {
        self.child_pending = false;
        self.leave_plan(ctx);
        self.finish()
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if self.child_pending {
            return Err(ExecutionError::InternalError(
                "iteration advanced before child completion".into(),
            ));
        }
        if ctx.resolution_stopped() {
            self.leave_plan(ctx);
            return Ok(None);
        }
        while self.iteration < self.plan.len() {
            if self.active.is_none() {
                let input = self.plan.select(game, ctx, self.iteration)?;
                self.active = Some(ActiveIteration {
                    scope: IterationScope::enter(ctx, input.object, Some(input.player), input.tags),
                    object: input.object,
                    player: input.player,
                    range_start: self.outcomes.len(),
                });
                self.child = 0;
            }
            if let Some(effect) = self.plan.effects().get(self.child) {
                let active = self.active.as_ref().expect("active iteration");
                let scope = ProgramActionScope {
                    targets: self.plan.target_scope(self.iteration),
                    iterated_object: active.object.map(Some),
                    iterated_player: Some(Some(active.player)),
                    entry_attachment_receipts: Some(if self.plan.entry_attachment_hints()
                        && self.child > 0
                        && crate::effects::permanents::entry_attachment_for_move(
                            &self.plan.effects()[self.child - 1], Some(effect),
                        ).is_some()
                    {
                        Some(self.outcomes.last().and_then(EffectOutcome::entry_attachments)
                            .ok_or_else(|| ExecutionError::InternalError(
                                "iteration entry instruction lost its attachment receipt".into(),
                            ))?)
                    } else { None }),
                    pending_entry_attachment: self.plan.entry_attachment_hints().then(|| {
                        crate::effects::permanents::entry_attachment_for_move(
                            effect,
                            self.plan.effects().get(self.child + 1),
                        )
                    }),
                    ..Default::default()
                };
                let action = ProgramAction {
                    effect: effect.clone(),
                    scope,
                    native: None,
                    identity: vec![0, self.iteration, self.child],
                };
                self.ends_unit = self.unit_ends.contains(&self.child);
                self.child += 1;
                self.child_pending = true;
                return Ok(Some(action));
            }
            self.leave_iteration(ctx);
            self.iteration += 1;
        }
        self.leave_plan(ctx);
        Ok(None)
    }
    fn accept_action(&mut self, child: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        if !self.child_pending {
            return Err(ExecutionError::InternalError(
                "unexpected iteration acknowledgement".into(),
            ));
        }
        self.child_pending = false;
        self.outcomes.push(child.outcome.clone());
        self.outputs.retain_owned_child(child);
        Ok(())
    }
    fn ends_action_unit(&self) -> bool {
        self.ends_unit
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        if self.child_pending
            || self.active.is_some()
            || self.tags.is_some()
            || self.result.is_some()
        {
            return Err(ExecutionError::InternalError(
                "unfinished iteration program".into(),
            ));
        }
        let aggregate = self.plan.project(self.outcomes, &self.ranges);
        Ok(ProgramCompletion::new(
            self.outputs.project_aggregate(aggregate),
        ))
    }
}
