//! Fixed life-payment effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{
    CostExecutableEffect, CostValidationError, EffectExecutor, ExecutionContext, ExecutionError,
};
use crate::events::LifeLossEvent;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;

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
        let player = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        if !game.pay_life(player, amount) {
            return Ok(EffectOutcome::impossible());
        }

        let outcome = EffectOutcome::count(amount as i32);
        if amount == 0 {
            return Ok(outcome);
        }

        Ok(outcome.with_event(TriggerEvent::new_with_provenance(
            LifeLossEvent::from_effect(player, amount),
            ctx.provenance,
        )))
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

    fn cost_description(&self) -> Option<String> {
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(amount) = self.amount
        {
            return Some(format!("Pay {} life", amount.max(0)));
        }
        None
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
        let ctx = ExecutionContext::new_default(source, controller);
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
    fn fixed_life_payment_emits_life_loss_with_effect_provenance() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let expected_provenance = ctx.provenance;

        let outcome = PayLifeEffect::you(2)
            .execute(&mut game, &mut ctx)
            .expect("life payment should resolve");

        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].provenance(), expected_provenance);
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
