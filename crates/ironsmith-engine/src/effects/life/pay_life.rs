//! Fixed life-payment effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{
    CostExecutableEffect, CostValidationError, EffectExecutor, ExecutionContext, ExecutionError,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ChooseSpec, PlayerFilter};

pub type PayLifeEffect = ironsmith_core::PayLifeEffect;

impl EffectExecutor for PayLifeEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let player = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        Ok(game
            .pay_life_with_context(player, amount, ctx)?
            .unwrap_or_else(EffectOutcome::impossible))
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let checked = game
            .continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let player = resolve_player_from_spec(&checked, &self.player, ctx)?;
        let amount = resolve_value(&checked, &self.amount, ctx)?.max(0) as u32;
        Ok(Box::new(FixedLifePaymentProposal {
            player,
            amount,
            payable: checked.can_pay_life(player, amount),
            prepared: None,
        }))
    }

    fn pay_life_amount(&self) -> Option<u32> {
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(amount) = self.amount
        {
            return Some(amount.max(0) as u32);
        }
        None
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.player.is_target().then_some(&self.player)
    }

    fn target_description(&self) -> &'static str {
        "player to pay life"
    }

    fn references_cost_x(&self) -> bool {
        matches!(self.amount.unhinted(), Value::X)
    }

    fn max_cost_x(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        let ctx = ExecutionContext::new_default(source, controller);
        let payer = resolve_player_from_spec(game, &self.player, &ctx).ok()?;
        let available = game.player(payer)?.life.max(0) as u32;
        Some(
            if game.can_pay_life_with_reason(
                payer,
                available,
                crate::costs::PaymentReason::ActivateAbility,
            ) {
                available
            } else {
                0
            },
        )
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(amount) = self.amount
        {
            return Some(format!("Pay {} life", amount.max(0)));
        }
        None
    }
}

struct FixedLifePaymentProposal {
    player: PlayerId,
    amount: u32,
    payable: bool,
    prepared: Option<crate::game_state::PreparedLifePayment>,
}
impl std::fmt::Debug for FixedLifePaymentProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixedLifePaymentProposal")
            .field("player", &self.player)
            .field("amount", &self.amount)
            .finish_non_exhaustive()
    }
}
impl crate::effects::SimultaneousEffectProposal for FixedLifePaymentProposal {
    fn declared_life_payment(&self) -> Option<(crate::ids::PlayerId,u32)> { self.payable.then_some((self.player,self.amount)) }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.payable {
            self.prepared = game.prepare_life_payment(self.player, self.amount, ctx, true)?;
        }
        Ok(())
    }
    fn commit_original(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        if !self.payable {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                EffectOutcome::impossible(),
            ));
        }
        if self.prepared.is_none() {
            self.prepare_original(game, ctx)?;
        }
        let prepared = self.prepared.take().ok_or_else(|| {
            ExecutionError::UnresolvableValue("prepared life payment is unavailable".into())
        })?;
        game.commit_life_payment_original(prepared, ctx)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !self.payable {
            return Ok(EffectOutcome::impossible());
        }
        Ok(game
            .pay_life_with_context(self.player, self.amount, ctx)?
            .unwrap_or_else(EffectOutcome::impossible))
    }
}

impl CostExecutableEffect for PayLifeEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
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
        source: ObjectId,
        controller: PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller).with_x(0);
        let player = resolve_player_from_spec(game, &self.player, &ctx).map_err(|_| {
            CostValidationError::Other("unable to resolve player for life payment".to_string())
        })?;
        let amount = resolve_value(game, &self.amount, &ctx)
            .map_err(|_| {
                CostValidationError::Other("unable to resolve life-payment amount".to_string())
            })?
            .max(0) as u32;

        if game.can_pay_life_with_reason(player, amount, reason) {
            Ok(())
        } else {
            Err(CostValidationError::NotEnoughLife)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Effect, OutcomeStatus};
    use crate::effects::MayEffect;

    #[test]
    fn fixed_life_payment_cannot_reduce_payer_below_zero() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 1;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = PayLifeEffect::you(2)
            .execute(&mut game, &mut ctx)
            .expect("life payment should resolve without an engine error");

        assert_eq!(outcome.status, OutcomeStatus::Impossible);
        assert_eq!(game.player(alice).expect("alice exists").life, 1);
        assert!(outcome.events.is_empty());
    }

    #[test]
    fn fixed_life_payment_retains_distinct_loss_and_payment_receipts() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = PayLifeEffect::you(2)
            .execute(&mut game, &mut ctx)
            .expect("life payment should resolve");

        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(outcome.events.len(), 2);
        assert!(
            outcome.events[0]
                .downcast::<crate::events::LifeLossEvent>()
                .is_some()
        );
        assert!(
            outcome.events[1]
                .downcast::<crate::events::LifePaidEvent>()
                .is_some()
        );
        assert_ne!(
            outcome.events[0].provenance(),
            outcome.events[1].provenance()
        );
        assert!(outcome.events.iter().all(|event| event.triggers_captured()));
    }

    #[test]
    fn dynamic_half_life_payment_rounds_up_against_the_payers_current_total() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 19;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = PayLifeEffect::you(Value::HalfLifeTotalRoundedUp(PlayerFilter::You))
            .execute(&mut game, &mut ctx)
            .expect("dynamic life payment should resolve");

        assert_eq!(outcome.as_count(), Some(10));
        assert_eq!(game.player(alice).expect("alice exists").life, 9);
    }

    #[test]
    fn active_player_decides_and_pays_single_optional_life_payment() {
        #[derive(Default)]
        struct AcceptAndCapturePlayer {
            prompted_player: Option<PlayerId>,
        }

        impl crate::decision::DecisionMaker for AcceptAndCapturePlayer {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.prompted_player = Some(ctx.player);
                true
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;
        let source = game.new_object_id();
        let mut decision_maker = AcceptAndCapturePlayer::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        MayEffect::new_for_player(
            vec![Effect::pay_life_player(2, PlayerFilter::Active)],
            PlayerFilter::Active,
        )
        .execute(&mut game, &mut ctx)
        .expect("active player should be able to accept the payment");
        drop(ctx);

        assert_eq!(decision_maker.prompted_player, Some(bob));
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(game.player(bob).expect("bob exists").life, 18);
    }

    #[test]
    fn impossible_single_optional_life_payment_declines_without_prompting() {
        struct PanicOnPrompt;

        impl crate::decision::DecisionMaker for PanicOnPrompt {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                panic!("an impossible life payment must not be offered")
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;
        game.player_mut(bob).expect("bob exists").life = 1;
        let source = game.new_object_id();
        let mut decision_maker = PanicOnPrompt;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let outcome = MayEffect::new_for_player(
            vec![Effect::pay_life_player(2, PlayerFilter::Active)],
            PlayerFilter::Active,
        )
        .execute(&mut game, &mut ctx)
        .expect("impossible optional payment should cleanly decline");

        assert_eq!(outcome.status, OutcomeStatus::Declined);
        assert_eq!(game.player(bob).expect("bob exists").life, 1);
    }
}
