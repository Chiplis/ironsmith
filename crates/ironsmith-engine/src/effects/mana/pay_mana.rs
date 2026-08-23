//! Pay mana effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{XValueSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ChooseSpec, PlayerFilter};

/// Effect that asks a player to pay a mana cost.
///
/// Returns `Count(1)` for a fixed or externally defined payment. For a bounded
/// player-chosen X payment, returns `Count(X)` and records the chosen number.
pub type PayManaEffect = ironsmith_core::PayManaEffect;

fn payment_reason(ctx: &ExecutionContext<'_>) -> crate::costs::PaymentReason {
    ctx.mana
        .payment_reason
        .unwrap_or(crate::costs::PaymentReason::Effect)
}

fn planner_request(
    game: &GameState,
    player_id: PlayerId,
    source: ObjectId,
    cost: crate::mana::ManaCost,
    x_value: u32,
    reason: crate::costs::PaymentReason,
) -> crate::mana_payment::ManaPaymentRequest {
    let mut request = crate::mana_payment::ManaPaymentRequest::new(player_id, source, reason, cost)
        .with_x(x_value)
        .with_spend_policy(game.mana_spend_policy(player_id, Some(source)));
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(player_id, Some(source), reason);
    request
}

fn try_pay_interactively(
    effect: &PayManaEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    x_value: u32,
) -> Result<bool, ExecutionError> {
    const MAX_REPLANS: usize = 16;
    let payment_reason = payment_reason(ctx);
    let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
        player_id,
        Some(ctx.source),
        &effect.cost,
        payment_reason,
    );
    let mut request = planner_request(
        game,
        player_id,
        ctx.source,
        adjusted_cost,
        x_value,
        payment_reason,
    );
    for _ in 0..MAX_REPLANS {
        let Some(plan) = crate::mana_payment::plan_mana_payment(game, &request)
            .ok()
            .and_then(|plans| plans.into_iter().next())
        else {
            return Ok(false);
        };
        let subject = game
            .object(ctx.source)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| "effect".to_string());
        let decision = crate::decisions::context::ManaPaymentContext::new(
            player_id,
            ctx.source,
            subject,
            request.clone(),
            plan.clone(),
        );
        let response = ctx.decision_maker.decide_mana_payment(game, &decision);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        match response {
            crate::mana_payment::ManaPaymentResponse::Cancel => return Ok(false),
            crate::mana_payment::ManaPaymentResponse::Replan { mut preferences } => {
                preferences.normalize();
                request.preferences = preferences;
            }
            crate::mana_payment::ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if plan_id == plan.id && request_hash == plan.request_hash => {
                return Ok(matches!(
                    crate::mana_payment::execute_mana_payment_plan(
                        game,
                        &request,
                        &plan,
                        &mut ctx.decision_maker,
                    ),
                    Ok(crate::mana_payment::ManaPaymentExecution::Paid)
                ));
            }
            crate::mana_payment::ManaPaymentResponse::Confirm { .. } => return Ok(false),
        }
    }
    Ok(false)
}

fn maximum_affordable_bounded_x(
    effect: &PayManaEffect,
    game: &GameState,
    ctx: &ExecutionContext<'_>,
    player_id: PlayerId,
    semantic_maximum: u32,
) -> Option<u32> {
    let reason = payment_reason(ctx);
    let adjusted_cost =
        game.adjust_mana_cost_for_payment_reason(player_id, Some(ctx.source), &effect.cost, reason);
    let can_pay = |x_value| {
        let request = planner_request(
            game,
            player_id,
            ctx.source,
            adjusted_cost.clone(),
            x_value,
            reason,
        );
        crate::mana_payment::plan_mana_payment(game, &request).is_ok()
    };
    if !can_pay(0) {
        return None;
    }

    // Paying a mana cost with a larger X cannot require less mana than paying
    // the same cost with a smaller X, so affordability is monotonic.
    let mut lower = 0;
    let mut upper = semantic_maximum;
    while lower < upper {
        let distance = upper - lower;
        let middle = lower + distance / 2 + distance % 2;
        if can_pay(middle) {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    Some(lower)
}

impl EffectExecutor for PayManaEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let bounded_x = if let Some(maximum) = &self.x_maximum {
            let semantic_maximum = resolve_value(game, maximum, ctx)?.max(0) as u32;
            let Some(affordable_maximum) =
                maximum_affordable_bounded_x(self, game, ctx, player_id, semantic_maximum)
            else {
                return Ok(EffectOutcome::impossible());
            };
            let chosen = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                player_id,
                Some(ctx.source),
                XValueSpec::new(ctx.source, affordable_maximum),
                FallbackStrategy::Maximum,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            Some(chosen.min(affordable_maximum))
        } else {
            None
        };
        let x_value = if let Some(chosen) = bounded_x {
            chosen
        } else {
            self.x_value
                .as_ref()
                .map(|value| resolve_value(game, value, ctx))
                .transpose()?
                .unwrap_or(0)
                .max(0) as u32
        };

        if !try_pay_interactively(self, game, ctx, player_id, x_value)? {
            Ok(EffectOutcome::impossible())
        } else if let Some(chosen) = bounded_x {
            Ok(EffectOutcome::count(chosen as i32)
                .with_execution_fact(ExecutionFact::ChosenNumber(chosen))
                .with_execution_fact(ExecutionFact::ManaPaid { x_value: chosen }))
        } else {
            Ok(EffectOutcome::count(1))
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to pay mana"
    }
}

impl CostExecutableEffect for PayManaEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let player_id = match self.player.inner() {
            ChooseSpec::Player(PlayerFilter::You | PlayerFilter::EffectController) => controller,
            ChooseSpec::Player(PlayerFilter::Specific(player))
            | ChooseSpec::SpecificPlayer(player) => *player,
            ChooseSpec::SourceController => game
                .object(source)
                .map(|object| game.controller_of(object))
                .unwrap_or(controller),
            _ => controller,
        };
        let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
            player_id,
            Some(source),
            &self.cost,
            crate::costs::PaymentReason::Effect,
        );
        let x_value = if self.x_maximum.is_some() {
            // Zero is always inside a bounded-X range. The semantic maximum
            // may depend on trigger context that cost preflight does not have.
            0
        } else {
            let context = ExecutionContext::new_default(source, controller);
            self.x_value
                .as_ref()
                .map(|value| resolve_value(game, value, &context))
                .transpose()
                .map_err(|_| {
                    CostValidationError::Other("unable to resolve mana payment X value".to_string())
                })?
                .unwrap_or(0)
                .max(0) as u32
        };
        let request = planner_request(
            game,
            player_id,
            source,
            adjusted_cost,
            x_value,
            crate::costs::PaymentReason::Effect,
        );
        if crate::mana_payment::plan_mana_payment(game, &request).is_ok() {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough mana available to pay cost".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::static_abilities::StaticAbility;
    use crate::target::PlayerFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

    #[derive(Default)]
    struct ActivateThenPayDecisionMaker {
        mana_payment_prompts: usize,
    }

    impl DecisionMaker for ActivateThenPayDecisionMaker {
        fn decide_mana_payment(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::ManaPaymentContext,
        ) -> crate::mana_payment::ManaPaymentResponse {
            self.mana_payment_prompts += 1;
            crate::mana_payment::ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }
    }

    struct ChooseBoundedXDecisionMaker {
        choice: u32,
        offered_maximum: Option<u32>,
    }

    impl ChooseBoundedXDecisionMaker {
        fn new(choice: u32) -> Self {
            Self {
                choice,
                offered_maximum: None,
            }
        }
    }

    impl DecisionMaker for ChooseBoundedXDecisionMaker {
        fn decide_number(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            assert!(ctx.is_x_value);
            self.offered_maximum = Some(ctx.max);
            self.choice.min(ctx.max)
        }
    }

    #[test]
    fn pay_mana_effect_activates_mana_ability_then_pays() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let mountain = CardBuilder::new(CardId::new(), "Test Mountain")
            .card_types(vec![CardType::Land])
            .build();
        let mountain_id = game.create_object_from_card(&mountain, alice, Zone::Battlefield);
        game.object_mut(mountain_id)
            .expect("mountain should exist")
            .abilities_mut()
            .push(Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                vec![ManaSymbol::Red],
            ));

        let mut dm = ActivateThenPayDecisionMaker::default();
        let mut ctx =
            ExecutionContext::new_default(mountain_id, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(dm.mana_payment_prompts, 1);
        assert!(game.is_tapped(mountain_id));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0
        );
    }

    #[test]
    fn pay_mana_effect_is_impossible_without_mana_sources() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Impossible);
    }

    #[test]
    fn pay_mana_effect_resolves_typed_x_value_from_source_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::new(), "Counter Payment Source")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.add_counters(source, crate::object::CounterType::PlusOnePlusOne, 2);
        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .red = 2;

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_value(crate::effect::Value::CountersOnSource(
            crate::object::CounterType::PlusOnePlusOne,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("counter-defined X payment should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0,
            "the X payment should spend one generic mana per source counter"
        );
    }

    #[test]
    fn bounded_x_payment_uses_trigger_amount_and_preserves_chosen_x() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .red = 5;
        let life_gain = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );

        let mut dm = ChooseBoundedXDecisionMaker::new(2);
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_triggering_event(life_gain);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::EventValue(
            crate::effect::EventValueSpec::LifeAmount,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("bounded X payment should execute");

        assert_eq!(dm.offered_maximum, Some(3));
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ChosenNumber(2))
        );
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            3
        );
    }

    #[test]
    fn bounded_x_payment_only_offers_affordable_values() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .red = 2;
        let life_gain = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, 5),
            crate::provenance::ProvNodeId::default(),
        );

        let mut dm = ChooseBoundedXDecisionMaker::new(5);
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_triggering_event(life_gain);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::EventValue(
            crate::effect::EventValueSpec::LifeAmount,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("bounded X payment should execute");

        assert_eq!(dm.offered_maximum, Some(2));
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0
        );
    }

    #[test]
    fn bounded_x_payment_of_zero_still_counts_as_completed() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let life_gain = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );

        let mut dm = ChooseBoundedXDecisionMaker::new(0);
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_triggering_event(life_gain);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::EventValue(
            crate::effect::EventValueSpec::LifeAmount,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("zero is a legal bounded X payment");

        assert_eq!(dm.offered_maximum, Some(0));
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ManaPaid { x_value: 0 })
        );
        assert!(
            crate::effect::EffectPredicateRuntimeExt::evaluate_outcome(
                &crate::effect::EffectPredicate::Happened,
                &result,
            ),
            "a successful zero-mana payment must satisfy an if-you-do branch"
        );
    }

    #[test]
    fn pay_mana_effect_can_use_krrik_life_for_black() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }

    #[test]
    fn pay_mana_effect_still_can_use_krrik_under_yasharn() {
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
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }
}
