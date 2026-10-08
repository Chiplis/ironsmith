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

impl SimultaneousEffectCompletion for ObservedOriginalCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let observations = self.observations.clone();
        let mut receipt = with_original_observations(game, observations.clone(), |game| {
            self.inner
                .complete_original_phase_with_outputs(game, ctx, original)
        })?;
        retain_completion_observations(&mut receipt, observations);
        Ok(receipt)
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let observations = self.observations.clone();
        let mut receipt = with_original_observations(game, observations.clone(), |game| {
            self.inner
                .complete_original_phase_from_outputs(game, ctx, original)
        })?;
        retain_completion_observations(&mut receipt, observations);
        Ok(receipt)
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        with_original_observations(game, self.observations.clone(), |game| {
            self.inner.freeze(game)
        })
    }
    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        with_original_observations(game, self.observations.clone(), |game| {
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
        with_original_observations(game, self.observations.clone(), |game| {
            self.inner.complete_with_outputs(game, ctx, original)
        })
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        with_original_observations(game, self.observations.clone(), |game| {
            self.inner
                .complete_from_original_outputs(game, ctx, original)
        })
    }
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let observations = self.observations.clone();
        let mut receipt = with_original_observations(game, observations.clone(), |game| {
            self.inner
                .prepare_draw_boundary_with_outputs(game, ctx, original)
        })?;
        retain_completion_observations(&mut receipt, observations);
        Ok(receipt)
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let observations = self.observations.clone();
        let mut receipt = with_original_observations(game, observations.clone(), |game| {
            self.inner
                .prepare_draw_boundary_from_outputs(game, ctx, original)
        })?;
        retain_completion_observations(&mut receipt, observations);
        Ok(receipt)
    }
}
