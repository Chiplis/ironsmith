//! A collective payment owns both its accepted total and the program using it.
//!
//! Contributions are sequential, starting with the effect controller. The
//! nested program keeps its own ordinary APNAP/simultaneous player semantics.
use crate::effect::{Effect, EffectOutcome, ExecutionFact, Value};
use crate::effects::{
    EffectExecutor, ExecutionContext, ExecutionError, ForPlayersEffect, PayManaEffect,
    SequenceEffect,
};
use crate::game_state::GameState;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub struct CollectManaPaymentsEffect {
    pub effects: Vec<Effect>,
    /// Who may pay; absent means every in-game player (join forces).
    pub payers: Option<PlayerFilter>,
    /// Only mana of these colors may be paid ("any amount of {R}").
    pub x_colors: Option<crate::color::ColorSet>,
    /// Contribute in APNAP order (CR 101.4) instead of controller first.
    pub apnap_order: bool,
    /// Run the program once per payer with that payer's own payment as X.
    pub per_payer: bool,
}
impl CollectManaPaymentsEffect {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            payers: None,
            x_colors: None,
            apnap_order: false,
            per_payer: false,
        }
    }

    pub fn paid_by(mut self, payers: PlayerFilter) -> Self {
        self.payers = Some(payers);
        self
    }

    pub fn with_x_colors(mut self, colors: crate::color::ColorSet) -> Self {
        self.x_colors = Some(colors);
        self
    }

    pub fn in_apnap_order(mut self) -> Self {
        self.apnap_order = true;
        self
    }

    pub fn per_payer(mut self) -> Self {
        self.per_payer = true;
        self
    }

    fn payer_order(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Vec<crate::ids::PlayerId>, ExecutionError> {
        let filter = self.payers.clone().unwrap_or(PlayerFilter::Any);
        let order = if self.apnap_order {
            ForPlayersEffect::new(filter, Vec::new())
        } else {
            ForPlayersEffect::new_starting_with_controller(filter, Vec::new())
        };
        order.selected_players(game, ctx)
    }

    fn payment_cost(&self) -> ManaCost {
        let cost = ManaCost::from_symbols(vec![ManaSymbol::X]);
        match self.x_colors {
            // CR 107.3: X may be paid only with mana of these colors.
            Some(colors) => cost.with_spending_restriction(
                crate::mana::ManaSpendingRestriction::OnX {
                    colors,
                    maximum_per_color: None,
                },
            ),
            None => cost,
        }
    }
}

fn add_accepted_payment(total: u32, amount: u32) -> Result<u32, ExecutionError> {
    total
        .checked_add(amount)
        .filter(|sum| *sum <= i32::MAX as u32)
        .ok_or_else(|| {
            ExecutionError::UnresolvableValue(
                "collective mana total exceeds the value domain".into(),
            )
        })
}

impl EffectExecutor for CollectManaPaymentsEffect {
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
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                let players = self.payer_order(game, ctx)?;
                let saved_x = ctx.x_value;
                let saved_reason = ctx.mana.payment_reason;
                // Spell/activation X and their restricted-mana permissions do not
                // carry into voluntary resolution payments.
                ctx.mana.payment_reason = Some(crate::costs::PaymentReason::Effect);
                let result = (|| {
                    let mut total = 0u32;
                    let mut per_payer = Vec::with_capacity(players.len());
                    let mut receipts = Vec::with_capacity(players.len() + 1);
                    for player in players {
                        ctx.x_value = None;
                        // Value::X is signed at its consumers. Never wrap, saturate,
                        // or silently publish a partial total at that boundary.
                        let pooled = game
                            .player(player)
                            .map_or(0, |player| player.mana_pool.total_wide());
                        if pooled > i32::MAX as u64 {
                            return Err(ExecutionError::UnresolvableValue(
                                "collective contribution exceeds the value domain".into(),
                            ));
                        }
                        let potential = crate::derived_view::DerivedGameView::new(game)
                            .potential_mana(player)
                            .total_wide();
                        let maximum = i32::try_from(potential).map_err(|_| {
                            ExecutionError::UnresolvableValue(
                                "collective contribution exceeds the value domain".into(),
                            )
                        })?;
                        let payment = PayManaEffect::new(
                            self.payment_cost(),
                            ChooseSpec::SpecificPlayer(player),
                        )
                        .with_x_maximum(Value::Fixed(maximum));
                        let receipt = payment.execute_with_outputs(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        // A proposed number, failed payment, or cancellation is
                        // never evidence of mana actually paid. An accepted zero
                        // remains a successful payment receipt.
                        let mut paid = 0u32;
                        for fact in receipt.outcome.instruction_result().execution_facts() {
                            if let ExecutionFact::ManaPaid { x_value } = fact {
                                paid = add_accepted_payment(paid, *x_value)?;
                            }
                        }
                        total = add_accepted_payment(total, paid)?;
                        per_payer.push((player, paid));
                        receipts.push(receipt);
                    }
                    if self.per_payer {
                        // Every payment is made before any payer's program runs;
                        // each program reads only its own payer's amount.
                        for (player, paid) in per_payer {
                            ctx.x_value = Some(paid);
                            let receipt = ctx.with_temp_iterated_player(Some(player), |ctx| {
                                SequenceEffect::new(self.effects.clone())
                                    .execute_with_outputs(game, ctx)
                            })?;
                            receipts.push(receipt);
                            if ctx.decision_maker.awaiting_choice() {
                                break;
                            }
                        }
                    } else {
                        ctx.x_value = Some(total);
                        receipts.push(
                            SequenceEffect::new(self.effects.clone())
                                .execute_with_outputs(game, ctx)?,
                        );
                    }
                    // Retain every payment/event receipt; expose the final body as
                    // the instruction result, not the sum of its unrelated counts.
                    Ok(crate::effects::CompletedEffectOutputs::from_children(
                        receipts,
                        EffectOutcome::aggregate_terminal,
                    ))
                })();
                ctx.x_value = saved_x;
                ctx.mana.payment_reason = saved_reason;
                result
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }
    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn total_checks_both_unsigned_overflow_and_the_signed_value_boundary() {
        assert_eq!(add_accepted_payment(i32::MAX as u32, 0).unwrap(), i32::MAX as u32);
        assert!(matches!(add_accepted_payment(i32::MAX as u32, 1), Err(ExecutionError::UnresolvableValue(_))));
        assert!(matches!(add_accepted_payment(u32::MAX, 1), Err(ExecutionError::UnresolvableValue(_))));
        assert_eq!(add_accepted_payment(2, 3).unwrap(), 5);
    }
}
