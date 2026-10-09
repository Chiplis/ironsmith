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
/// Returns `Count(1)` for a fixed or externally defined payment. For a
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
        .with_spend_policy(game.mana_spend_policy_for_reason(player_id, Some(source), reason));
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
) -> Result<Option<Vec<crate::effects::CompletedEffectOutputs>>, ExecutionError> {
    crate::effects::composition::execute_optional_world_transaction(game, ctx, |game, ctx| {
        try_pay_interactively_inner(effect, game, ctx, player_id, x_value)
    })
}

fn try_pay_interactively_inner(
    effect: &PayManaEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    x_value: u32,
) -> Result<Option<Vec<crate::effects::CompletedEffectOutputs>>, ExecutionError> {
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
    let mut replans = 0;
    let mut payment_open = false;
    let mut outputs = Vec::new();
    loop {
        let planned = match crate::mana_payment::plan_first_mana_payment(game, &request) {
            Ok(plan) => Some(plan),
            Err(crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)) => {
                return Err(error);
            }
            Err(_) => None,
        };
        let Some(plan) = planned.or_else(|| {
            payment_open.then(|| crate::mana_payment::unfunded_mana_payment_plan(game, &request))
        }) else {
            return Ok(None);
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
        payment_open = true;
        let response = ctx.decision_maker.decide_mana_payment(game, &decision);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        match response {
            crate::mana_payment::ManaPaymentResponse::Activate {
                source,
                ability_index,
            } => {
                let activated = crate::mana_payment::activate_mana_during_payment_with_outputs(
                    game,
                    &request,
                    source,
                    ability_index,
                    ctx.decision_maker,
                )
                .map_err(|error| match error {
                    crate::special_actions::ActionError::ExecutionFailure { error, .. } => error,
                    other => {
                        ExecutionError::Impossible(format!("illegal mana activation: {other}"))
                    }
                })?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                if let Some(activated) = activated {
                    outputs.extend(activated);
                }
                request
                    .preferences
                    .required_sources
                    .retain(|id| *id != source);
                request
                    .preferences
                    .required_activations
                    .retain(|activation| activation.source != source);
            }
            crate::mana_payment::ManaPaymentResponse::Cancel => return Ok(None),
            crate::mana_payment::ManaPaymentResponse::Replan { mut preferences } => {
                replans += 1;
                if replans >= MAX_REPLANS {
                    return Ok(None);
                }
                preferences.normalize();
                request.preferences = preferences;
            }
            crate::mana_payment::ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if plan.payable && plan_id == plan.id && request_hash == plan.request_hash => {
                let execution = crate::effects::ExecutionContextCheckpoint::capture(ctx);
                return match crate::mana_payment::execute_mana_payment_plan_in_context_with_outputs(
                    game,
                    &request,
                    &plan,
                    &mut ctx.decision_maker,
                    Some(&execution),
                ) {
                    Ok(completed)
                        if completed.status == crate::mana_payment::ManaPaymentExecution::Paid =>
                    {
                        outputs.extend(completed.outputs);
                        Ok(Some(outputs))
                    }
                    Err(crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)) => {
                        Err(error)
                    }
                    _ => Ok(None),
                };
            }
            crate::mana_payment::ManaPaymentResponse::Confirm { .. } => return Ok(None),
        }
    }
}

fn maximum_affordable_bounded_x(
    effect: &PayManaEffect,
    game: &GameState,
    ctx: &ExecutionContext<'_>,
    player_id: PlayerId,
    semantic_maximum: u32,
) -> Result<Option<u32>, ExecutionError> {
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
        match crate::mana_payment::check_mana_payment(game, &request) {
            Ok(()) => Ok(true),
            Err(crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)) => {
                Err(error)
            }
            Err(_) => Ok(false),
        }
    };
    if !can_pay(0)? {
        return Ok(None);
    }

    // Paying a mana cost with a larger X cannot require less mana than paying
    // the same cost with a smaller X, so affordability is monotonic.
    let mut lower = 0;
    let mut upper = semantic_maximum;
    while lower < upper {
        let distance = upper - lower;
        let middle = lower + distance / 2 + distance % 2;
        if can_pay(middle)? {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    Ok(Some(lower))
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
                let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
                let chooses_x =
                    self.cost.has_x() && self.x_value.is_none()
                        && (self.independent_x_choice || ctx.x_value.is_none());
                let bounded_x = if self.x_maximum.is_some() || chooses_x {
                    let semantic_maximum = if let Some(maximum) = &self.x_maximum {
                        resolve_value(game, maximum, ctx)?.max(0) as u32
                    } else if self.cost.has_waterbend_obligation() {
                        let reason = payment_reason(ctx);
                        let adjusted = game.adjust_mana_cost_for_payment_reason(
                            player_id,
                            Some(ctx.source),
                            &self.cost,
                            reason,
                        );
                        let request =
                            planner_request(game, player_id, ctx.source, adjusted, 0, reason);
                        crate::mana_payment::maximum_waterbend_x(game, &request).map_err(
                            |failure| match failure {
                                crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(
                                    error,
                                ) => error,
                                _ => ExecutionError::IncompleteEvidence(
                                    "unable to establish Waterbend X bound".into(),
                                ),
                            },
                        )?
                    } else {
                        crate::derived_view::DerivedGameView::new(game)
                            .potential_mana(player_id)
                            .total()
                    };
                    let Some(affordable_maximum) =
                        maximum_affordable_bounded_x(self, game, ctx, player_id, semantic_maximum)?
                    else {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::impossible(),
                        ));
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
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
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
                        .unwrap_or(ctx.x_value.unwrap_or(0) as i32)
                        .max(0) as u32
                };

                let Some(children) = try_pay_interactively(self, game, ctx, player_id, x_value)?
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::impossible(),
                    ));
                };
                let primary = if let Some(chosen) = bounded_x {
                    EffectOutcome::count(chosen as i32)
                        .with_execution_fact(ExecutionFact::ChosenNumber(chosen))
                        .with_execution_fact(ExecutionFact::ManaPaid { x_value: chosen })
                } else {
                    EffectOutcome::count(1)
                };
                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    children,
                    |outcomes| {
                        let observations = EffectOutcome::aggregate(
                            std::iter::once(primary.clone()).chain(outcomes),
                        );
                        primary.with_authoritative_observations(observations)
                    },
                ))
            },
        )
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
    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        let player = resolve_player_from_spec(game, &self.player, ctx)
            .map_err(CostValidationError::ExecutionFailed)?;
        let adjusted =
            game.adjust_mana_cost_for_payment_reason(player, Some(ctx.source), &self.cost, reason);
        let x = if self.x_maximum.is_some() {
            0
        } else {
            self.x_value
                .as_ref()
                .map(|value| resolve_value(game, value, ctx))
                .transpose()
                .map_err(CostValidationError::ExecutionFailed)?
                .map(|value| value.max(0) as u32)
                .unwrap_or(ctx.x_value.unwrap_or(0))
        };
        let request = planner_request(game, player, ctx.source, adjusted, x, reason);
        crate::mana_payment::check_mana_payment(game, &request).map_err(|error| match error {
            crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error) => {
                CostValidationError::ExecutionFailed(error)
            }
            _ => CostValidationError::Other("not enough mana available to pay cost".into()),
        })
    }

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
            crate::costs::PaymentReason::Effect,
        )
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, controller, &mut decision_maker);
        self.can_execute_as_cost_with_context(game, &mut ctx, reason)
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
    fn cancelled_effect_payment_restores_earlier_manual_mana_activation() {
        struct ActivateThenCancel { source: ObjectId, prompts: usize }
        impl DecisionMaker for ActivateThenCancel {
            fn decide_mana_payment(&mut self, _: &GameState,
                _: &crate::decisions::context::ManaPaymentContext,
            ) -> crate::mana_payment::ManaPaymentResponse {
                self.prompts += 1;
                if self.prompts == 1 {
                    crate::mana_payment::ManaPaymentResponse::Activate { source: self.source, ability_index: 0 }
                } else { crate::mana_payment::ManaPaymentResponse::Cancel }
            }
        }
        for dispatched in [false, true] {
            let mut game = setup_game();
            let player = PlayerId::from_index(0);
            let land = CardBuilder::new(CardId::new(), "Manual source")
                .card_types(vec![CardType::Land]).build();
            let source = game.create_object_from_card(&land, player, Zone::Battlefield);
            game.object_mut(source).unwrap().abilities_mut().push(Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()), vec![ManaSymbol::Red],
            ));
            game.take_pending_trigger_events();
            let mut dm = ActivateThenCancel { source, prompts: 0 };
            let mut ctx = ExecutionContext::new(source, player, &mut dm);
            let effect = PayManaEffect::new(ManaCost::from_symbols(vec![ManaSymbol::Red]),
                ChooseSpec::Player(PlayerFilter::You));
            let result = if dispatched {
                crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            } else { effect.execute(&mut game, &mut ctx) }.unwrap();
            assert_eq!(result.status, crate::effect::OutcomeStatus::Impossible);
            assert_eq!(dm.prompts, 2);
            assert!(!game.is_tapped(source));
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            assert!(game.take_pending_trigger_events().is_empty());
        }
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

#[cfg(test)]
mod waterbend_x_contracts {
    use super::*;
    use crate::effects::EffectExecutor;
    struct ChooseMaximum { maximum: Option<u32> }
    impl crate::decision::DecisionMaker for ChooseMaximum {
        fn decide_number(&mut self, _game: &GameState, context: &crate::decisions::context::NumberContext) -> u32 {
            self.maximum = Some(context.max); context.max
        }
    }
    #[test]
    fn freely_chosen_waterbend_x_counts_tap_resources_and_keeps_multiple_x_symbols() {
        for symbols in [1, 2] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let payer = PlayerId::from_index(0);
            let mut resources = Vec::new();
            for index in 0..2 {
                let card = crate::card::CardBuilder::new(crate::CardId::new(), format!("Resource {index}"))
                    .card_types(vec![crate::CardType::Artifact]).build();
                resources.push(game.create_object_from_card(&card, payer, crate::Zone::Battlefield));
            }
            let cost = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X; symbols]).with_waterbend();
            let effect = PayManaEffect::new(cost, ChooseSpec::Player(PlayerFilter::You));
            let mut chooser = ChooseMaximum { maximum: None };
            let mut context = ExecutionContext::new(resources[0], payer, &mut chooser);
            let outcome = effect.execute(&mut game, &mut context).unwrap();
            assert!(outcome.execution_facts().contains(&ExecutionFact::ManaPaid { x_value: (2 / symbols) as u32 }));
            assert_eq!(chooser.maximum, Some((2 / symbols) as u32));
            assert!(resources.iter().all(|id| game.is_tapped(*id)));
            assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
        }
    }
}
