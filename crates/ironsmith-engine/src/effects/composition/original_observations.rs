//! Actual observations from one committed original group, retained through
//! completion routing. These views never publish another event or action.
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, OriginalEffectOutput,
    SimultaneousEffectCommit, SimultaneousEffectCompletion,
};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;
use std::sync::Arc;

pub(crate) fn retain_original_observations<'a, O: OriginalEffectOutput + 'a>(
    receipts: impl IntoIterator<Item = &'a mut SimultaneousEffectCommit<O>>,
    observations: Vec<TriggerEvent>,
) {
    if observations.is_empty() {
        return;
    }
    let observations = Arc::new(observations);
    for receipt in receipts {
        retain_completion_observations(receipt, observations.clone());
    }
}

fn retain_completion_observations<O>(
    receipt: &mut SimultaneousEffectCommit<O>,
    observations: Arc<Vec<TriggerEvent>>,
) {
    if let Some(inner) = receipt.completion.take() {
        receipt.completion = Some(Box::new(ObservedOriginalCompletion {
            inner,
            observations,
        }));
    }
}

fn with_original_observations<T>(
    game: &mut GameState,
    observations: Arc<Vec<TriggerEvent>>,
    body: impl FnOnce(&mut GameState) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let depth = game.effect_store.retained_original_observation_scopes.len();
    game.effect_store
        .retained_original_observation_scopes
        .push(observations);
    let result = body(game);
    if game.effect_store.retained_original_observation_scopes.len() != depth + 1 {
        return Err(ExecutionError::InternalError(
            "original observation scope was not restored by its nested owner".into(),
        ));
    }
    game.effect_store.retained_original_observation_scopes.pop();
    result
}

struct ObservedOriginalCompletion {
    inner: Box<dyn SimultaneousEffectCompletion>,
    observations: Arc<Vec<TriggerEvent>>,
}

impl crate::effects::composition::CompletionDecorator for ObservedOriginalCompletion {
    fn phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn freeze_inner(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        with_original_observations(game, self.observations.clone(), |game| {
            self.inner.freeze(game)
        })
    }

    fn observe_inner(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        with_original_observations(game, self.observations.clone(), |game| {
            self.inner.observe_original(game, ctx, original)
        })
    }

    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: crate::effects::composition::CompletionPhase,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        let observations = self.observations.clone();
        let mut receipt = with_original_observations(game, observations.clone(), |game| {
            phase.dispatch(self.inner, game, ctx)
        })?;
        retain_completion_observations(&mut receipt, observations);
        Ok(receipt)
    }

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        with_original_observations(game, self.observations.clone(), |game| {
            original.dispatch(self.inner, game, ctx)
        })
    }
}

/// Optionally replace the aggregate summary without dropping observations,
/// facts, or the exact original instruction result. None preserves both results.
pub(crate) fn project_outcome_summary(
    mut outcome: crate::effect::EffectOutcome,
    project: impl FnOnce(&crate::effect::EffectOutcome) -> Option<crate::effect::EffectOutcome>,
) -> crate::effect::EffectOutcome {
    if let Some(summary) = project(&outcome) {
        outcome.status = summary.status;
        outcome.value = summary.value;
    }
    outcome
}
