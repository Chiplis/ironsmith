//! Recorded execution boundary for a captured damage occurrence.

use crate::effect::EffectOutcome;
use crate::effects::{
    EffectExecutor, ExecutionContext, ExecutionContextCheckpoint, ExecutionError,
};
use crate::events::processing::{
    SimultaneousDamageEvent, with_deferred_prevention_follow_up_outputs,
};
use crate::game_state::GameState;
use crate::provenance::ProvNodeId;

/// Frozen authored assignments that can join one simultaneous damage action.
/// Inputs retain opaque checkpoint identities; wrappers never rebuild them.
#[derive(Debug, Clone, Default)]
pub struct DamageActionInputs {
    pub(super) inputs: Vec<super::multi_source_damage::CapturedDamageInput>,
}

impl DamageActionInputs {
    /// Seal replacement and consequence proposals before any sibling original
    /// mutates the world. The enclosing action owns suspension and rollback.
    pub(crate) fn seal(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let batch = game.simultaneous_action_batch();
        self.seal_with_batch(game, ctx, batch)
    }

    /// Authored instruction adapters retain their ordinary occurrence policy.
    /// Preparation and physical original commitment still have one owner.
    pub(super) fn seal_with_batch(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        batch: Option<ProvNodeId>,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let prepared = if self.inputs.is_empty() {
            None
        } else {
            super::multi_source_damage::prepare_captured_damage_batch(
                game,
                ctx,
                self.inputs,
                batch,
            )?
        };
        Ok(crate::effects::outcome_recording::record_proposal(
            Box::new(SealedDamageAction { prepared }),
            None,
        ))
    }
    pub(crate) fn collect(inputs: impl IntoIterator<Item = Option<Self>>) -> Option<Self> {
        let mut combined = Self::default();
        for inputs in inputs {
            combined.inputs.extend(inputs?.inputs);
        }
        Some(combined)
    }

    pub(crate) fn execute(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // Finished/invalid instructions bind their own results without
        // creating a physical damage action or allocating its identity.
        if self.inputs.is_empty() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let batch = game.simultaneous_action_batch();
        execute_captured_damage_batch_with_outputs(game, ctx, self.inputs, batch)
    }
}

struct SealedDamageAction {
    prepared: Option<super::multi_source_damage::PreparedDamageBatch>,
}

impl std::fmt::Debug for SealedDamageAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SealedDamageAction")
            .finish_non_exhaustive()
    }
}

impl crate::effects::SimultaneousEffectProposal for SealedDamageAction {
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        match self.prepared {
            Some(prepared) => prepared.commit_original_with_outputs(game, ctx),
            None => Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            )),
        }
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original(self, game, ctx)
    }
}

/// Compose prepared instructions through one damage owner. The parent owns
/// rollback; no binding view contributes a second copy of physical history.
pub(crate) fn complete_prepared_damage_action(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut proposal: Box<dyn crate::effects::SimultaneousEffectProposal>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    proposal.prepare_selection(game, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    proposal.prepare_original(game, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let inputs = proposal.damage_action_inputs().ok_or_else(|| {
        ExecutionError::InternalError(
            "prepared shared damage action lost its contribution contract".into(),
        )
    })?;
    let opened = game.open_simultaneous_action();
    let result = inputs.execute(game, ctx);
    game.close_simultaneous_action(opened);
    let mut outputs = result?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let binding = proposal.bind_damage_action(game, ctx, &outputs)?;
    let binding = binding.transfer_owned_outputs(&mut outputs);
    let observations = outputs.outcome.clone();
    Ok(outputs.project_aggregate(binding.with_authoritative_observations(observations)))
}

#[derive(Debug, Clone)]
enum DamageBatchInputs {
    Events(Vec<SimultaneousDamageEvent>),
    Captured(Vec<super::multi_source_damage::CapturedDamageInput>),
}

#[derive(Debug, Clone)]
struct DamageBatch {
    inputs: DamageBatchInputs,
    batch: Option<ProvNodeId>,
}

impl EffectExecutor for DamageBatch {
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Reborrow the decision maker for the prevention-deferral owner,
                // preserving every owned context field across the batch callback.
                let captured = ExecutionContextCheckpoint::capture(ctx);
                let source = ctx.source;
                let controller = ctx.controller;
                let mut completed_context = None;
                let result = with_deferred_prevention_follow_up_outputs(
                    game,
                    ctx.decision_maker,
                    |game, dm| {
                        let mut parent = ExecutionContext::new(source, controller, dm);
                        captured.restore_ref(&mut parent);
                        let result = match &self.inputs {
                            DamageBatchInputs::Events(events) => super::multi_source_damage::commit_damage_batch_with_outputs(game, &mut parent, events.clone(), self.batch),
                            DamageBatchInputs::Captured(inputs) => super::multi_source_damage::commit_captured_damage_batch_with_outputs(game, &mut parent, inputs.clone(), self.batch),
                        };
                        completed_context = Some(ExecutionContextCheckpoint::capture(&parent));
                        result
                    },
                );
                if let Some(completed) = completed_context {
                    completed.restore(ctx);
                }
                result
            },
        )
    }
}

pub(crate) fn execute_damage_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<SimultaneousDamageEvent>,
    batch: Option<ProvNodeId>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_damage_batch_with_outputs(game, ctx, events, batch)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn execute_damage_batch_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<SimultaneousDamageEvent>,
    batch: Option<ProvNodeId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    DamageBatch {
        inputs: DamageBatchInputs::Events(events),
        batch,
    }
    .execute_child_with_outputs(game, ctx)
}

/// Prepared participants retain their authored assignment scopes through the
/// same recorded execution, prevention-deferral and rollback boundary.
pub(super) fn execute_captured_damage_batch_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    inputs: Vec<super::multi_source_damage::CapturedDamageInput>,
    batch: Option<ProvNodeId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    DamageBatch {
        inputs: DamageBatchInputs::Captured(inputs),
        batch,
    }
    .execute_child_with_outputs(game, ctx)
}
