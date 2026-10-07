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
    fn postlude(&mut self) -> Vec<Effect> {
        Vec::new()
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
        scope: None,
        input: None,
        tags,
        result,
        child_pending: false,
        range_start: 0,
        ranges: Vec::new(),
        unit_ends,
        ends_unit: false,
        postlude: None,
        postlude_next: 0,
        outputs,
        outcomes: Vec::new(),
    })
}
struct IterationProgramCursor {
    plan: Box<dyn SelectedIterationPlan>,
    iteration: usize,
    child: usize,
    scope: Option<IterationScope>,
    input: Option<IterationInput>,
    tags: Option<IterationTagScope>,
    result: Option<IterationResultScope>,
    child_pending: bool,
    range_start: usize,
    ranges: Vec<(PlayerId, usize, usize)>,
    unit_ends: Vec<usize>,
    ends_unit: bool,
    postlude: Option<Vec<Effect>>,
    postlude_next: usize,
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
impl ActionProgramCursor for IterationProgramCursor {
    fn finish_stopped(mut self: Box<Self>, _game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<ProgramCompletion, ExecutionError> {
        self.child_pending = false;
        if let Some(scope) = self.scope.take() { scope.leave(ctx); }
        if let Some(input) = self.input.take() {
            self.ranges.push((input.player, self.range_start, self.outcomes.len()));
        }
        if let Some(tags) = self.tags.take() { tags.leave(ctx); }
        if let Some(result) = self.result.take() { result.leave(ctx); }
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
            if let Some(scope) = self.scope.take() { scope.leave(ctx); }
            if let Some(input) = self.input.take() {
                self.ranges.push((input.player, self.range_start, self.outcomes.len()));
            }
            if let Some(tags) = self.tags.take() { tags.leave(ctx); }
            if let Some(result) = self.result.take() { result.leave(ctx); }
            return Ok(None);
        }
        while self.iteration < self.plan.len() {
            if self.input.is_none() {
                let input = self.plan.select(game, ctx, self.iteration)?;
                self.scope = Some(IterationScope::enter(
                    ctx,
                    input.object,
                    Some(input.player),
                    input.tags.clone(),
                ));
                self.input = Some(input);
                self.child = 0;
                self.range_start = self.outcomes.len();
            }
            if let Some(effect) = self.plan.effects().get(self.child) {
                let input = self.input.as_ref().expect("active iteration");
                let scope = ProgramActionScope {
                    iterated_object: input.object.map(Some),
                    iterated_player: Some(Some(input.player)),
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
            self.scope.take().expect("iteration scope").leave(ctx);
            let input = self.input.take().expect("iteration input");
            self.ranges
                .push((input.player, self.range_start, self.outcomes.len()));
            self.iteration += 1;
        }
        if let Some(tags) = self.tags.take() {
            tags.leave(ctx);
        }
        if let Some(result) = self.result.take() {
            result.leave(ctx);
        }
        let postlude = self.postlude.get_or_insert_with(|| self.plan.postlude());
        if let Some(effect) = postlude.get(self.postlude_next) {
            let action = ProgramAction {
                effect: effect.clone(),
                scope: ProgramActionScope::default(),
                native: None,
                identity: vec![1, self.postlude_next],
            };
            self.postlude_next += 1;
            self.ends_unit = true;
            self.child_pending = true;
            return Ok(Some(action));
        }
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
            || self.scope.is_some()
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
