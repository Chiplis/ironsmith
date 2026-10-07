//! May effect implementation.

use crate::decision::FallbackStrategy;
use crate::effect::{Effect, EffectOutcome, ExecutionFact, OutcomeValue};
#[cfg(test)]
use crate::effects::execute_effect;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;

// An object-selection prelude supplies references for the action; its object
// payload must not erase that action's numeric result (for example, how many
// permanents were sacrificed). Keep all choice facts and retain a standalone
// choice's result when the optional program contains only choices.
pub(crate) fn is_object_selection(effect: &Effect) -> bool {
    effect.0.is_object_selection_prelude()
}

struct OptionalBranchProjection {
    has_action: bool,
}
impl super::branch_program::SelectedBranchProjection for OptionalBranchProjection {
    fn empty_outcome(&self) -> EffectOutcome {
        EffectOutcome::aggregate(Vec::<EffectOutcome>::new())
    }
    fn project_child(&self, effect: &Effect, mut outcome: EffectOutcome) -> EffectOutcome {
        if self.has_action && is_object_selection(effect) {
            outcome.set_value(OutcomeValue::None);
        }
        outcome
    }
    fn complete_outcome(&self, outcome: EffectOutcome) -> EffectOutcome {
        outcome.with_execution_fact(ExecutionFact::Accepted)
    }
    fn completion_facts(&self) -> Vec<ExecutionFact> {
        vec![ExecutionFact::Accepted]
    }
}

fn optional_branch_cursor(
    effects: &[Effect],
    player: Option<Option<PlayerId>>,
) -> Box<dyn crate::effects::ActionProgramCursor> {
    super::branch_program::selected_branch_cursor_with_projection(
        vec![super::branch_program::SelectedProgramBranch {
            effects: effects.to_vec(),
            identity: vec![0],
            repetitions: 1,
            scope: crate::effects::ProgramActionScope {
                iterated_player: player,
                // Selection consumes the offer guard. An enclosing first-child
                // scope must not reintroduce it for nested offers in the body.
                optional_identity_guard: Some(None),
                ..Default::default()
            },
            // Optionality applies to child actions. Boundary matching runs in
            // the enclosing optionality and the selected participant scope.
            child_scope: Some(crate::effects::ProgramActionScope {
                optional_action: Some(true),
                ..Default::default()
            }),
            first_scope: None,
            match_before_first: false,
        }],
        Some(Box::new(OptionalBranchProjection {
            has_action: effects.iter().any(|effect| !is_object_selection(effect)),
        })),
    )
}

fn execute_optional_effects_with_outputs(
    effects: &[Effect],
    pay_as_cost: bool,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if pay_as_cost {
        let cost = crate::costs::Cost::try_effects(effects.iter().cloned())
            .map_err(ExecutionError::InternalError)?;
        let payer = ctx.iteration.iterated_player.unwrap_or(ctx.controller);
        if let ironsmith_core::TotalCostKind::All(components) = cost.kind()
            && components.iter().all(|component| component.0.supports_prepared_payment()) {
            let prepared = crate::costs::prepare_total_cost(&cost, game, ctx, payer, crate::costs::PaymentReason::Effect)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
            }
            let Some(prepared) = prepared else {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()));
            };
            let simultaneous = prepared.has_simultaneous_originals();
            let outputs = super::complete_prepared_original_with_outputs(prepared, game, ctx, simultaneous)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
            }
            let aggregate = EffectOutcome::aggregate_with_primary_result(
                EffectOutcome::count(1).with_execution_fact(ExecutionFact::Accepted),
                [outputs.outcome.clone()],
            );
            return Ok(outputs.project_aggregate(aggregate));
        }
        return match crate::special_actions::pay_total_cost_with_choice_in_context(
            game, payer, ctx.source, &cost, crate::costs::PaymentReason::Effect, ctx,
        ) {
            Ok(()) if !ctx.decision_maker.awaiting_choice() => Ok(
                crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(1).with_execution_fact(ExecutionFact::Accepted))),
            Ok(()) => Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))),
            Err(crate::cost::CostPaymentError::ExecutionFailed(error)) => Err(error),
            Err(_) => Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible())),
        };
    }
    super::action_program::execute_action_program_with_outputs(
        optional_branch_cursor(effects, None),
        game,
        ctx,
        purpose,
    )
}

/// Effect that offers an optional choice to the player.
///
/// "You may X" - the player can choose whether to execute the effects.
///
/// # Fields
///
/// * `effects` - The optional effects to execute if accepted
/// * `fallback` - Strategy when no decision maker is present (default: Decline)
///
/// # Result
///
/// - If player declines: `crate::effect::OutcomeStatus::Declined`
/// - If player accepts: the result of the last inner effect (or Count(0) if no effects)
///
/// # Example
///
/// ```ignore
/// // "You may draw a card"
/// let effect = MayEffect::new(vec![Effect::draw(1)]);
///
/// // "You may sacrifice a creature" - composed with ChooseObjectsEffect
/// let effect = MayEffect::new(vec![
///     Effect::choose_objects(ObjectFilter::creature().you_control(), 1, PlayerFilter::You, "sac"),
///     Effect::sacrifice(ChooseSpec::tagged("sac")),
/// ]);
///
/// // With auto-accept fallback
/// let effect = MayEffect::new(vec![Effect::draw(1)]).with_fallback(FallbackStrategy::Accept);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MayEffect {
    /// The optional effects to execute.
    pub effects: Vec<Effect>,
    /// Optional explicit decider for "that player may ..." patterns.
    pub decider: Option<PlayerFilter>,
    /// Strategy when no decision maker is present.
    pub fallback: FallbackStrategy,
    /// Execute all children as one TotalCost transaction.
    pub pay_as_cost: bool,
}

pub(crate) struct PreparedOptionalExecution {
    pub(crate) previous_iterated_player: Option<PlayerId>,
}

#[derive(Debug, Clone)]
struct AcceptedOptionalAction {
    identity_guard: Option<crate::effects::context::OptionalIdentityGuard>,
    limit: Option<crate::effects::DoThisLimit>,
    player: Option<PlayerId>,
}

pub(super) fn record_optional_limit(game: &mut GameState, limit: crate::effects::DoThisLimit) {
    game.record_do_this_action(limit.source, limit.trigger_identity);
}

impl AcceptedOptionalAction {
    fn record(&self, game: &mut GameState) {
        if let Some(guard) = &self.identity_guard {
            game.record_hidden_identity_obligations(
                &[guard.object],
                &guard.filter,
                &guard.filter_ctx,
                "accepted conditional reveal",
            );
        }
        if let Some(limit) = self.limit {
            record_optional_limit(game, limit);
        }
    }
}

impl MayEffect {
    /// Create a new May effect with default Decline fallback.
    pub fn new(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            decider: None,
            fallback: FallbackStrategy::Decline,
            pay_as_cost: false,
        }
    }

    /// Create a new May effect where a specific player decides.
    pub fn new_for_player(effects: Vec<Effect>, decider: PlayerFilter) -> Self {
        Self {
            effects,
            decider: Some(decider),
            fallback: FallbackStrategy::Decline,
            pay_as_cost: false,
        }
    }

    /// Create a new May effect from a single effect (convenience).
    pub fn single(effect: Effect) -> Self {
        Self::new(vec![effect])
    }

    /// Set the fallback strategy for when no decision maker is present.
    pub fn with_fallback(mut self, fallback: FallbackStrategy) -> Self {
        self.fallback = fallback;
        self
    }

    pub fn with_pay_as_cost(mut self, pay_as_cost: bool) -> Self {
        self.pay_as_cost = pay_as_cost;
        self
    }

    /// Choose an optional action without executing its children. Simultaneous
    /// owners retain this answer while scheduling the children's action units.
    pub(super) fn prepare_optional_choice(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<bool, ExecutionError> {
        let Some(accepted) = self.prepare_optional_decision(game, ctx)? else {
            return Ok(false);
        };
        accepted.record(game);
        Ok(true)
    }

    /// Capture the acceptance claim without mutating the original world. The
    /// same decision owner is used by live, prepared and scheduled programs.
    fn prepare_optional_decision(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<AcceptedOptionalAction>, ExecutionError> {
        // Direct native preparation can enter this owner without ordinary
        // dispatch. Bind only its chooser role, never an untaken child's role.
        if self.decider.as_ref().is_some_and(|decider| decider.mentions_player_filter(&PlayerFilter::Defending))
            && !ctx.bind_defending_player(game)? { return Ok(None); }
        let identity_guard = ctx.optional_identity_guard.take();
        // "Do this only once each turn" governs the ability's first optional
        // instruction. Once it has been performed the limit's number of times
        // this turn, it is no longer offered; declining doesn't count.
        let do_this_limit = ctx.do_this_limit.take();
        if do_this_limit.is_some_and(|limit| limit.reached(game)) {
            return Ok(None);
        }
        if self.should_auto_decline_without_prompt(game, ctx)? {
            return Ok(None);
        }

        // Prefer a friendly search prompt over the raw compiled lowering text
        // for same-name library searches like Doubling Chant.
        let description = self
            .effects
            .first()
            .and_then(|effect| {
                let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
                if !choose.is_search {
                    return None;
                }
                let max = choose.count.max.unwrap_or(choose.count.min);
                super::choose_objects_runtime::friendly_same_name_search_prompt(
                    game,
                    ctx,
                    &choose.filter,
                    choose.count.min,
                    max,
                )
            })
            .unwrap_or_else(|| self.prompt_description(game, ctx));

        // Use explicit decider when present ("that player may ..."), otherwise
        // preserve established behavior: iterated player if set, then controller.
        let deciding_player = if let Some(decider) = &self.decider {
            crate::effects::helpers::resolve_player_filter_as_chooser(game, decider, ctx)?
        } else {
            ctx.iteration.iterated_player.unwrap_or(ctx.controller)
        };

        let should_do = crate::decisions::make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            deciding_player,
            Some(ctx.source),
            crate::decisions::MaySpec::new(ctx.source, description)
                .with_can_accept(identity_guard.as_ref().is_none_or(|guard| guard.can_accept)),
            self.fallback,
        );

        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if !should_do {
            return Ok(None);
        }
        if identity_guard
            .as_ref()
            .is_some_and(|guard| !guard.can_accept)
        {
            return Err(ExecutionError::InvalidTarget);
        }
        let player = if self.decider_binds_iterated_player(ctx) {
            Some(deciding_player)
        } else {
            ctx.iteration.iterated_player
        };
        Ok(Some(AcceptedOptionalAction {
            identity_guard,
            limit: do_this_limit,
            player,
        }))
    }

    /// Live execution records accepted claims before its first child action.
    pub(crate) fn prepare_optional_execution(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<PreparedOptionalExecution>, ExecutionError> {
        let Some(accepted) = self.prepare_optional_decision(game, ctx)? else {
            return Ok(None);
        };
        accepted.record(game);
        let previous_iterated_player = ctx.iteration.iterated_player;
        ctx.iteration.iterated_player = accepted.player;
        Ok(Some(PreparedOptionalExecution {
            previous_iterated_player,
        }))
    }

    /// Outside any per-player iteration, an explicit non-controller decider
    /// ("any opponent may ...") is the player the accepted effects' "they" /
    /// "that player" name.
    fn decider_binds_iterated_player(&self, ctx: &ExecutionContext) -> bool {
        ctx.iteration.iterated_player.is_none()
            && self.decider.as_ref().is_some_and(|decider| {
                !matches!(decider, PlayerFilter::You | PlayerFilter::IteratedPlayer)
            })
    }

    /// What this offer says, phrased to follow "You may ".
    ///
    /// The optional branch was compiled from a sentence of the source's own
    /// card text, so the prompt quotes that sentence back rather than exposing
    /// the compiled structure the engine actually holds.
    fn prompt_description(&self, game: &GameState, ctx: &ExecutionContext) -> String {
        // A nested optional retargeting instruction asks its own question.
        // Describe only the actions controlled by this outer decision.
        let prompted = self
            .effects
            .iter()
            .filter(|effect| {
                !effect
                    .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
                    .is_some_and(|choose| choose.may)
            })
            .cloned()
            .collect::<Vec<_>>();
        crate::runtime_display::effect_sentences::optional_effect_prompt(
            game,
            ctx.source,
            ctx.source_snapshot.as_ref(),
            ctx.ability_index,
            &prompted,
        )
    }
}

impl EffectExecutor for MayEffect {
    fn directly_mentions_player_filter(&self, needle: &PlayerFilter) -> bool {
        self.decider.as_ref().is_some_and(|decider| decider.mentions_player_filter(needle))
    }
    fn contains_current_source_suspend_cast(&self) -> bool {
        self.effects.iter().any(|effect| effect.0.contains_current_source_suspend_cast())
    }

    fn supports_replacement_draw_continuation(&self) -> bool {
        !self.pay_as_cost && self.effects.iter().all(crate::effects::replacement::replacement_effect_supported)
    }
    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

    fn supports_prepared_action_program(&self) -> bool {
        !self.pay_as_cost && self.effects
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
        let accepted = self.prepare_optional_decision(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let Some(accepted) = accepted else {
            return Ok(Some(
                super::action_program::finished_program_completion_cursor(
                    crate::effects::ProgramCompletion {
                        outputs: crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::declined(),
                        ),
                        facts: vec![ExecutionFact::Declined],
                    },
                ),
            ));
        };
        let cursor = optional_branch_cursor(&self.effects, Some(accepted.player));
        Ok(Some(super::action_program::prepared_program_cursor(
            cursor,
            crate::effects::ProgramPreparation::new(move |game| {
                accepted.record(game);
                Ok(())
            }),
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
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
        execute_may_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        if !self.pay_as_cost { return true; }
        crate::costs::Cost::try_effects(self.effects.iter().cloned()).is_ok_and(|cost|
            matches!(cost.kind(), ironsmith_core::TotalCostKind::All(components)
                if components.iter().all(|component| component.0.supports_prepared_payment())))
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_simultaneous_player_action() {
            return Err(ExecutionError::Impossible("optional cost requires its ordinary payment owner".into()));
        }
        let offer_context = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let mut acceptance = self.prepare_optional_decision(game, ctx)?;
        let effects = if acceptance.is_some() {
            self.effects.clone()
        } else {
            Vec::new()
        };
        let iterated_player = acceptance
            .as_ref()
            .map(|accepted| accepted.player)
            .unwrap_or(ctx.iteration.iterated_player);
        let prepared = if self.pay_as_cost && acceptance.is_some() {
            let cost = crate::costs::Cost::try_effects(self.effects.iter().cloned()).map_err(ExecutionError::InternalError)?;
            crate::costs::prepare_total_cost(&cost, game, ctx, iterated_player.unwrap_or(ctx.controller), crate::costs::PaymentReason::Effect)?
        } else if self.pay_as_cost { None } else { super::prepared_branch::prepare_action_branch(
            &effects,
            game,
            ctx,
            iterated_player,
            true,
            true,
        )? };
        let rejected_payment = self.pay_as_cost && acceptance.is_some() && prepared.is_none()
            && !ctx.decision_maker.awaiting_choice();
        let accepted = acceptance.is_some();
        if rejected_payment {
            acceptance = None;
            offer_context.restore(ctx);
        }
        Ok(Box::new(MayProposal {
            rejected_payment,
            pay_as_cost: self.pay_as_cost,
            accepted,
            effects,
            prepared,
            iterated_player,
            acceptance,
        }))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

/// A player's accepted (or declined) "may" action for a simultaneous
/// each-player instruction: the choice was made at prepare time, the accepted
/// effects run in the batched commit.
#[derive(Debug)]
struct MayProposal {
    accepted: bool,
    rejected_payment: bool,
    acceptance: Option<AcceptedOptionalAction>,
    prepared: Option<Box<dyn crate::effects::SimultaneousEffectProposal>>,
    effects: Vec<crate::effect::Effect>,
    pay_as_cost: bool,
    iterated_player: Option<PlayerId>,
}

struct OptionalPaymentOutcome;
impl super::OriginalOutcomeAdapter for OptionalPaymentOutcome {
    fn finish(self: Box<Self>, _game: &mut GameState, _ctx: &mut ExecutionContext,
        result: Result<EffectOutcome, ExecutionError>) -> Result<EffectOutcome, ExecutionError> {
        result.map(|outcome| EffectOutcome::aggregate_with_primary_result(
            EffectOutcome::count(1).with_execution_fact(ExecutionFact::Accepted), [outcome]))
    }
}

impl crate::effects::SimultaneousEffectProposal for MayProposal {
    fn has_simultaneous_originals(&self) -> bool {
        self.prepared
            .as_ref()
            .is_some_and(|inner| inner.has_simultaneous_originals())
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        self.prepared
            .as_ref()
            .and_then(|inner| inner.nominal_payment_quantity())
    }

    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        if self.acceptance.is_some() {
            return None;
        }
        if self.effects.is_empty() {
            Some(crate::effects::damage::DamageActionInputs::default())
        } else {
            self.prepared.as_ref()?.damage_action_inputs()
        }
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        if self.acceptance.is_some() {
            return Err(ExecutionError::InternalError(
                "optional damage bound before acceptance preparation".into(),
            ));
        }
        if self.effects.is_empty() {
            return Ok(crate::effects::DamageActionBinding::from_outcome(
                if self.accepted {
                    EffectOutcome::resolved().with_execution_fact(ExecutionFact::Accepted)
                } else {
                    EffectOutcome::declined()
                },
            ));
        }
        let inner = self.prepared.ok_or_else(|| {
            ExecutionError::InternalError("optional damage lost its selected branch".into())
        })?;
        inner.bind_damage_action(game, ctx, owner)
    }

    fn declared_life_payment(&self) -> Option<(PlayerId, u32)> {
        self.prepared
            .as_ref()
            .and_then(|inner| inner.declared_life_payment())
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.prepared
            .as_ref()
            .map(|inner| inner.declared_payment_resources())
            .unwrap_or_default()
    }

    fn declared_life_payments(&self) -> Vec<(PlayerId, u32)> {
        self.prepared
            .as_ref()
            .map(|inner| inner.declared_life_payments())
            .unwrap_or_default()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if let Some(accepted) = self.acceptance.take() {
            accepted.record(game);
        }
        if let Some(inner) = &mut self.prepared {
            inner.prepare_selection(game, ctx)?;
        }
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if let Some(accepted) = self.acceptance.take() {
            accepted.record(game);
        }
        if let Some(inner) = &mut self.prepared {
            inner.prepare_original(game, ctx)?;
        }
        Ok(())
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if let Some(inner) = &mut self.prepared {
            inner.seal_original(game, ctx)?;
        }
        Ok(())
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
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        if self.rejected_payment {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible())));
        }
        if let Some(inner) = self.prepared.take() {
            let receipt = inner.commit_original_with_outputs(game, ctx)?;
            return if self.pay_as_cost {
                super::adapt_original_outcome_with_outputs(receipt, Box::new(OptionalPaymentOutcome), game, ctx)
            } else { Ok(receipt) };
        }
        if self.pay_as_cost && self.accepted && !ctx.decision_maker.awaiting_choice() {
            return Err(ExecutionError::InternalError("accepted optional cost lost its prepared total".into()));
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let outcome = if !self.accepted {
            Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::declined(),
            ))
        } else {
            ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
                execute_optional_effects_with_outputs(
                    &self.effects,
                    self.pay_as_cost,
                    game,
                    ctx,
                    crate::effects::EffectExecutionPurpose::Action,
                )
            })
        };
        outcome.map(crate::effects::SimultaneousEffectCommit::finished)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::complete_prepared_original(self, game, ctx)
    }
}

fn execute_may_with_outputs(
    effect: &MayEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    // The complete optional instruction owns all of its child actions.
    // Pending answers cannot become a decline or publish earlier partial
    // actions; retry must retain the original context and one-shot state.
    let unpaid_checkpoint = effect.pay_as_cost.then(|| (game.clone(), crate::effects::ExecutionContextCheckpoint::capture(ctx)));
    let result = super::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let Some(prepared) = effect.prepare_optional_execution(game, ctx)? else {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::declined(),
                ));
            };
            let result = execute_optional_effects_with_outputs(&effect.effects, effect.pay_as_cost, game, ctx, purpose);
            ctx.iteration.iterated_player = prepared.previous_iterated_player;
            result
        },
    );
    if result.as_ref().is_ok_and(|outputs| !outputs.outcome.status.is_success()) {
        if let Some((checkpoint, context)) = unpaid_checkpoint {
            game.restore_execution_checkpoint(checkpoint, false);
            context.restore(ctx);
        }
    }
    result
}

impl CostExecutableEffect for MayEffect {
    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_may_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Payment,
        )
    }

    fn payment_bindings_are_owned_by_children(&self) -> bool {
        true
    }

    fn canonical_cost_effect(&self) -> Option<crate::effect::Effect> {
        let effects = crate::effects::canonical_cost_children(&self.effects)?;
        let mut replacement = self.clone();
        replacement.effects = effects;
        Some(crate::effect::Effect::new(replacement))
    }

    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        Ok(())
    }
}

impl MayEffect {
    /// Some parsed patterns compile to "may (if condition, do X)" where the
    /// condition is a strict gate (no else branch). If the gate is false, skip
    /// prompting entirely so UI doesn't offer an option that cannot do anything.
    fn should_auto_decline_without_prompt(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<bool, ExecutionError> {
        // "Any player may sacrifice two creatures of their choice" is not an
        // option for a player who controls one: an optional action has to be
        // performed in full, so a leading fixed-count choice the deciding
        // player cannot satisfy withdraws the offer instead of shrinking it.
        if let Some(choose) = self
            .effects
            .first()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && super::choose_objects_runtime::fixed_choice_requirement_is_unmet(choose, game, ctx)?
        {
            return Ok(true);
        }

        if let Some(put) = self.effects.first().and_then(|effect| {
            let mut effect = effect;
            while let Some(child) = effect.transparent_child_effect() {
                effect = child;
            }
            effect.downcast_ref::<crate::effects::PutCountersEffect>()
        }) && put.completion_action == Some(crate::events::KeywordActionKind::Blight)
        {
            return Ok(crate::effects::CostExecutableEffect::can_execute_as_cost(
                put,
                game,
                ctx.source,
                ctx.controller,
            )
            .is_err());
        }
        if let Some(evidence) = self.effects.first().and_then(|effect| {
            let mut effect = effect;
            while let Some(child) = effect.transparent_child_effect() {
                effect = child;
            }
            effect.downcast_ref::<crate::effects::CollectEvidenceEffect>()
        }) {
            let required = super::collect_evidence::evidence_requirement(evidence, game, ctx)?;
            return Ok(
                super::collect_evidence::evidence_capacity(game, ctx.controller, None) < required,
            );
        }
        if self.effects.len() != 1 {
            return Ok(false);
        }

        if let Some(pay_life) = self.effects[0].downcast_ref::<crate::effects::PayLifeEffect>() {
            let player = resolve_player_from_spec(game, &pay_life.player, ctx)?;
            let amount = resolve_value(game, &pay_life.amount, ctx)?.max(0) as u32;
            return Ok(!game.can_pay_life_with_reason(
                player,
                amount,
                crate::costs::PaymentReason::Effect,
            ));
        }

        let Some(conditional) = self.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
        else {
            return Ok(false);
        };

        if !conditional.if_false.is_empty() {
            return Ok(false);
        }

        let condition_met = crate::condition_eval::evaluate_condition_resolution(
            game,
            &conditional.condition,
            ctx,
        )?;
        Ok(!condition_met)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::PowerToughness;
    use crate::cards::tokens::lander_token_definition;
    use crate::effect::{Condition, ExecutionFact};
    use crate::effect::{EffectId, EffectPredicate};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn source_creature_definition() -> crate::cards::CardDefinition {
        crate::cards::CardDefinitionBuilder::new(CardId::new(), "Terrapact Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn terrapact_style_effects(fallback: crate::decision::FallbackStrategy) -> Vec<Effect> {
        vec![
            Effect::with_id(
                0,
                Effect::new(
                    MayEffect::new_for_player(
                        vec![Effect::create_tokens(lander_token_definition(), 2)],
                        PlayerFilter::target_opponent(),
                    )
                    .with_fallback(fallback),
                ),
            ),
            Effect::if_then(
                EffectId(0),
                EffectPredicate::DidNotHappen,
                vec![Effect::put_counters_on_source(
                    CounterType::PlusOnePlusOne,
                    2,
                )],
            ),
        ]
    }

    fn count_lander_tokens(game: &GameState, controller: PlayerId) -> usize {
        game.battlefield
            .iter()
            .filter(|&&id| {
                game.object(id).is_some_and(|obj| {
                    matches!(obj.kind, crate::object::ObjectKind::Token)
                        && obj.name == "Lander"
                        && game.controller_of(obj) == controller
                })
            })
            .count()
    }

    #[test]
    fn test_may_auto_decline_without_decision_maker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        // Use AutoPassDecisionMaker which declines boolean choices
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let initial_life = game.player(alice).unwrap().life;

        let effect = MayEffect::new(vec![Effect::gain_life(5)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        // Life should not have changed
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_may_clone_box() {
        let effect = MayEffect::new(vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MayEffect"));
    }

    #[test]
    fn test_may_with_multiple_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        // Use AutoPassDecisionMaker which declines boolean choices
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        // Create a May with multiple effects
        let effect = MayEffect::new(vec![Effect::gain_life(2), Effect::lose_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // With AutoPassDecisionMaker, should decline
        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
    }

    #[test]
    fn test_may_single_convenience() {
        let effect = MayEffect::single(Effect::gain_life(1));
        assert_eq!(effect.effects.len(), 1);
    }

    #[test]
    fn test_may_for_specific_player_constructor() {
        let effect =
            MayEffect::new_for_player(vec![Effect::draw(1)], PlayerFilter::target_player());
        assert!(matches!(effect.decider, Some(PlayerFilter::Target(_))));
    }

    #[test]
    fn may_forwards_inner_target_spec() {
        let effect = MayEffect::new(vec![Effect::counter(ChooseSpec::target_spell())]);

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[derive(Default)]
    struct PanicOnBooleanDecisionMaker;

    impl crate::decision::DecisionMaker for PanicOnBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            panic!("boolean prompt should be skipped for false guarded condition");
        }
    }

    #[test]
    fn may_skips_prompt_for_single_guarded_conditional_when_false() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = PanicOnBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let initial_life = game.player(alice).expect("alice should exist").life;

        let guarded = Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::LifeTotalOrLess(0),
            vec![Effect::gain_life(5)],
        ));
        let effect = MayEffect::new(vec![guarded]);

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(
            game.player(alice).expect("alice should exist").life,
            initial_life
        );
    }

    fn create_battlefield_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_definition(&definition, controller, Zone::Battlefield)
    }

    fn sacrifice_two_effects(chooser: PlayerFilter) -> Vec<Effect> {
        let mut chosen = crate::filter::ObjectFilter::creature();
        chosen.controller = Some(chooser.clone());
        let mut sacrificed = crate::filter::ObjectFilter::creature();
        sacrificed
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::TagKey::from("sacrificed_0"),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        vec![
            Effect::choose_objects(chosen, 2, chooser.clone(), "sacrificed_0"),
            Effect::sacrifice_player(sacrificed, 2, chooser),
        ]
    }

    /// "Any player may sacrifice two creatures of their choice" (Prowling
    /// Pangolin): a player who controls only one creature can't sacrifice two,
    /// so the option is never offered to them.
    #[test]
    fn may_sacrifice_two_is_not_offered_when_only_one_creature_is_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let lone_creature = create_battlefield_creature(&mut game, "Lone Bear", alice);

        let mut dm = PanicOnBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = MayEffect::new(sacrifice_two_effects(PlayerFilter::You))
            .with_fallback(FallbackStrategy::Accept);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert!(
            !result.execution_facts().contains(&ExecutionFact::Accepted),
            "an unperformable optional action must not count as taken"
        );
        assert!(
            game.battlefield.contains(&lone_creature),
            "the lone creature must not be sacrificed"
        );
    }

    #[test]
    fn may_sacrifice_two_is_offered_when_two_creatures_are_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_battlefield_creature(&mut game, "Bear One", alice);
        let second = create_battlefield_creature(&mut game, "Bear Two", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MayEffect::new(sacrifice_two_effects(PlayerFilter::You))
            .with_fallback(FallbackStrategy::Accept);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert!(result.execution_facts().contains(&ExecutionFact::Accepted));
        assert!(!game.battlefield.contains(&first));
        assert!(!game.battlefield.contains(&second));
    }

    /// The whole card: "any player may sacrifice two creatures of their choice.
    /// If a player does, sacrifice this creature." The controller, holding one
    /// creature, is skipped rather than allowed to half-pay, so the offer
    /// passes on to the next player in turn order.
    #[test]
    fn any_player_may_sacrifice_two_skips_players_who_control_only_one() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let alice_creature = create_battlefield_creature(&mut game, "Alice Bear", alice);
        let bob_first = create_battlefield_creature(&mut game, "Bob Bear One", bob);
        let bob_second = create_battlefield_creature(&mut game, "Bob Bear Two", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);

        let each_player = Effect::new(
            super::super::ForPlayersEffect::new_starting_with_controller(
                PlayerFilter::Any,
                vec![Effect::new(
                    MayEffect::new_for_player(
                        sacrifice_two_effects(PlayerFilter::IteratedPlayer),
                        PlayerFilter::IteratedPlayer,
                    )
                    .with_fallback(FallbackStrategy::Accept),
                )],
            )
            .stop_after_first_happened(),
        );

        execute_effect(&mut game, &each_player, &mut ctx).expect("effect should resolve");

        assert!(
            game.battlefield.contains(&alice_creature),
            "a player controlling one creature can't sacrifice two"
        );
        assert!(!game.battlefield.contains(&bob_first));
        assert!(!game.battlefield.contains(&bob_second));
    }

    #[test]
    fn may_acceptance_emits_execution_fact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            MayEffect::new(vec![Effect::gain_life(1)]).with_fallback(FallbackStrategy::Accept);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert!(result.execution_facts().contains(&ExecutionFact::Accepted));
    }

    #[test]
    fn terrapact_style_choice_creates_landers_when_accepted() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(
            &source_creature_definition(),
            alice,
            Zone::Battlefield,
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);

        for effect in terrapact_style_effects(crate::decision::FallbackStrategy::Accept) {
            execute_effect(&mut game, &effect, &mut ctx).expect("effect should resolve");
        }

        assert_eq!(count_lander_tokens(&game, alice), 2);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
    }
}
