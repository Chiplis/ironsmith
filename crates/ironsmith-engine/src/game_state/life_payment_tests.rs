//! Source-authored checked payment and receipt-boundary regressions; unrun.
use super::*;
use crate::ability::Ability;
use crate::cards::CardDefinitionBuilder;
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::{Effect, Value};
use crate::effects::{
    EffectExecutor, ExecutionContext, ExecutionError, ForPlayersEffect, PayLifeEffect,
};
use crate::events::{EventKind, LifeLossEvent, LifePaidEvent};
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::PlayerFilter;
use crate::triggers::{Trigger, TriggerContext, TriggerEvent, TriggerMatcher, TriggerQueue};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixture() -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardDefinitionBuilder::new(crate::CardId::new(), "Payment observer")
        .card_types(vec![crate::CardType::Enchantment])
        .with_ability(Ability::triggered(
            Trigger::player_pays_life(PlayerFilter::You),
            vec![Effect::gain_life(Value::EventValue(
                crate::effect::EventValueSpec::Amount,
            ))],
        ))
        .build();
    let source = game.create_object_from_definition(&card, A, Zone::Battlefield);
    (game, source)
}
fn seed_wide_history(game: &mut GameState) {
    let id = game
        .provenance_graph_mut()
        .alloc_root_event(EventKind::LifeLoss);
    game.record_turn_history_event(&TriggerEvent::new_with_provenance(
        LifeLossEvent::from_effect(B, i32::MAX as u32),
        id,
    ));
}
#[test]
fn payment_and_loss_are_distinct_originals_and_republication_is_idempotent() {
    let (mut game, _) = fixture();
    assert!(game.pay_life(A, 3).unwrap());
    assert_eq!(game.player(A).unwrap().life, 17);
    assert_eq!(game.effect_store.pending_trigger_entries.len(), 1);
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifeLoss),
        1
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        1
    );
    let notices = game.take_pending_trigger_events();
    assert_eq!(
        notices
            .iter()
            .filter(|event| event.downcast::<LifePaidEvent>().is_some())
            .count(),
        1
    );
    assert!(
        notices
            .iter()
            .filter(|event| event.downcast::<LifePaidEvent>().is_some())
            .all(|event| !event.inner().is_replacement_proposal())
    );
    let mut queue = TriggerQueue::new();
    crate::game_loop::queue_triggers_from_reported_events(
        &mut game,
        &mut queue,
        notices.clone(),
        true,
    );
    for event in notices {
        game.stage_turn_history_event(&event);
    }
    assert!(queue.is_empty());
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifeLoss),
        1
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        1
    );
    let mut restored = game.clone();
    assert_eq!(
        restored.take_pending_trigger_entries().len(),
        1,
        "native checkpoint retains already matched observer"
    );
}
#[test]
fn payment_observer_survives_later_cost_departure_and_prospective_costs_do_not_publish() {
    let (mut game, source) = fixture();
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::life(2),
        crate::costs::Cost::sacrifice_self(),
    ]);
    crate::cost::can_pay_cost_with_reason(
        &game,
        source,
        A,
        &cost,
        crate::costs::PaymentReason::ActivateAbility,
    )
    .unwrap();
    assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.effect_store.pending_trigger_entries.is_empty());
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        A,
        source,
        &cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(game.object(source).is_none());
    let entries = game.take_pending_trigger_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].source, source,
        "payment-time observer was not removed by the following sacrifice cost"
    );
}
#[derive(Debug, Clone, PartialEq)]
struct CompletePaymentWorld;
impl TriggerMatcher for CompletePaymentWorld {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event.downcast::<LifePaidEvent>().is_some()
            && [A, B]
                .into_iter()
                .all(|player| ctx.game.player(player).unwrap().life == 18)
            && ctx
                .game
                .turn_store
                .turn_history
                .event_kind_count(EventKind::LifeLoss)
                == 2
    }
    fn display(&self) -> String {
        "complete simultaneous payment world".into()
    }
}
#[test]
fn each_player_payment_publishes_after_all_originals_and_before_following_actions() {
    for variable in [false, true] {
        let (mut game, source) = fixture();
        game.object_mut(source).unwrap().abilities_mut().clear();
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::new(CompletePaymentWorld),
                vec![Effect::gain_life(1)],
            ));
        let payment = if variable {
            Effect::new(crate::effects::PayAnyLifeEffect::new(
                crate::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                0,
            ))
        } else {
            Effect::new(PayLifeEffect::with_filter(2, PlayerFilter::IteratedPlayer))
        };
        struct Two;
        impl crate::decision::DecisionMaker for Two {
            fn decide_number(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::NumberContext,
            ) -> u32 {
                2
            }
        }
        let mut dm = Two;
        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![
                payment,
                Effect::gain_life_player(
                    1,
                    crate::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                ),
            ],
        )
        .execute(&mut game, &mut ExecutionContext::new(source, A, &mut dm))
        .unwrap();
        assert_eq!(game.player(A).unwrap().life, 19);
        assert_eq!(game.player(B).unwrap().life, 19);
        assert_eq!(
            game.take_pending_trigger_entries().len(),
            2,
            "both observers see the complete 18/18 original payment frame, before the following gain"
        );
    }
}
#[test]
fn wide_life_history_preserves_direct_and_bulk_mana_payments() {
    let (mut game, source) = fixture();
    seed_wide_history(&mut game);
    assert!(game.pay_life(A, 2).unwrap());
    assert_eq!(game.player(A).unwrap().life, 18);
    assert_eq!(game.effect_store.pending_trigger_entries.len(), 1);
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Blue, ManaSymbol::Life(2)]);
    assert!(game.try_pay_mana_cost(A, Some(source), &cost, 0).unwrap());
    assert_eq!(game.player(A).unwrap().mana_pool.blue, 0);
    assert_eq!(game.player(A).unwrap().life, 16);
    assert_eq!(game.effect_store.pending_trigger_entries.len(), 2);
    assert_eq!(
        game.turn_store
            .turn_history
            .total_life_lost_for_players(&[A, B]),
        i32::MAX as u32 + 4
    );
}
#[test]
fn speculative_exact_payment_accepts_wide_history_without_mutating_the_original() {
    let (mut game, source) = fixture();
    seed_wide_history(&mut game);
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Life(2)]);
    assert!(
        game.mana_cost_with_payable_continuation(
            A,
            Some(source),
            &cost,
            0,
            crate::costs::PaymentReason::CastSpell,
            &crate::player::ManaSpendPolicy::default(),
            true,
            false,
            true,
            |_, _| true,
        )
        .unwrap()
        .is_some()
    );
    let request = crate::mana_payment::ManaPaymentRequest::new(
        A,
        source,
        crate::costs::PaymentReason::CastSpell,
        cost,
    );
    assert!(crate::mana_payment::plan_mana_payment(&game, &request).is_ok());
    assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.effect_store.pending_trigger_entries.is_empty());
}
#[test]
fn rejected_payment_and_failed_whole_upkeep_leave_no_success_receipts() {
    let (mut game, source) = fixture();
    assert!(!game.pay_life(A, 21).unwrap());
    game.object_mut(source)
        .unwrap()
        .counters
        .insert(crate::CounterType::Age, 2);
    let effect = crate::effects::CumulativeUpkeepEffect::new(
        PlayerFilter::You,
        vec![Effect::pay_life(11)],
        vec![],
    );
    struct Accept;
    impl crate::decision::DecisionMaker for Accept {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }
    effect
        .execute(
            &mut game,
            &mut ExecutionContext::new(source, A, &mut Accept),
        )
        .unwrap();
    assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.effect_store.pending_trigger_entries.is_empty());
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        0
    );
}

#[derive(Debug, Clone, PartialEq)]
struct PaidAtEighteen;
impl TriggerMatcher for PaidAtEighteen {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event
            .downcast::<LifePaidEvent>()
            .is_some_and(|paid| paid.player == A && paid.amount == 2)
            && ctx.game.player(A).unwrap().life == 18
    }
    fn display(&self) -> String {
        "payment observed at eighteen".into()
    }
}
fn life_replacement(
    game: &mut GameState,
    source: ObjectId,
    action: crate::replacement::ReplacementAction,
) {
    game.effect_store.replacement_effects.add_one_shot_effect(
        crate::replacement::ReplacementEffect::with_matcher(
            source,
            A,
            crate::events::life::matchers::WouldLoseLifeMatcher::you(),
            action,
        ),
    );
}
#[test]
fn modified_payment_keeps_nominal_paid_amount_and_processed_loss_receipt() {
    for replace in [false, true] {
        let (mut game, source) = fixture();
        let action = if replace {
            crate::replacement::ReplacementAction::Instead(vec![Effect::gain_life(3)])
        } else {
            crate::replacement::ReplacementAction::Double
        };
        life_replacement(&mut game, source, action);
        let mut ctx = ExecutionContext::new_default(source, A);
        let outcome = PayLifeEffect::you(2).execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            outcome.as_count(),
            Some(2),
            "CR118.11: fulfilled nominal payment is still two"
        );
        assert_eq!(game.player(A).unwrap().life, if replace { 23 } else { 16 });
        assert_eq!(
            outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<LifeLossEvent>())
                .map(|loss| loss.amount)
                .sum::<u32>(),
            if replace { 0 } else { 4 }
        );
        assert_eq!(
            outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<LifePaidEvent>())
                .map(|paid| paid.amount)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(game.take_pending_trigger_entries().len(), 1);
    }
}
#[test]
fn life_payment_observers_precede_replacement_additions_and_whole_attempt_rolls_back() {
    for fail in [false, true] {
        let (mut game, source) = fixture();
        game.object_mut(source).unwrap().abilities_mut().clear();
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::new(PaidAtEighteen),
                vec![Effect::gain_life(1)],
            ));
        let mut additions = vec![Effect::gain_life(5)];
        if fail {
            additions.push(Effect::lose_life(Value::X));
        }
        life_replacement(
            &mut game,
            source,
            crate::replacement::ReplacementAction::Additionally(additions),
        );
        let result =
            PayLifeEffect::you(2).execute(&mut game, &mut ExecutionContext::new_default(source, A));
        assert_eq!(result.is_err(), fail);
        assert_eq!(game.player(A).unwrap().life, if fail { 20 } else { 23 });
        assert_eq!(
            game.take_pending_trigger_entries().len(),
            usize::from(!fail)
        );
        assert_eq!(
            game.turn_store
                .turn_history
                .event_kind_count(EventKind::LifePaid),
            u32::from(!fail)
        );
    }
}
#[derive(Default)]
struct PendingLifeChoice {
    pending: bool,
    numbers: usize,
    pause: bool,
}
impl crate::decision::DecisionMaker for PendingLifeChoice {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.pending = self.pause;
        true
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &crate::decisions::context::NumberContext,
    ) -> u32 {
        let _ = c;
        self.numbers += 1;
        self.pending = self.pause;
        0
    }
}
#[test]
fn chosen_zero_is_acknowledged_under_life_prohibition_but_pending_zero_is_uncommitted() {
    for pause in [false, true] {
        for simultaneous in [false, true] {
            let (mut game, source) = fixture();
            game.object_mut(source)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(
                    crate::static_abilities::StaticAbility::your_life_total_cant_change(),
                ));
            let mut dm = PendingLifeChoice {
                pause,
                ..Default::default()
            };
            let effect = crate::effects::PayAnyLifeEffect::new(
                crate::ChooseSpec::Player(PlayerFilter::You),
                0,
            );
            if simultaneous {
                ForPlayersEffect::new(PlayerFilter::You, vec![Effect::new(effect)])
                    .execute(&mut game, &mut ExecutionContext::new(source, A, &mut dm))
                    .unwrap();
            } else {
                effect
                    .execute(&mut game, &mut ExecutionContext::new(source, A, &mut dm))
                    .unwrap();
            }
            assert_eq!(dm.numbers, 1);
            assert_eq!(dm.pending, pause);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(
                game.turn_store
                    .turn_history
                    .event_kind_count(EventKind::LifePaid),
                u32::from(!pause)
            );
            assert_eq!(
                game.turn_store
                    .turn_history
                    .event_kind_count(EventKind::LifeLoss),
                0
            );
            assert_eq!(
                game.take_pending_trigger_entries().len(),
                usize::from(!pause)
            );
        }
    }
}
#[test]
fn native_life_cost_does_not_narrow_oversized_amount_to_acknowledged_zero() {
    let (mut game, source) = fixture();
    let amount = i32::MAX as u32 + 1;
    let cost = crate::costs::Cost::life(amount);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::costs::CostContext::new(source, A, &mut dm);
    assert_eq!(cost.life_amount(), Some(amount));
    assert!(cost.can_pay(&game, &ctx).is_err());
    assert!(cost.pay(&mut game, &mut ctx).is_err());
    assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.take_pending_trigger_entries().is_empty());
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        0
    );
}
#[test]
fn bulk_mana_payment_preserves_replacement_choice_suspension_and_receipt_rollback() {
    let (mut game, source) = fixture();
    life_replacement(
        &mut game,
        source,
        crate::replacement::ReplacementAction::Additionally(vec![Effect::new(
            crate::effects::MayEffect::new(vec![Effect::gain_life(1)]),
        )]),
    );
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Blue, ManaSymbol::Life(2)]);
    let mut dm = PendingLifeChoice {
        pause: true,
        ..Default::default()
    };
    let paid = game
        .try_pay_mana_cost_with_reason_and_dm(
            A,
            Some(source),
            &cost,
            0,
            crate::costs::PaymentReason::CastSpell,
            &mut dm,
        )
        .unwrap();
    assert!(!paid);
    assert!(dm.pending);
    assert_eq!(game.player(A).unwrap().life, 20);
    assert_eq!(game.player(A).unwrap().mana_pool.blue, 1);
    assert!(game.take_pending_trigger_entries().is_empty());
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        0
    );
    dm.pending = false;
    dm.pause = false;
    assert!(
        game.try_pay_mana_cost_with_reason_and_dm(
            A,
            Some(source),
            &cost,
            0,
            crate::costs::PaymentReason::CastSpell,
            &mut dm
        )
        .unwrap()
    );
    assert_eq!(game.player(A).unwrap().life, 19);
    assert_eq!(game.player(A).unwrap().mana_pool.blue, 0);
    assert_eq!(game.take_pending_trigger_entries().len(), 1);
}

#[test]
fn nonmana_cost_scope_captures_payment_before_added_program_sacrifices_observer() {
    let (mut game, source) = fixture();
    life_replacement(
        &mut game,
        source,
        crate::replacement::ReplacementAction::Additionally(vec![Effect::sacrifice_source()]),
    );
    let total = crate::cost::TotalCost::from_cost(crate::costs::Cost::life(2));
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        A,
        source,
        &total,
        crate::costs::PaymentReason::ActivateAbility,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(game.player(A).unwrap().life, 18);
    assert!(game.object(source).is_none());
    let captured = game.take_pending_trigger_entries();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].source, source);
}

#[derive(Debug, Clone, PartialEq)]
struct GainAfterOtherPayerOriginal;
impl TriggerMatcher for GainAfterOtherPayerOriginal {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event
            .downcast::<crate::events::LifeGainEvent>()
            .is_some_and(|gain| gain.player == A)
            && ctx.game.player(A).unwrap().life == 23
            && ctx.game.player(B).unwrap().life == 18
    }
    fn display(&self) -> String {
        "gain in completed two-payer frame".into()
    }
}
#[test]
fn simultaneous_instead_life_action_is_prepared_then_observed_after_every_payer_original() {
    let (mut game, source) = fixture();
    game.object_mut(source).unwrap().abilities_mut().clear();
    game.object_mut(source)
        .unwrap()
        .abilities_mut()
        .push(Ability::triggered(
            Trigger::new(GainAfterOtherPayerOriginal),
            vec![Effect::gain_life(1)],
        ));
    life_replacement(
        &mut game,
        source,
        crate::replacement::ReplacementAction::Instead(vec![Effect::gain_life(3)]),
    );
    assert!(game.pay_life_simultaneously(&[(A, 2), (B, 2)]).unwrap());
    assert_eq!(game.player(A).unwrap().life, 23);
    assert_eq!(game.player(B).unwrap().life, 18);
    assert_eq!(game.take_pending_trigger_entries().len(), 1);
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        2
    );
}
#[test]
fn unsupported_compound_simultaneous_instead_fails_before_originals() {
    let (mut game, source) = fixture();
    life_replacement(
        &mut game,
        source,
        crate::replacement::ReplacementAction::Instead(vec![
            Effect::gain_life(1),
            Effect::gain_life(2),
        ]),
    );
    assert!(matches!(
        game.pay_life_simultaneously(&[(A, 2), (B, 2)]),
        Err(ExecutionError::UnresolvableValue(_))
    ));
    assert_eq!(game.player(A).unwrap().life, 20);
    assert_eq!(game.player(B).unwrap().life, 20);
    assert_eq!(
        game.turn_store
            .turn_history
            .event_kind_count(EventKind::LifePaid),
        0
    );
    assert!(game.take_pending_trigger_entries().is_empty());
}
#[test]
fn direct_payment_and_multiple_payer_completions_share_one_token_resource_budget() {
    let token = CardDefinitionBuilder::new(crate::CardId::new(), "Payment resource token")
        .token()
        .card_types(vec![crate::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .build();
    for many_payers in [false, true] {
        let (mut game, source) = fixture();
        game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
            max_created_tokens: 1,
            ..Default::default()
        });
        if many_payers {
            for player in [A, B] {
                game.effect_store.replacement_effects.add_one_shot_effect(
                    crate::replacement::ReplacementEffect::with_matcher(
                        source,
                        A,
                        crate::events::life::matchers::WouldLoseLifeMatcher::new(
                            PlayerFilter::Specific(player),
                        ),
                        crate::replacement::ReplacementAction::Additionally(vec![
                            Effect::create_tokens(token.clone(), 1),
                        ]),
                    ),
                );
            }
        } else {
            life_replacement(
                &mut game,
                source,
                crate::replacement::ReplacementAction::Additionally(vec![
                    Effect::create_tokens(token.clone(), 1),
                    Effect::create_tokens(token.clone(), 1),
                ]),
            );
        }
        let result = if many_payers {
            game.pay_life_simultaneously(&[(A, 2), (B, 2)])
        } else {
            game.pay_life(A, 2)
        };
        assert!(matches!(
            result,
            Err(ExecutionError::ResourceLimitExceeded { .. })
        ));
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.battlefield, vec![source]);
        assert!(game.take_pending_trigger_entries().is_empty());
        assert_eq!(
            game.turn_store
                .turn_history
                .event_kind_count(EventKind::LifePaid),
            0
        );
    }
}
#[test]
fn cost_mana_and_pay_mana_keep_temporary_replacement_scope() {
    for effect_owner in [false, true] {
        let (mut game, source) = fixture();
        let temporary = crate::replacement::ReplacementEffect::with_matcher(
            source,
            A,
            crate::events::life::matchers::WouldLoseLifeMatcher::you(),
            crate::replacement::ReplacementAction::Double,
        );
        let cost = ManaCost::from_symbols(vec![ManaSymbol::Life(2)]);
        if effect_owner {
            let mut ctx = ExecutionContext::new_default(source, A);
            ctx.replacement
                .additional_replacement_effects
                .push(temporary);
            let effect = crate::effects::PayManaEffect::new(
                cost,
                crate::ChooseSpec::Player(PlayerFilter::You),
            );
            effect.execute(&mut game, &mut ctx).unwrap();
        } else {
            let cost = crate::costs::Cost::mana(cost);
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = crate::costs::CostContext::new(source, A, &mut dm);
            ctx.replacement
                .additional_replacement_effects
                .push(temporary);
            cost.pay(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 16);
        assert_eq!(game.take_pending_trigger_entries().len(), 1);
    }
}

#[test]
fn simultaneous_life_replacement_choices_use_apnap_but_results_keep_input_order() {
    let (mut game, source) = fixture();
    game.turn.active_player = A;
    game.effect_store.replacement_effects.add_one_shot_effect(
        crate::replacement::ReplacementEffect::with_matcher(
            source,
            A,
            crate::events::life::matchers::WouldLoseLifeMatcher::any_player(),
            crate::replacement::ReplacementAction::Double,
        ),
    );
    let outcomes = game
        .pay_life_receipts_simultaneously_with_context(
            &[(B, 2), (A, 2)],
            &mut ExecutionContext::new_default(source, A),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        game.player(A).unwrap().life,
        16,
        "active player consumes shared one-shot first"
    );
    assert_eq!(game.player(B).unwrap().life, 18);
    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome
                .events
                .iter()
                .find_map(|event| event.downcast::<LifePaidEvent>())
                .unwrap()
                .player)
            .collect::<Vec<_>>(),
        vec![B, A]
    );
}
