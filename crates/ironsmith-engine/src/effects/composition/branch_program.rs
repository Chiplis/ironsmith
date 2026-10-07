//! Selected branch traversal delegates actual children to their action owners.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    ActionProgramCursor, CompletedEffectOutputs, ExecutionContext, ExecutionError, ProgramAction,
    ProgramActionScope, ProgramCompletion,
};
use crate::game_state::GameState;

/// A selected clause retains its authored identity, repetitions and local
/// context. Child target scopes remain inherited; Sequence owns rebasing.
pub(super) struct SelectedProgramBranch {
    pub effects: Vec<Effect>,
    pub identity: Vec<usize>,
    pub repetitions: usize,
    pub scope: ProgramActionScope,
    pub child_scope: Option<ProgramActionScope>,
    pub first_scope: Option<ProgramActionScope>,
    pub match_before_first: bool,
}

/// Branch-specific result policy only projects the enclosing aggregate. The
/// cursor retains genuine child packets independently of that projection.
pub(super) trait SelectedBranchProjection: Send {
    /// Raw clause programs retain their existing child gateway boundaries;
    /// authored branch families can request explicit instruction matching.
    fn matches_instruction_boundaries(&self) -> bool {
        true
    }
    fn empty_outcome(&self) -> EffectOutcome {
        EffectOutcome::count(0)
    }
    fn project_child(&self, _effect: &Effect, outcome: EffectOutcome) -> EffectOutcome {
        outcome
    }
    fn complete_outcome(&self, outcome: EffectOutcome) -> EffectOutcome {
        outcome
    }
    fn completion_facts(&self) -> Vec<crate::effect::ExecutionFact> {
        Vec::new()
    }
}

pub(super) fn selected_branch_cursor(
    branches: Vec<SelectedProgramBranch>,
) -> Box<dyn ActionProgramCursor> {
    selected_branch_cursor_with_projection(branches, None)
}

pub(super) fn selected_branch_cursor_with_projection(
    branches: Vec<SelectedProgramBranch>,
    projection: Option<Box<dyn SelectedBranchProjection>>,
) -> Box<dyn ActionProgramCursor> {
    let unit_ends = branches
        .iter()
        .map(|branch| {
            super::action_units::partition_action_units(&branch.effects, |_| None, |_, _| true)
                .into_iter()
                .filter_map(|unit| unit.last().copied())
                .collect()
        })
        .collect();
    let mut outputs = CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
    outputs.projections_complete = true;
    Box::new(SelectedBranchCursor {
        branches,
        unit_ends,
        branch: 0,
        repetition: 0,
        next: 0,
        ends_unit: false,
        outputs,
        outcomes: Vec::new(),
        projection,
        pending_child: None,
    })
}

struct SelectedBranchCursor {
    branches: Vec<SelectedProgramBranch>,
    unit_ends: Vec<Vec<usize>>,
    branch: usize,
    repetition: usize,
    next: usize,
    ends_unit: bool,
    outputs: CompletedEffectOutputs,
    outcomes: Vec<EffectOutcome>,
    projection: Option<Box<dyn SelectedBranchProjection>>,
    pending_child: Option<(usize, usize)>,
}
impl std::fmt::Debug for SelectedBranchCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectedBranchCursor")
            .field("branch", &self.branch)
            .field("repetition", &self.repetition)
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}
impl ActionProgramCursor for SelectedBranchCursor {
    fn finish_stopped(mut self: Box<Self>, _game: &mut GameState, _ctx: &mut ExecutionContext)
        -> Result<ProgramCompletion, ExecutionError> {
        self.pending_child = None;
        self.finish()
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ProgramAction>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() || ctx.resolution_stopped() {
            return Ok(None);
        }
        loop {
            let Some(branch) = self.branches.get(self.branch) else {
                return Ok(None);
            };
            if branch.effects.is_empty() || self.repetition >= branch.repetitions {
                self.branch += 1;
                self.repetition = 0;
                self.next = 0;
                continue;
            }
            if self.next >= branch.effects.len() {
                self.repetition += 1;
                self.next = 0;
                continue;
            }
            let effect = &branch.effects[self.next];
            if (branch.match_before_first || !self.outcomes.is_empty())
                && self
                    .projection
                    .as_ref()
                    .is_none_or(|projection| projection.matches_instruction_boundaries())
            {
                branch.scope.run(game, ctx, |game, ctx| {
                    crate::effects::match_triggers_at_instruction_boundary(
                        game,
                        ctx,
                        Some(effect),
                        self.outcomes
                            .iter()
                            .flat_map(|outcome| outcome.events.iter()),
                    )?;
                    Ok(())
                })?;
            }
            let mut scope = branch.scope.clone();
            if let Some(child) = &branch.child_scope {
                let mut inner = child.clone();
                inner.prepend_outer(Some(Box::new(scope)));
                scope = inner;
            }
            if self.repetition == 0 && self.next == 0 {
                if let Some(first) = &branch.first_scope {
                    let mut inner = first.clone();
                    inner.prepend_outer(Some(Box::new(scope)));
                    scope = inner;
                }
            }
            let mut identity = branch.identity.clone();
            identity.extend([self.repetition, self.next]);
            self.ends_unit = self.unit_ends[self.branch].contains(&self.next);
            self.pending_child = Some((self.branch, self.next));
            self.next += 1;
            return Ok(Some(ProgramAction {
                native: None,
                effect: effect.clone(),
                identity,
                scope,
            }));
        }
    }
    fn accept_action(&mut self, child: CompletedEffectOutputs) -> Result<(), ExecutionError> {
        let (branch, index) = self.pending_child.take().ok_or_else(|| {
            ExecutionError::InternalError("branch acknowledged without a selected child".into())
        })?;
        let outcome = if let Some(projection) = &self.projection {
            projection.project_child(&self.branches[branch].effects[index], child.outcome.clone())
        } else {
            child.outcome.clone()
        };
        self.outcomes.push(outcome);
        self.outputs.retain_owned_child(child);
        Ok(())
    }
    fn ends_action_unit(&self) -> bool {
        self.ends_unit
    }
    fn finish(self: Box<Self>) -> Result<ProgramCompletion, ExecutionError> {
        let mut outputs = if self.outcomes.is_empty() {
            CompletedEffectOutputs::aggregate_only(self.projection.as_ref().map_or_else(
                || EffectOutcome::count(0),
                |projection| projection.empty_outcome(),
            ))
        } else {
            self.outputs
                .project_aggregate(EffectOutcome::aggregate(self.outcomes))
        };
        let facts = if let Some(projection) = self.projection {
            let aggregate = projection.complete_outcome(outputs.outcome.clone());
            outputs = outputs.project_aggregate(aggregate);
            projection.completion_facts()
        } else {
            Vec::new()
        };
        Ok(ProgramCompletion { outputs, facts })
    }
}

struct ClauseProjection;
impl SelectedBranchProjection for ClauseProjection {
    fn empty_outcome(&self) -> EffectOutcome {
        EffectOutcome::aggregate(Vec::new())
    }
    fn matches_instruction_boundaries(&self) -> bool {
        false
    }
}

pub(super) fn selected_clause_cursor(
    effects: &[Effect],
    identity: Vec<usize>,
    scope: ProgramActionScope,
) -> Box<dyn crate::effects::ActionProgramCursor> {
    selected_branch_cursor_with_projection(
        vec![SelectedProgramBranch {
            effects: effects.to_vec(),
            identity,
            repetitions: 1,
            scope,
            child_scope: None,
            first_scope: None,
            match_before_first: false,
        }],
        Some(Box::new(ClauseProjection)),
    )
}
