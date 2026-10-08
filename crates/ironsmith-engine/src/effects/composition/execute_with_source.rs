//! Execute an effect while temporarily treating another object as the source.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_effect_source_with_lki;
use crate::effects::{CostExecutableEffect, CostValidationError, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
pub type ExecuteWithSourceEffect = ironsmith_core::ExecuteWithSourceEffect<Effect>;

/// Freeze the source and LKI used by the complete child action.
pub(crate) fn resolve_source_binding(
    effect: &ExecuteWithSourceEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)> {
    let resolved = resolve_effect_source_with_lki(game, ctx, &effect.source);
    finish_source_binding(effect, game, ctx, resolved)
}

fn finish_source_binding(
    effect: &ExecuteWithSourceEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    resolved: Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)>,
) -> Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)> {
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    // The source can leave the battlefield before this effect runs: a
    // sacrifice and its own reflexive trigger are the printed case. The
    // ability still resolves from the source's last known information
    // (CR 608.2h), which the stack entry already carries, so rebinding is
    // simply a no-op there rather than a failure.
    let rebind_to_own_source_lki = matches!(effect.source.base(), ChooseSpec::Source)
        && ctx.source_snapshot.is_some()
        && game.object(ctx.source).is_none();
    let Some((source_id, tagged_snapshot)) = resolved else {
        if rebind_to_own_source_lki {
            return Some((ctx.source, ctx.source_snapshot.clone()));
        }
        return None;
    };
    let source_snapshot = match game.object(source_id) {
        // A tagged source keeps its tagged last known information even
        // after it left (CR 608.2h); that snapshot is authoritative.
        _ if tagged_snapshot.is_some() => tagged_snapshot,
        Some(source_obj) => match effect.source.base() {
            ChooseSpec::Source => ctx.source_snapshot.as_ref().and_then(|snapshot| {
                // Rebinding an effect to its own source must not replace the
                // stack entry's battlefield LKI with the counter-cleared card
                // object now in a graveyard (or another destination zone).
                (snapshot.stable_id == source_obj.stable_id
                    && (snapshot.object_id != source_obj.id || snapshot.zone != source_obj.zone))
                    .then(|| snapshot.clone())
            }),
            _ => None,
        }
        .or_else(|| Some(ObjectSnapshot::from_object(source_obj, game))),
        None if rebind_to_own_source_lki => return Some((ctx.source, ctx.source_snapshot.clone())),
        None => return None,
    };

    Some((source_id, source_snapshot))
}

type SourceBinding = (crate::ids::ObjectId, Option<ObjectSnapshot>);

/// Source scopes replace only the source pair. Other successful context
/// outputs, including tags, results and mana, remain owned by the participant.
fn enter_source_binding(ctx: &mut ExecutionContext, binding: &SourceBinding) -> SourceBinding {
    let previous = (ctx.source, ctx.source_snapshot.take());
    ctx.source = binding.0;
    ctx.source_snapshot = binding.1.clone();
    previous
}

fn restore_source_binding(ctx: &mut ExecutionContext, previous: SourceBinding) {
    ctx.source = previous.0;
    ctx.source_snapshot = previous.1;
}

pub(super) fn with_source_binding<T>(
    ctx: &mut ExecutionContext,
    binding: &SourceBinding,
    f: impl FnOnce(&mut ExecutionContext) -> T,
) -> T {
    let previous = enter_source_binding(ctx, binding);
    let result = f(ctx);
    restore_source_binding(ctx, previous);
    result
}

/// A selected program keeps its source pair in the participant's retained
/// execution frame for the entire body. Actual child source exports remain
/// visible to later units; restore the enclosing pair only after completion.
struct RestoreEnclosingSource {
    previous: SourceBinding,
}
impl super::OriginalOutcomeAdapter for RestoreEnclosingSource {
    fn finish(
        self: Box<Self>,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<EffectOutcome, ExecutionError>,
    ) -> Result<EffectOutcome, ExecutionError> {
        restore_source_binding(ctx, self.previous);
        result
    }
}

#[derive(Debug)]
struct SourceProposal {
    binding: Option<(crate::ids::ObjectId, Option<ObjectSnapshot>)>,
    inner: Option<Box<dyn crate::effects::SimultaneousEffectProposal>>,
}
/// Deferred programs use the same source-only scope as ordinary execution.
/// Keep other context outputs visible to the enclosing participant adapter.
struct SourceOriginalCompletion {
    binding: (crate::ids::ObjectId, Option<ObjectSnapshot>),
    inner: Box<dyn crate::effects::SimultaneousEffectCompletion>,
}

impl crate::effects::SimultaneousEffectCompletion for SourceOriginalCompletion {
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
        let Self { binding, inner } = *self;
        let mut receipt = with_source_binding(ctx, &binding, |ctx| {
            inner.complete_original_phase_with_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { binding, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
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
        let Self { binding, inner } = *self;
        let mut receipt = with_source_binding(ctx, &binding, |ctx| {
            inner.complete_original_phase_from_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { binding, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
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
        let Self { binding, inner } = *self;
        let mut receipt = with_source_binding(ctx, &binding, |ctx| {
            inner.prepare_draw_boundary_with_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { binding, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
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
        let Self { binding, inner } = *self;
        let mut receipt = with_source_binding(ctx, &binding, |ctx| {
            inner.prepare_draw_boundary_from_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { binding, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        with_source_binding(ctx, &self.binding, |ctx| {
            self.inner.observe_original(game, ctx, original)
        })
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self { binding, inner } = *self;
        with_source_binding(ctx, &binding, |ctx| {
            inner.complete_with_outputs(game, ctx, original)
        })
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self { binding, inner } = *self;
        with_source_binding(ctx, &binding, |ctx| {
            inner.complete_from_original_outputs(game, ctx, original)
        })
    }
}

impl crate::effects::SimultaneousEffectProposal for SourceProposal {
    fn has_simultaneous_originals(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.has_simultaneous_originals())
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        self.inner
            .as_ref()
            .and_then(|inner| inner.nominal_payment_quantity())
    }

    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        match &self.inner {
            Some(inner) => inner.damage_action_inputs(),
            None => Some(crate::effects::damage::DamageActionInputs::default()),
        }
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        let (Some(binding), Some(inner)) = (self.binding, self.inner) else {
            return Ok(crate::effects::DamageActionBinding::from_outcome(
                EffectOutcome::target_invalid(),
            ));
        };
        with_source_binding(ctx, &binding, |ctx| {
            inner.bind_damage_action(game, ctx, owner)
        })
    }
    fn declared_life_payment(&self) -> Option<(crate::ids::PlayerId, u32)> {
        self.inner
            .as_ref()
            .and_then(|inner| inner.declared_life_payment())
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.inner
            .as_ref()
            .map(|inner| inner.declared_payment_resources())
            .unwrap_or_default()
    }

    fn declared_life_payments(&self) -> Vec<(crate::ids::PlayerId, u32)> {
        self.inner
            .as_ref()
            .map(|inner| inner.declared_life_payments())
            .unwrap_or_default()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        let (Some(binding), Some(inner)) = (&self.binding, &mut self.inner) else {
            return Ok(());
        };
        with_source_binding(ctx, binding, |ctx| inner.prepare_selection(game, ctx))
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        let (Some(binding), Some(inner)) = (&self.binding, &mut self.inner) else {
            return Ok(());
        };
        with_source_binding(ctx, binding, |ctx| inner.prepare_original(game, ctx))
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        let (Some(binding), Some(inner)) = (&self.binding, &mut self.inner) else {
            return Ok(());
        };
        with_source_binding(ctx, binding, |ctx| inner.seal_original(game, ctx))
    }

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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let (Some(binding), Some(inner)) = (self.binding, self.inner) else {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::target_invalid(),
                ),
            ));
        };
        let mut receipt = with_source_binding(ctx, &binding, |ctx| {
            inner.commit_original_with_outputs(game, ctx)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(SourceOriginalCompletion { binding, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (Some(binding), Some(inner)) = (self.binding, self.inner) else {
            return Ok(EffectOutcome::target_invalid());
        };
        with_source_binding(ctx, &binding, |ctx| inner.commit(game, ctx))
    }
}

fn execute_source_program_with_outputs(
    effect: &ExecuteWithSourceEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment)
        && !ctx.decision_maker.awaiting_choice()
    {
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
    }
    let source_binding = resolve_source_binding(effect, game, ctx);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let Some(binding) = source_binding else {
        if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
            return Err(ExecutionError::Impossible(
                "source payment has no bound source".into(),
            ));
        }
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::target_invalid(),
        ));
    };
    with_source_binding(ctx, &binding, |ctx| {
        if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
            let reason = ctx
                .mana
                .payment_reason
                .unwrap_or(crate::costs::PaymentReason::Other);
            crate::costs::check_effect_cost_program(
                std::slice::from_ref(effect.effect.as_ref()),
                game,
                ctx,
                reason,
            )
            .map_err(|error| {
                ExecutionError::Impossible(format!("source payment is unavailable: {error:?}"))
            })?;
        }
        purpose.execute(game, &effect.effect, ctx)
    })
}

impl EffectExecutor for ExecuteWithSourceEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        crate::effects::replacement::replacement_effect_supported(&self.effect)
    }
    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

    fn supports_prepared_action_program(&self) -> bool {
        // The mutable selection owner supports chooser-bearing source specs as
        // well as locked sources. Child capabilities still preserve domain
        // boundaries such as sequential draws and unsupported containers.
        super::action_program::action_program_child_is_prepared(&self.effect)
    }
    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        let binding = resolve_source_binding(self, game, ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let Some(binding) = binding else {
            return Ok(Some(super::action_program::finished_program_cursor(
                crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::target_invalid(),
                ),
            )));
        };
        let previous = enter_source_binding(ctx, &binding);
        Ok(Some(super::action_program::adapted_child_program_cursor(
            self.effect.as_ref().clone(),
            Box::new(RestoreEnclosingSource { previous }),
        )))
    }

    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        let mut bindings = crate::effects::CostChoiceBindings::from_spec(&self.source);
        bindings.append(self.effect.0.cost_choice_bindings());
        bindings
    }

    fn transparent_cost_precheck_child_effect(&self) -> Option<&Effect> {
        None
    }

    fn max_cost_x(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Option<u32> {
        let ctx = ExecutionContext::new_default(source, controller);
        let resolved = crate::effects::helpers::resolve_effect_source_from_spec_with_lki(
            game,
            &ctx,
            &self.source,
        );
        let binding = finish_source_binding(self, game, &ctx, resolved)?;
        self.effect.max_cost_x(game, binding.0, controller)
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        // This capability promises a bound source to cost preflight. Mutable
        // source choices need their own selection/feasibility contract.
        (crate::effects::helpers::source_binding_is_locked(&self.source)
            && self.effect.0.as_cost_executable().is_some())
        .then_some(self as &dyn CostExecutableEffect)
    }

    fn supports_damage_action_cohort(&self) -> bool {
        self.supports_simultaneous_player_action() && self.effect.0.supports_damage_action_cohort()
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        crate::effects::helpers::source_binding_is_locked(&self.source)
            && self.effect.0.supports_simultaneous_player_action()
    }
    fn is_read_only_simultaneous_player_action(&self) -> bool {
        crate::effects::helpers::source_binding_is_locked(&self.source)
            && (self.source.is_target() || !matches!(self.source.base(), ChooseSpec::Iterated))
            && self.effect.0.is_read_only_simultaneous_player_action()
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_simultaneous_player_action() {
            return Err(ExecutionError::Impossible("chooser-bearing source scope requires the mutable action-program preparation owner".into()));
        }
        let resolved = crate::effects::helpers::resolve_effect_source_from_spec_with_lki(
            game,
            ctx,
            &self.source,
        );
        let binding = finish_source_binding(self, game, ctx, resolved);
        let inner = match &binding {
            Some(binding) => Some(with_source_binding(ctx, binding, |ctx| {
                self.effect.prepare_simultaneous_player_action(game, ctx)
            })?),
            None => None,
        };
        Ok(Box::new(SourceProposal { binding, inner }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.effect);
    }

    fn transparent_child_effect(&self) -> Option<&Effect> {
        Some(&self.effect)
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
        execute_source_program_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.effect.0.get_target_spec()
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        self.effect.0.decision_related_object_specs()
    }

    fn target_description(&self) -> &'static str {
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.effect.0.get_target_count()
    }
}

impl CostExecutableEffect for ExecuteWithSourceEffect {
    fn cost_choice_tap_state(&self) -> Option<bool> {
        None
    }

    fn cost_choice_candidate_is_eligible(
        &self,
        game: &GameState,
        execution: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
        tag: &crate::tag::TagKey,
        object: crate::ids::ObjectId,
    ) -> Option<bool> {
        // A source drawn from this prospective selection is not bound yet.
        // The complete group query validates that source after the witness
        // is available; do not substitute an older collection with this tag.
        if matches!(self.source.base(), ChooseSpec::Tagged(source_tag) if source_tag == tag) {
            return None;
        }
        let resolved = crate::effects::helpers::resolve_effect_source_from_spec_with_lki(
            game,
            execution,
            &self.source,
        );
        let binding = finish_source_binding(self, game, execution, resolved)?;
        with_source_binding(execution, &binding, |execution| {
            self.effect
                .0
                .as_cost_executable()?
                .cost_choice_candidate_is_eligible(game, execution, reason, tag, object)
        })
    }

    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_source_program_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Payment,
        )
    }

    fn payment_bindings_are_owned_by_children(&self) -> bool {
        true
    }

    fn supports_prepared_payment(&self) -> bool {
        crate::effects::helpers::source_binding_is_locked(&self.source)
            && self
                .effect
                .0
                .as_cost_executable()
                .is_some_and(|cost| cost.supports_prepared_payment())
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_prepared_payment() {
            return Err(ExecutionError::Impossible(
                "source payment lacks a bound prepared owner".into(),
            ));
        }
        let checked = game
            .continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let resolved = crate::effects::helpers::resolve_effect_source_from_spec_with_lki(
            &checked,
            ctx,
            &self.source,
        );
        let binding = finish_source_binding(self, &checked, ctx, resolved);
        let inner = match &binding {
            Some(binding) => with_source_binding(ctx, binding, |ctx| {
                let reason = ctx
                    .mana
                    .payment_reason
                    .unwrap_or(crate::costs::PaymentReason::Other);
                crate::costs::check_effect_cost_program(
                    std::slice::from_ref(self.effect.as_ref()),
                    &checked,
                    ctx,
                    reason,
                )
                .map_err(|error| {
                    ExecutionError::Impossible(format!("source payment is unavailable: {error:?}"))
                })?;
                super::prepared_branch::prepare_action_for_purpose(
                    &self.effect,
                    crate::effects::EffectExecutionPurpose::Payment,
                    &checked,
                    ctx,
                )
            })?,
            None if ctx.decision_maker.awaiting_choice() => None,
            None => {
                return Err(ExecutionError::Impossible(
                    "source payment has no bound source".into(),
                ));
            }
        };
        Ok(Box::new(SourceProposal { binding, inner }))
    }

    // The singleton prepared total acknowledges its actual child inside the
    // captured source scope, before additions. Ordinary dispatch acknowledges
    // the payment child there too. Never redo source-sensitive binding exports
    // after the scope restores the caller's source.
    fn payment_x_from_outcome(
        &self,
        _outcome: &EffectOutcome,
        _execution: &ExecutionContext,
    ) -> Result<Option<u32>, CostValidationError> {
        Ok(None)
    }

    fn finalize_payment_bindings(
        &self,
        _game: &GameState,
        _outcome: &EffectOutcome,
        _execution: &mut ExecutionContext,
        _payment_x: Option<u32>,
    ) -> Result<(), crate::cost::CostPaymentError> {
        Ok(())
    }

    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        if !crate::effects::helpers::source_binding_is_locked(&self.source) {
            return Err(CostValidationError::Other(
                "source cost requires selected source inputs".into(),
            ));
        }
        let checked = game.continuous_query_snapshot().map_err(|error| {
            CostValidationError::Other(format!("source cost discovery failed: {error:?}"))
        })?;
        let resolved = crate::effects::helpers::resolve_effect_source_from_spec_with_lki(
            &checked,
            ctx,
            &self.source,
        );
        let binding = finish_source_binding(self, &checked, ctx, resolved)
            .ok_or_else(|| CostValidationError::Other("source cost has no bound source".into()))?;
        with_source_binding(ctx, &binding, |ctx| {
            crate::costs::check_effect_cost_program(
                std::slice::from_ref(self.effect.as_ref()),
                &checked,
                ctx,
                reason,
            )
        })
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            crate::costs::PaymentReason::Other,
        )
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        let mut ctx = ExecutionContext::new_default(source, controller);
        ctx.x_value = game.object(source).and_then(|object| object.x_value);
        CostExecutableEffect::can_execute_as_cost_with_context(self, game, &mut ctx, reason)
    }

    fn canonical_cost_effect(&self) -> Option<Effect> {
        let child = self
            .effect
            .0
            .as_cost_executable()?
            .canonical_cost_effect()?;
        let mut normalized = self.clone();
        normalized.effect = Box::new(child);
        Some(Effect::new(normalized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::events::DamageEvent;
    use crate::events::DamageTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn execute_with_source_uses_the_resolved_object_as_damage_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_source = game.new_object_id();
        let creature = create_creature(&mut game, "Borrowed Source", alice);
        let mut ctx = ExecutionContext::new_default(spell_source, alice);

        let effect = ExecuteWithSourceEffect::new(
            ChooseSpec::SpecificObject(creature),
            Effect::deal_damage(2, ChooseSpec::AnyTarget),
        );
        let outcome = ctx
            .with_temp_targets(vec![ResolvedTarget::Player(bob)], |ctx| {
                effect.execute(&mut game, ctx)
            })
            .expect("wrapped effect should resolve");
        let events_debug = format!("{:?}", outcome.events);

        assert!(
            outcome.events.iter().any(|event| {
                event.downcast::<DamageEvent>().is_some_and(|damage| {
                    damage.source == creature
                        && damage.amount == 2
                        && matches!(damage.target, DamageTarget::Player(player) if player == bob)
                })
            }),
            "expected damage from wrapped source, got {events_debug}"
        );
    }

    #[test]
    fn execute_with_source_returns_target_invalid_when_source_is_missing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell_source = game.new_object_id();
        let missing = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(spell_source, alice);

        let outcome =
            ExecuteWithSourceEffect::new(ChooseSpec::SpecificObject(missing), Effect::gain_life(2))
                .execute(&mut game, &mut ctx)
                .expect("missing wrapped source should return an outcome");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn execute_with_source_preserves_source_lki_after_the_source_moves() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Countered Source", alice);
        game.add_counters(source, crate::object::CounterType::Charge, 3)
            .expect("source counters");
        let snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should move");

        let mut ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        ExecuteWithSourceEffect::new(
            ChooseSpec::Source,
            Effect::gain_life(crate::effect::Value::CountersOnSource(
                crate::object::CounterType::Charge,
            )),
        )
        .execute(&mut game, &mut ctx)
        .expect("wrapped source-LKI effect should resolve");

        assert_eq!(game.player(alice).expect("Alice").life, 23);
    }
}
