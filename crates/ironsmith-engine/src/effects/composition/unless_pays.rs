//! "Unless pays" effect implementation.

use crate::costs::Cost;
use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::special_actions::{
    can_pay_total_cost_with_reason_in_context, pay_total_cost_with_choice_in_context_with_outputs,
};
use crate::target::PlayerFilter;

// Execution failures do not establish that a legal cost cannot be paid.
fn acknowledged_payment<T>(
    result: Result<T, crate::cost::CostPaymentError>,
) -> Result<Option<T>, ExecutionError> {
    crate::costs::acknowledged_total_cost(result)
}
fn payment_succeeded(
    result: Result<(), crate::cost::CostPaymentError>,
) -> Result<bool, ExecutionError> {
    acknowledged_payment(result).map(|receipt| receipt.is_some())
}

/// Offers select willingness only. Queries and pending decisions establish
/// no payment acknowledgement and cannot start a consequence branch.
fn select_payment_offer(
    game: &GameState,
    payer: PlayerId,
    cost: &crate::cost::TotalCost,
    ctx: &mut ExecutionContext,
) -> Result<Option<bool>, ExecutionError> {
    let can_afford = payment_succeeded(can_pay_total_cost_with_reason_in_context(
        game,
        payer,
        ctx.source,
        cost,
        crate::costs::PaymentReason::Effect,
        ctx,
    ))?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let wants_to_pay = can_afford
        && make_boolean_decision(
            game,
            &mut ctx.decision_maker,
            payer,
            ctx.source,
            format!("{} to prevent effect?", cost.display()),
            FallbackStrategy::Accept,
        );
    Ok((!ctx.decision_maker.awaiting_choice()).then_some(wants_to_pay))
}

fn execute_consequences(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    declarations: Vec<crate::effects::CompletedEffectOutputs>,
    player: Option<PlayerId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let cursor = super::branch_program::selected_clause_cursor(
        effects,
        vec![0],
        crate::effects::ProgramActionScope {
            iterated_player: Some(player),
            ..Default::default()
        },
    );
    let mut outputs = super::action_program::execute_action_program_with_outputs(
        cursor,
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    outputs.retain_batch_children(declarations);
    Ok(outputs)
}

fn paid_clause_outputs(
    declarations: Vec<crate::effects::CompletedEffectOutputs>,
    payments: Vec<crate::effects::CompletedEffectOutputs>,
) -> crate::effects::CompletedEffectOutputs {
    let mut outputs =
        crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::declined());
    outputs.retain_batch_children(declarations);
    outputs.retain_published_children(payments);
    outputs
}

/// Effect that executes inner effects unless a player pays a mana cost.
///
/// "Sacrifice this creature unless you pay {U}" - the player can choose to pay
/// the mana to prevent the inner effects from happening.
///
/// # Fields
///
/// * `effects` - The effects to execute if the player does NOT pay
/// * `player` - Which player is asked to pay
/// * `mana` - The mana cost that must be paid to prevent the effects
///
/// # Result
///
/// - If player pays: `crate::effect::OutcomeStatus::Declined` (effects prevented)
/// - If player doesn't pay: the result of executing inner effects
#[derive(Debug, Clone, PartialEq)]
pub struct UnlessPaysEffect {
    /// The effects to execute if the player does not pay.
    pub effects: Vec<Effect>,
    /// Which player is asked to pay.
    pub player: PlayerFilter,
    /// Total cost required to prevent the effects.
    pub cost: crate::cost::TotalCost,
    /// Whether the Oracle clause placed the payment before the consequence.
    pub leading_surface: bool,
    /// Whether the cost is payable before a surrounding delayed step.
    pub before_delayed_step: bool,
}

impl UnlessPaysEffect {
    /// Create a new "unless pays" effect.
    pub fn new(effects: Vec<Effect>, player: PlayerFilter, mana: Vec<ManaSymbol>) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects, player, mana, None, None, None, None,
        )
    }

    /// Create a new "unless pays" effect with a composable total cost.
    pub fn new_total_cost(
        effects: Vec<Effect>,
        player: PlayerFilter,
        cost: crate::cost::TotalCost,
    ) -> Self {
        Self {
            effects,
            player,
            cost,
            leading_surface: false,
            before_delayed_step: false,
        }
    }

    pub fn with_leading_surface(mut self, leading_surface: bool) -> Self {
        self.leading_surface = leading_surface;
        self
    }

    pub fn before_delayed_step(mut self, before_delayed_step: bool) -> Self {
        self.before_delayed_step = before_delayed_step;
        self
    }

    /// Create a new "unless pays" effect with optional life payment.
    pub fn new_with_life(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects, player, mana, life, None, None, None,
        )
    }

    /// Create a new "unless pays" effect with optional life and dynamic generic payment.
    pub fn new_with_life_and_additional(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects,
            player,
            mana,
            life,
            additional_generic,
            None,
            None,
        )
    }

    /// Create a new "unless pays" effect with optional life, dynamic generic payment,
    /// and dynamic mana multiplier.
    pub fn new_with_life_and_additional_and_multiplier(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
        mana_multiplier: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects,
            player,
            mana,
            life,
            additional_generic,
            mana_multiplier,
            None,
        )
    }

    /// Create a new "unless pays" effect with optional life, additional generic mana,
    /// mana multiplier, and a bound X value.
    pub fn new_with_life_and_additional_and_multiplier_and_x(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
        mana_multiplier: Option<Value>,
        x_value: Option<Value>,
    ) -> Self {
        Self::new_total_cost(
            effects,
            player,
            build_unless_payment_total_cost(
                mana,
                life,
                additional_generic,
                mana_multiplier,
                x_value,
            ),
        )
    }
}

fn build_unless_payment_total_cost(
    mana: Vec<ManaSymbol>,
    life: Option<Value>,
    additional_generic: Option<Value>,
    mana_multiplier: Option<Value>,
    x_value: Option<Value>,
) -> crate::cost::TotalCost {
    let mut components = Vec::new();
    let mana_cost = ManaCost::from_symbols(mana);
    if !mana_cost.is_empty()
        || additional_generic.is_some()
        || mana_multiplier.is_some()
        || x_value.is_some()
    {
        if additional_generic.is_some() || mana_multiplier.is_some() || x_value.is_some() {
            components.push(Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                mana_cost,
                x_value,
                additional_generic,
                mana_multiplier,
                ironsmith_core::DynamicManaDisplayHint::Default,
            )));
        } else {
            components.push(Cost::mana(mana_cost));
        }
    }
    if let Some(life) = life {
        let effect = Effect::new(crate::effects::PayLifeEffect::new(
            life,
            crate::target::ChooseSpec::Player(PlayerFilter::You),
        ));
        components.push(Cost::try_effect(effect).unwrap_or_else(|detail| {
            panic!("unless-pays life cost is not cost-executable: {detail}")
        }));
    }
    crate::cost::TotalCost::from_costs(components)
}

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

fn choose_payable_cost_for_simultaneous_action(
    game: &GameState,
    payer: PlayerId,
    source: crate::ids::ObjectId,
    cost: &crate::cost::TotalCost,
    ctx: &mut ExecutionContext,
) -> Result<Option<crate::cost::TotalCost>, ExecutionError> {
    crate::costs::select_payable_total_cost(
        game,
        payer,
        source,
        cost,
        crate::costs::PaymentReason::Effect,
        ctx,
    )
}

/// Number of leading instructions that only declare (and tag) a target.
fn leading_target_declaration_count(effects: &[Effect]) -> usize {
    effects
        .iter()
        .take_while(|effect| {
            let inner = effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map_or(*effect, |tagged| tagged.effect.as_ref());
            inner
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
        })
        .count()
}

#[derive(Debug, Clone, Copy)]
enum PendingUnlessInstruction {
    Declaration,
    Payment,
    Consequence,
}

struct UnlessPaysProgram {
    clause: UnlessPaysEffect,
    prepared: bool,
    declared: usize,
    next_declaration: usize,
    declarations: Vec<crate::effects::CompletedEffectOutputs>,
    paying_players: Option<Vec<PlayerId>>,
    next_payer: usize,
    payment: Option<crate::effects::CompletedEffectOutputs>,
    consequence: Option<Box<dyn crate::effects::ActionProgramCursor>>,
    result: Option<crate::effects::ProgramCompletion>,
    pending: Option<PendingUnlessInstruction>,
    preparations: Vec<crate::effects::ProgramPreparation>,
    ends_unit: bool,
}
impl std::fmt::Debug for UnlessPaysProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnlessPaysProgram")
            .field("clause", &self.clause)
            .field("next_declaration", &self.next_declaration)
            .field("next_payer", &self.next_payer)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}
impl UnlessPaysProgram {
    fn selected(
        clause: &UnlessPaysEffect,
        prepared: bool,
    ) -> Box<dyn crate::effects::ActionProgramCursor> {
        Box::new(Self {
            clause: clause.clone(),
            prepared,
            declared: leading_target_declaration_count(&clause.effects),
            next_declaration: 0,
            declarations: Vec::new(),
            paying_players: None,
            next_payer: 0,
            payment: None,
            consequence: None,
            result: None,
            pending: None,
            preparations: Vec::new(),
            ends_unit: false,
        })
    }

    fn accept_instruction(
        &mut self,
        outputs: crate::effects::CompletedEffectOutputs,
        accept_consequence: impl FnOnce(
            &mut dyn crate::effects::ActionProgramCursor,
            crate::effects::CompletedEffectOutputs,
        ) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        match self.pending.take().ok_or_else(|| {
            ExecutionError::InternalError(
                "unless-payment acknowledged without a selected instruction".into(),
            )
        })? {
            PendingUnlessInstruction::Declaration => self.declarations.push(outputs),
            PendingUnlessInstruction::Payment => {
                // This status is the actual total-cost owner's acknowledgement,
                // independent of the component packets' physical outcomes.
                if outputs.outcome.status == crate::effect::OutcomeStatus::Succeeded {
                    self.payment = Some(outputs);
                }
            }
            PendingUnlessInstruction::Consequence => accept_consequence(
                self.consequence
                    .as_mut()
                    .expect("active consequence")
                    .as_mut(),
                outputs,
            )?,
        }
        Ok(())
    }

    fn complete_selected(
        self: Box<Self>,
        prefix: Option<crate::effects::ProgramCompletion>,
    ) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        let mut completed = if let Some(payment) = self.payment {
            let outcome = EffectOutcome::aggregate_with_primary_result(
                EffectOutcome::declined(),
                [payment.outcome.clone()],
            );
            crate::effects::ProgramCompletion::new(payment.project_aggregate(outcome))
        } else if let Some(result) = self.result {
            result
        } else {
            prefix.ok_or_else(|| {
                ExecutionError::InternalError(
                    "unless-payment lost its completed consequence".into(),
                )
            })?
        };
        completed.outputs.retain_batch_children(self.declarations);
        Ok(completed)
    }
}
impl crate::effects::ActionProgramCursor for UnlessPaysProgram {
    fn take_preparations(&mut self) -> Vec<crate::effects::ProgramPreparation> {
        std::mem::take(&mut self.preparations)
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<crate::effects::ProgramAction>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() || ctx.resolution_stopped() {
            return Ok(None);
        }
        if self.pending.is_some() {
            return Err(ExecutionError::InternalError(
                "unless-payment selected another instruction before acknowledgement".into(),
            ));
        }
        if self.payment.is_some() || self.result.is_some() {
            return Ok(None);
        }
        // Targets may determine both the payer and the cost. Resolve them at
        // their authored boundary, before even selecting a payment offer.
        if self.next_declaration < self.declared {
            let index = self.next_declaration;
            self.next_declaration += 1;
            self.pending = Some(PendingUnlessInstruction::Declaration);
            self.ends_unit = false;
            let mut action = crate::effects::ProgramAction::new(self.clause.effects[index].clone());
            action.identity = vec![2, index];
            return Ok(Some(action));
        }
        if self.paying_players.is_none() {
            self.paying_players = Some(match self.clause.player {
                PlayerFilter::Any => players_in_turn_order(game),
                PlayerFilter::Opponent => {
                    let filter_ctx = ctx.filter_context(game);
                    players_in_turn_order(game)
                        .into_iter()
                        .filter(|player| self.clause.player.matches_player(*player, &filter_ctx))
                        .collect()
                }
                _ => vec![resolve_player_filter(game, &self.clause.player, ctx)?],
            });
        }
        let paying_players = self
            .paying_players
            .as_ref()
            .expect("selected paying players");
        while let Some(&payer) = paying_players.get(self.next_payer) {
            let index = self.next_payer;
            self.next_payer += 1;
            let Some(wants_to_pay) = select_payment_offer(game, payer, &self.clause.cost, ctx)?
            else {
                return Ok(None);
            };
            if !wants_to_pay {
                continue;
            }
            let cost = if self.prepared {
                let selected = choose_payable_cost_for_simultaneous_action(
                    game,
                    payer,
                    ctx.source,
                    &self.clause.cost,
                    ctx,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                let Some(cost) = selected else {
                    continue;
                };
                cost
            } else {
                // Keep ordinary OneOf selection and sequential component input
                // selection inside the payment owner at the actual request.
                self.clause.cost.clone()
            };
            self.pending = Some(PendingUnlessInstruction::Payment);
            self.ends_unit = true;
            let mut action = crate::effects::ProgramAction::new(Effect::new(self.clause.clone()));
            action.identity = vec![1, index];
            action.native = Some(super::action_program::NativeProgramAction::TotalCost {
                cost,
                payer,
                reason: crate::costs::PaymentReason::Effect,
            });
            return Ok(Some(action));
        }
        if self.consequence.is_none() {
            let player = match paying_players.as_slice() {
                [player] if ctx.iteration.iterated_player.is_none() => Some(*player),
                _ => ctx.iteration.iterated_player,
            };
            self.consequence = Some(super::branch_program::selected_clause_cursor(
                &self.clause.effects[self.declared..],
                vec![0],
                crate::effects::ProgramActionScope {
                    iterated_player: Some(player),
                    ..Default::default()
                },
            ));
        }
        let consequence = self.consequence.as_mut().expect("selected consequence");
        let next = consequence.next_action(game, ctx)?;
        self.preparations.extend(consequence.take_preparations());
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if next.is_some() {
            self.pending = Some(PendingUnlessInstruction::Consequence);
            self.ends_unit = consequence.ends_action_unit();
            return Ok(next);
        }
        if !self.preparations.is_empty() {
            return Ok(None);
        }
        self.result = Some(
            self.consequence
                .take()
                .expect("completed consequence")
                .finish()?,
        );
        Ok(None)
    }
    fn accept_action(
        &mut self,
        outputs: crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.accept_instruction(outputs, |cursor, outputs| cursor.accept_action(outputs))
    }
    fn accept_action_with_context(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outputs: crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.accept_instruction(outputs, |cursor, outputs| {
            cursor.accept_action_with_context(game, ctx, outputs)
        })
    }
    fn ends_action_unit(&self) -> bool {
        self.ends_unit
    }
    fn finish(self: Box<Self>) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        if self.pending.is_some() {
            return Err(ExecutionError::InternalError(
                "unless-payment finished before instruction acknowledgement".into(),
            ));
        }
        self.complete_selected(None)
    }

    fn finish_stopped(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        // No acknowledgement is manufactured for an uncommitted request. The
        // active child owns its completed prefix and scope cancellation.
        self.pending = None;
        let prefix = if let Some(consequence) = self.consequence.take() {
            consequence.finish_stopped(game, ctx)?
        } else {
            crate::effects::ProgramCompletion::new(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            )
        };
        self.complete_selected(Some(prefix))
    }
}

#[derive(Debug)]
struct UnlessPaysProposal {
    prepared: Option<Box<dyn crate::effects::SimultaneousEffectProposal>>,
    effects: Vec<Effect>,
    payer: PlayerId,
    cost: Option<crate::cost::TotalCost>,
    iterated_player: Option<PlayerId>,
}

impl crate::effects::SimultaneousEffectProposal for UnlessPaysProposal {
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
        if let Some(inner) = self.prepared.take() {
            let paid = self.cost.is_some();
            let receipt = inner.commit_original_with_outputs(game, ctx)?;
            if !paid {
                return Ok(receipt);
            }
            return Ok(super::compose_original_commits_with_projection_outputs(
                vec![receipt],
                Box::new(|outcomes| {
                    EffectOutcome::aggregate_with_primary_result(
                        EffectOutcome::declined(),
                        outcomes,
                    )
                }),
            ));
        }
        self.execute_fallback_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::finished)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.prepared.is_some() {
            return super::complete_prepared_original(self, game, ctx);
        }
        self.execute_fallback_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

impl UnlessPaysProposal {
    fn execute_fallback_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let proposal = *self;
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                if let Some(cost) = &proposal.cost {
                    let payment =
                        ctx.with_temp_iterated_player(proposal.iterated_player, |ctx| {
                            acknowledged_payment(
                                pay_total_cost_with_choice_in_context_with_outputs(
                                    game,
                                    proposal.payer,
                                    ctx.source,
                                    cost,
                                    crate::costs::PaymentReason::Effect,
                                    ctx,
                                ),
                            )
                        })?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if let Some(outputs) = payment {
                        return Ok(paid_clause_outputs(Vec::new(), outputs));
                    }
                }
                execute_consequences(
                    game,
                    ctx,
                    &proposal.effects,
                    Vec::new(),
                    proposal.iterated_player,
                )
            },
        )
    }
}

impl EffectExecutor for UnlessPaysEffect {
    fn supports_prepared_action_program(&self) -> bool {
        crate::costs::total_cost_supports_prepared_program(&self.cost)
            && self
                .effects
                .iter()
                .all(super::action_program::action_program_child_is_prepared)
    }

    fn select_prepared_action_program(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        Ok(Some(UnlessPaysProgram::selected(self, true)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
        crate::ability::visit_total_cost_owned_effects(&self.cost, visitor);
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.player == PlayerFilter::IteratedPlayer
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if self.player != PlayerFilter::IteratedPlayer {
            return Err(ExecutionError::Impossible(
                "simultaneous unless-payment requires an iterated-player payer".to_string(),
            ));
        }
        let payer = resolve_player_filter(game, &self.player, ctx)?;
        let Some(wants_to_pay) = select_payment_offer(game, payer, &self.cost, ctx)? else {
            // This selected parent carries no original while the offer is
            // pending. The enclosing transaction owns suspension and replay.
            return Ok(Box::new(UnlessPaysProposal {
                prepared: None,
                effects: self.effects.clone(),
                payer,
                cost: None,
                iterated_player: ctx.iteration.iterated_player,
            }));
        };
        let cost = if wants_to_pay {
            choose_payable_cost_for_simultaneous_action(game, payer, ctx.source, &self.cost, ctx)?
        } else {
            None
        };

        if ctx.decision_maker.awaiting_choice() || ctx.resolution_stopped() {
            return Ok(Box::new(UnlessPaysProposal {
                prepared: None,
                effects: self.effects.clone(),
                payer,
                cost: None,
                iterated_player: ctx.iteration.iterated_player,
            }));
        }
        let iterated_player = ctx.iteration.iterated_player;
        let prepared = if let Some(cost) = &cost {
            crate::costs::prepare_total_cost(
                cost,
                game,
                ctx,
                payer,
                crate::costs::PaymentReason::Effect,
            )?
        } else {
            super::prepared_branch::prepare_action_branch(
                &self.effects,
                game,
                ctx,
                iterated_player,
                false,
                false,
            )?
        };
        Ok(Box::new(UnlessPaysProposal {
            prepared,
            effects: self.effects.clone(),
            payer,
            cost,
            iterated_player: ctx.iteration.iterated_player,
        }))
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
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                super::action_program::execute_action_program_with_outputs(
                    UnlessPaysProgram::selected(self, false),
                    game,
                    ctx,
                    crate::effects::EffectExecutionPurpose::Action,
                )
            },
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn dynamic_energy_sacrifice_unless(tag: crate::tag::TagKey) -> UnlessPaysEffect {
        let tagged = crate::target::ChooseSpec::Tagged(tag);
        let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(tagged.clone()));
        let pay_energy = Effect::new(crate::effects::PayEnergyEffect::new(
            Value::ManaValueOf(Box::new(tagged)),
            crate::target::ChooseSpec::Player(PlayerFilter::You),
        ));
        let cost = Cost::try_effect(pay_energy)
            .expect("paying dynamic energy is executable as an effect cost");
        UnlessPaysEffect::new_total_cost(
            vec![sacrifice],
            PlayerFilter::You,
            TotalCost::from_cost(cost),
        )
    }

    fn add_creature_with_mana_value(
        game: &mut GameState,
        controller: PlayerId,
        mana_value: u8,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Exchanged creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    struct AcceptForLastOpponent {
        accepting_player: PlayerId,
        prompted: Vec<PlayerId>,
    }

    impl DecisionMaker for AcceptForLastOpponent {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let player = game.controlling_player_for(ctx.player);
            self.prompted.push(player);
            player == self.accepting_player
        }
    }

    fn add_payment_replacement_permanent(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        ability: StaticAbility,
    ) {
        let source = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source, controller, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("static-ability source should exist")
            .abilities_mut()
            .push(Ability::static_ability(ability));
    }

    #[test]
    fn unless_pays_effect_can_use_krrik_life_for_black_under_yasharn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Yasharn Effect Helper",
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new(
            vec![Effect::lose_life(3)],
            PlayerFilter::You,
            vec![ManaSymbol::Black],
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_total_cost_life_payment_prevents_inner_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::from_costs(vec![Cost::life(2)]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays total cost effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_total_cost_executes_inner_effect_when_unaffordable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::from_costs(vec![Cost::life(30)]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays total cost effect should execute");

        assert_ne!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 15);
    }

    // UNRUN main/campaign integration regression: Stop preserves the actual
    // prefix packet while its aggregate and child view share one history event.
    #[test]
    fn stopped_unless_consequence_retains_actual_prefix_without_duplicate_history() {
        #[derive(Debug, Clone)]
        struct Stop;
        impl EffectExecutor for Stop {
            fn execute(
                &self,
                _game: &mut GameState,
                ctx: &mut ExecutionContext,
            ) -> Result<EffectOutcome, ExecutionError> {
                ctx.stop_resolution();
                Ok(EffectOutcome::count(7))
            }
        }

        for dispatched in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source = game.new_object_id();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let effect = UnlessPaysEffect::new_total_cost(
                vec![Effect::gain_life(2), Effect::new(Stop), Effect::gain_life(9)],
                PlayerFilter::You,
                TotalCost::from_cost(Cost::life(30)),
            );
            let outputs = if dispatched {
                crate::effects::execute_effect_with_outputs(
                    &mut game,
                    &Effect::new(effect),
                    &mut ctx,
                )
            } else {
                effect.execute_with_outputs(&mut game, &mut ctx)
            }
            .expect("successful Stop retains the completed consequence prefix");

            assert!(ctx.resolution_stopped());
            assert_eq!(game.player(alice).expect("alice exists").life, 22);
            assert_eq!(outputs.outcome.events_of_type::<crate::events::LifeGainEvent>().count(), 1);
            let event = outputs.outcome.events.iter()
                .find(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                .expect("the stopped aggregate retains its real life-gain event");
            let child = outputs.shared.iter()
                .find(|child| child.outputs.outcome.events.iter().any(|owned| owned.ptr_eq(event)))
                .expect("the original child packet retains the same event identity");
            assert!(matches!(&child.ownership, crate::effects::SharedOutcomeOwnership::Batch));
            assert_eq!(child.outputs.outcome.events_of_type::<crate::events::LifeGainEvent>().count(), 1);
            assert_eq!(game.turn_store.turn_history.event_kind_count(crate::events::EventKind::LifeGain), 1);
            assert_eq!(game.turn_store.turn_history.total_life_gained_for_players(&[alice]), 2);
        }
    }

    #[test]
    fn effect_backed_unless_cost_resolves_you_relative_to_the_payer() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let alice_creature = add_creature_with_mana_value(&mut game, alice, 1);
        let bob_creature = add_creature_with_mana_value(&mut game, bob, 1);
        let source = game.new_object_id();
        let counter_target = crate::target::ChooseSpec::Object(
            crate::filter::ObjectFilter::creature().controlled_by(PlayerFilter::You),
        )
        .with_count(crate::effect::ChoiceCount::exactly(1));
        let counter_cost = Cost::try_effect(Effect::new(
            crate::effects::PutCountersEffect::minus_one_counters(1, counter_target),
        ))
        .expect("putting a counter is executable as an effect-backed cost");
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::Specific(bob),
            TotalCost::from_cost(counter_cost),
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("the designated player should be able to pay the counter cost");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(
            game.counter_count(alice_creature, crate::object::CounterType::MinusOneMinusOne),
            0
        );
        assert_eq!(
            game.counter_count(bob_creature, crate::object::CounterType::MinusOneMinusOne),
            1
        );
    }

    #[test]
    fn any_opponent_unless_payment_checks_each_opponent_and_excludes_controller() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source = game.new_object_id();
        let mut dm = AcceptForLastOpponent {
            accepting_player: charlie,
            prompted: Vec::new(),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::Opponent,
            TotalCost::from_cost(Cost::life(3)),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("one of multiple opponents should be allowed to pay");
        drop(ctx);

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(dm.prompted, vec![bob, charlie]);
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 20);
        assert_eq!(game.player(charlie).expect("charlie").life, 17);
    }

    #[test]
    fn gained_energy_pays_tagged_creatures_mana_value_or_sacrifices_it() {
        for (mana_value, should_survive, expected_energy) in [(4, true, 0), (5, false, 4)] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let target = add_creature_with_mana_value(&mut game, alice, mana_value);
            let source = game.new_object_id();
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
            let snapshot = crate::ObjectSnapshot::from_object(
                game.object(target).expect("target exists"),
                &game,
            );
            let tag = crate::tag::TagKey::from("exchanged");
            ctx.tag_object(tag.clone(), snapshot);

            crate::effects::EnergyCountersEffect::you(4)
                .execute(&mut game, &mut ctx)
                .expect("energy gain should execute before the payment choice");
            dynamic_energy_sacrifice_unless(tag)
                .execute(&mut game, &mut ctx)
                .expect("dynamic energy unless-payment should execute");

            assert_eq!(game.battlefield.contains(&target), should_survive);
            assert_eq!(
                game.player(alice).expect("alice exists").energy_counters,
                expected_energy
            );
            assert_eq!(
                game.player(alice).expect("alice exists").graveyard.len(),
                usize::from(!should_survive)
            );
        }
    }

    #[test]
    fn unless_pays_one_of_pays_selected_branch_only() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::life(2)),
                TotalCost::from_cost(Cost::life(4)),
            ]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays one-of cost should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_one_of_pays_only_affordable_branch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::life(30)),
                TotalCost::from_cost(Cost::life(2)),
            ]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays one-of cost should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_dynamic_x_mana_resolves_in_effect_context() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::from_cost(Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                ManaCost::from_symbols(vec![ManaSymbol::X]),
                Some(Value::Fixed(3)),
                None,
                None,
                ironsmith_core::DynamicManaDisplayHint::Default,
            ))),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays dynamic mana should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }
}

#[cfg(test)]
mod resource_failure_tests {
    use super::*;
    #[test]
    fn payment_exhaustion_is_never_the_unpaid_consequence_in_either_owner() {
        for simultaneous in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let player = PlayerId::from_index(0);
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Unless resource fixture")
                .card_types(vec![crate::types::CardType::Artifact]).build();
            let source = game.create_object_from_card(&card, player, crate::zone::Zone::Battlefield);
            let hand = game.create_object_from_card(&card, player, crate::zone::Zone::Hand);
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
                source, player, crate::events::cards::matchers::WouldDiscardMatcher::you(),
                crate::replacement::ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::new(crate::effects::CreateTokenEffect::you(crate::cards::tokens::treasure_token_definition(), 2))]),
            ));
            game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
            game.take_pending_trigger_events(); let next = game.next_object_id_counter();
            let effect = UnlessPaysEffect::new_total_cost(vec![Effect::sacrifice_source()],
                if simultaneous { PlayerFilter::IteratedPlayer } else { PlayerFilter::You },
                crate::cost::TotalCost::from_cost(Cost::discard(1, None)));
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, player, &mut dm); ctx.iteration.iterated_player = Some(player);
            let result = if simultaneous {
                effect.prepare_simultaneous_player_action(&game, &mut ctx).unwrap().commit(&mut game, &mut ctx)
            } else { effect.execute(&mut game, &mut ctx) };
            assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. })));
            assert!(game.battlefield.contains(&source)); assert_eq!(game.object(hand).unwrap().zone, crate::zone::Zone::Hand);
            assert_eq!(game.player(player).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), next);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}
