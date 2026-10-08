//! UNRUN source-only regressions for the source-counter conversion bridge.

use super::*;
use ironsmith::costs::{Cost, CostContext, CostPaymentResult, PaymentReason};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{CountersContext, NumberContext};
use ironsmith::effect::{Effect, EffectId};
use ironsmith::effect::EffectPredicateRuntimeExt as _;
use ironsmith::effects::RemoveAnyCountersFromSourceEffect;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_core::CounterType;
use ironsmith_compiled_artifact::WireEffect;
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_effect, materialize_effect};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const RECEIPT: EffectId = EffectId(73);

/// These are three independent entry routes. The native route deliberately has
/// no retained serialized model, so it must exercise the native encoder table.
fn routes(model: &RemoveAnyCountersFromSourceEffect) -> [Effect; 3] {
    let direct = runtime_effect_from_core_model(compiler::effect::Effect::new(model.clone())).unwrap();
    let expected = WireEffect::new("RemoveAnyCountersFromSourceEffect", serde_json::to_value(model).unwrap());
    let json = serde_json::to_string(&expected).unwrap();
    let artifact = materialize_effect(serde_json::from_str(&json).unwrap()).unwrap();
    let native = Effect::new(model.clone());
    assert!(native.serialized_model().is_none());
    let encoded_native = encode_runtime_effect(native).unwrap();
    assert_eq!(encoded_native, expected);
    let restored_native = materialize_effect(encoded_native).unwrap();
    for effect in [&direct, &artifact, &restored_native] {
        assert_eq!(effect.downcast_ref::<RemoveAnyCountersFromSourceEffect>(), Some(model));
        assert_eq!(encode_runtime_effect(effect.clone()).unwrap(), expected);
    }
    [direct, artifact, restored_native]
}

fn object(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
    let card = ironsmith::card::CardBuilder::new(CardId::new(), "Counter payload source")
        .card_types(vec![CardType::Artifact]).build();
    game.create_object_from_card(&card, owner, zone)
}

struct Choices {
    source: ObjectId,
    number: u32,
    maximum: u32,
    counters: Vec<(CounterType, u32)>,
    number_calls: usize,
    counter_calls: usize,
    pause_number: bool,
    pending: bool,
}

impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }

    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        assert_eq!(ctx.player, B);
        assert_eq!(ctx.source, Some(self.source));
        assert_eq!((ctx.min, ctx.max), (0, self.maximum));
        self.number_calls += 1;
        self.pending = self.pause_number;
        self.number
    }

    fn decide_counters(&mut self, _: &GameState, ctx: &CountersContext) -> Vec<(CounterType, u32)> {
        assert_eq!(ctx.player, B);
        assert_eq!(ctx.source, Some(self.source));
        assert_eq!(ctx.target, Target::Object(self.source));
        assert_eq!(ctx.min_total, u64::from(self.number));
        assert_eq!(ctx.max_total, u64::from(self.number));
        assert!(ctx.available_counters.contains(&(CounterType::Charge, 5)));
        assert!(ctx.available_counters.contains(&(CounterType::PlusOnePlusOne, 2)));
        self.counter_calls += 1;
        self.counters.clone()
    }
}

#[derive(Debug, Clone)]
struct ExactPaymentCause {
    source: ObjectId,
    cause: ironsmith_core::EventCause,
}

impl ironsmith::events::traits::ReplacementMatcher for ExactPaymentCause {
    fn matches_prepared_event(
        &self,
        event: &dyn ironsmith::events::traits::GameEventType,
        _: &ironsmith::events::context::PreparedEventContext,
    ) -> bool {
        let Some(removal) = event.as_any().downcast_ref::<ironsmith::events::counters::RemoveCountersEvent>() else {
            return false;
        };
        removal.target == self.source
            && removal.source == Some(self.source)
            && removal.actor == Some(B)
            && removal.cause == self.cause
    }

    fn display(&self) -> String { "Observe the exact counter payment cause".into() }
}

#[test]
fn source_counter_payload_routes_preserve_every_existing_field() {
    for counter_type in [None, Some(CounterType::Loyalty), Some(CounterType::Named("elixir".into()))] {
        for display_x in [false, true] {
            for remove_all in [false, true] {
                routes(&RemoveAnyCountersFromSourceEffect { counter_type, display_x, remove_all });
            }
        }
    }
}

#[test]
fn source_counter_payload_routes_pay_exact_source_quantity_type_and_reason() {
    for restricted in [false, true] {
        for mode in 0..3 {
            let counter_type = restricted.then_some(CounterType::Charge);
            let model = match mode {
                0 => RemoveAnyCountersFromSourceEffect::any_number(counter_type),
                1 => RemoveAnyCountersFromSourceEffect::x(counter_type),
                _ => RemoveAnyCountersFromSourceEffect::all(counter_type),
            };
            for effect in routes(&model) {
                for reason in [PaymentReason::Other, PaymentReason::ActivateAbility,
                    PaymentReason::ActivateManaAbility, PaymentReason::Effect] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let source = object(&mut game, B, Zone::Battlefield);
                    let other = object(&mut game, A, Zone::Battlefield);
                    for id in [source, other] {
                        game.object_mut(id).unwrap().counters.insert(CounterType::Charge, 5);
                        game.object_mut(id).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
                    }
                    let maximum = if restricted { 5 } else { 7 };
                    let amount = if mode == 2 { maximum } else { 2 };
                    let selections = if mode == 2 {
                        vec![(CounterType::PlusOnePlusOne, 2), (CounterType::Charge, 5)]
                    } else {
                        vec![(CounterType::PlusOnePlusOne, 2)]
                    };
                    let mut choices = Choices { source, number: amount, maximum, counters: selections,
                        number_calls: 0, counter_calls: 0, pause_number: false, pending: false };
                    let cost = Cost::try_effect(Effect::with_id(RECEIPT.0, effect.clone())).unwrap();
                    let mut ctx = CostContext::new(source, B, &mut choices).with_reason(reason);
                    if mode == 1 { ctx.x_value = Some(amount); }
                    if reason == PaymentReason::Effect {
                        ctx.requesting_effect_cause = Some(ironsmith_core::EventCause::from_spell_resolution(other, A));
                    }
                    let cause = if reason == PaymentReason::Effect {
                        ironsmith_core::EventCause {
                            cause_type: ironsmith_core::CauseType::Cost,
                            ..ironsmith_core::EventCause::from_spell_resolution(other, A)
                        }
                    } else { ironsmith_core::EventCause::from_cost(source, B) };
                    let observer = game.effect_store.replacement_effects.add_one_shot_effect(
                        ironsmith::replacement::ReplacementEffect::with_matcher(source, B,
                            ExactPaymentCause { source, cause },
                            ironsmith::replacement::ReplacementAction::Modify(ironsmith::replacement::EventModification::Subtract(0))));
                    let receipt = cost.pay_with_outputs(&mut game, &mut ctx).unwrap();
                    assert_eq!(receipt.result, CostPaymentResult::Paid);
                    assert_eq!(ctx.x_value, Some(amount));
                    assert_eq!(ctx.reason, reason);
                    assert_eq!(ctx.effect_outcomes[&RECEIPT].count_or_zero(), i64::from(amount));
                    let outputs = receipt.outputs.unwrap();
                    assert_eq!(outputs.outcome.count_or_zero(), i64::from(amount));
                    assert!(game.effect_store.replacement_effects.get_effect(observer).is_none(),
                        "the actual removal must preserve its payment cause and attribution");
                    drop(ctx);
                    assert_eq!(choices.number_calls, usize::from(mode == 0));
                    assert_eq!(choices.counter_calls, usize::from(!restricted));
                    assert_eq!(game.counter_count(source, CounterType::Charge),
                        if restricted { 5 - amount } else if mode == 2 { 0 } else { 5 });
                    assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), if restricted { 2 } else { 0 });
                    assert_eq!(game.counter_count(other, CounterType::Charge), 5);
                    assert_eq!(game.counter_count(other, CounterType::PlusOnePlusOne), 2);
                    for marker in outputs.outcome.events_of_type::<ironsmith::events::MarkersChangedEvent>() {
                        assert_eq!(marker.location, ironsmith::marker::MarkerLocation::Object(source));
                        assert_eq!(marker.source, Some(source));
                        assert_eq!(marker.source_controller, Some(B));
                        assert!(marker.is_removed());
                    }
                }
            }
        }
    }
}

#[test]
fn source_counter_payload_routes_preserve_unsigned_all_and_zero_payments() {
    for amount in [0, i32::MAX as u32 + 1, u32::MAX] {
        for effect in routes(&RemoveAnyCountersFromSourceEffect::all(Some(CounterType::Charge))) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = object(&mut game, B, Zone::Battlefield);
            game.object_mut(source).unwrap().counters.insert(CounterType::Charge, amount);
            game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne, 9);
            let mut choices = SelectFirstDecisionMaker;
            let mut ctx = CostContext::new(source, B, &mut choices);
            let receipt = Cost::try_effect(effect).unwrap().pay_with_outputs(&mut game, &mut ctx).unwrap();
            assert_eq!(receipt.result, CostPaymentResult::Paid);
            let outputs = receipt.outputs.unwrap();
            assert_eq!(outputs.outcome.count_or_zero(), i64::from(amount));
            assert!(ironsmith::effect::EffectPredicate::Happened.evaluate_outcome(&outputs.outcome),
                "an acknowledged zero counter quantity remains an accepted cost");
            assert_eq!(ctx.x_value, Some(amount));
            assert_eq!(game.counter_count(source, CounterType::Charge), 0);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 9);
        }
    }
}

#[test]
fn source_counter_payment_keeps_nominal_quantity_separate_from_prevented_removal() {
    for effect in routes(&RemoveAnyCountersFromSourceEffect::all(Some(CounterType::Charge))) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = object(&mut game, B, Zone::Battlefield);
        let requester = object(&mut game, A, Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 5);
        let requesting = ironsmith_core::EventCause::from_spell_resolution(requester, A);
        let cause = ironsmith_core::EventCause { cause_type: ironsmith_core::CauseType::Cost, ..requesting.clone() };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(source, B,
                ExactPaymentCause { source, cause }, ironsmith::replacement::ReplacementAction::Prevent));
        let cost = Cost::try_effect(Effect::with_id(RECEIPT.0, effect)).unwrap();
        let mut choices = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, B, &mut choices).with_reason(PaymentReason::Effect);
        ctx.requesting_effect_cause = Some(requesting.clone());
        let receipt = cost.pay_with_outputs(&mut game, &mut ctx).unwrap();
        assert_eq!(receipt.result, CostPaymentResult::Paid);
        assert_eq!(ctx.x_value, Some(5), "cost X owns the nominal quantity");
        assert_eq!(ctx.effect_outcomes[&RECEIPT].instruction_result().count_or_zero(), 0);
        assert_eq!(ctx.requesting_effect_cause, Some(requesting));
        let outputs = receipt.outputs.unwrap();
        assert_eq!(outputs.outcome.count_or_zero(), 0, "the original receipt owns actual removals");
        assert_eq!(outputs.outcome.status, ironsmith::effect::OutcomeStatus::Succeeded);
        assert!(ironsmith::effect::EffectPredicate::Happened.evaluate_outcome(&outputs.outcome),
            "a physically prevented counter removal still paid the accepted cost");
        fn retains_prevented_child(outputs: &ironsmith::effects::CompletedEffectOutputs) -> bool {
            outputs.outcome.instruction_result().status == ironsmith::effect::OutcomeStatus::Prevented
                || outputs.shared.iter().any(|child| retains_prevented_child(&child.outputs))
                || outputs.participants.iter().any(|child| retains_prevented_child(&child.outputs))
        }
        assert!(retains_prevented_child(&outputs), "the payment acknowledgement cannot erase its physical child's receipt");
        assert_eq!(outputs.outcome.events_of_type::<ironsmith::events::MarkersChangedEvent>().count(), 0);
        assert_eq!(game.counter_count(source, CounterType::Charge), 5);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
}

struct AcceptPayments;
impl DecisionMaker for AcceptPayments {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
}

#[test]
fn prevented_all_and_announced_x_costs_still_take_if_you_do_and_skip_unless_consequences() {
    for announced_x in [false, true] {
        let model = if announced_x {
            RemoveAnyCountersFromSourceEffect::x(Some(CounterType::Charge))
        } else { RemoveAnyCountersFromSourceEffect::all(Some(CounterType::Charge)) };
        for effect in routes(&model) {
            for optional in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = object(&mut game, B, Zone::Battlefield);
                let requester = object(&mut game, A, Zone::Battlefield);
                game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 5);
                game.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(source, B,
                        ironsmith::events::counters::matchers::WouldRemoveCountersMatcher::new(
                            ironsmith::target::ObjectFilter::specific(source), Some(CounterType::Charge)),
                        ironsmith::replacement::ReplacementAction::Prevent));
                let owned_cost = Effect::with_id(RECEIPT.0, effect.clone());
                let instruction = if optional {
                    Effect::with_id(75, Effect::new(ironsmith::effects::MayEffect::new(vec![owned_cost]).with_pay_as_cost(true)))
                } else {
                    Effect::new(ironsmith::effects::UnlessPaysEffect::new_total_cost(
                        vec![Effect::lose_life(9)], ironsmith::target::PlayerFilter::You,
                        ironsmith::cost::TotalCost::from_costs(vec![Cost::try_effect(owned_cost).unwrap()])))
                };
                let mut decisions = AcceptPayments;
                let mut ctx = ironsmith::effects::EffectContext::new(source, B, &mut decisions)
                    .with_cause(ironsmith_core::EventCause::from_spell_resolution(requester, A));
                if announced_x { ctx.x_value = Some(2); }
                ironsmith::effects::execute_effect(&mut game, &instruction, &mut ctx).unwrap();
                assert_eq!(ctx.x_value, Some(if announced_x { 2 } else { 5 }));
                assert_eq!(ctx.effect_outcomes[&RECEIPT].instruction_result().count_or_zero(), 0);
                assert_eq!(game.counter_count(source, CounterType::Charge), 5);
                assert_eq!(game.player(B).unwrap().life, 20, "UnlessPays must not take its unpaid consequence");
                if optional {
                    let follow = Effect::if_then(EffectId(75), ironsmith::effect::EffectPredicate::Happened,
                        vec![Effect::gain_life(3)]);
                    ironsmith::effects::execute_effect(&mut game, &follow, &mut ctx).unwrap();
                    assert_eq!(game.player(B).unwrap().life, 23, "If-you-do must see the accepted payment");
                }
            }
        }
    }
}

#[test]
fn mixed_counter_payment_projects_physical_removals_without_replacement_addition_counts() {
    for replace_with_life in [false, true] {
        for effect in routes(&RemoveAnyCountersFromSourceEffect::all(None)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = object(&mut game, B, Zone::Battlefield);
            game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 5);
            game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
            let action = if replace_with_life {
                ironsmith::replacement::ReplacementAction::Instead(vec![Effect::gain_life(11)])
            } else {
                ironsmith::replacement::ReplacementAction::Modify(ironsmith::replacement::EventModification::Subtract(2))
            };
            game.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(source, B,
                    ironsmith::events::counters::matchers::WouldRemoveCountersMatcher::new(
                        ironsmith::target::ObjectFilter::specific(source), Some(CounterType::Charge)), action));
            let mut choices = Choices { source, number: 7, maximum: 7,
                counters: vec![(CounterType::PlusOnePlusOne, 2), (CounterType::Charge, 5)],
                number_calls: 0, counter_calls: 0, pause_number: false, pending: false };
            let cost = Cost::try_effect(Effect::with_id(RECEIPT.0, effect)).unwrap();
            let mut ctx = CostContext::new(source, B, &mut choices);
            let receipt = cost.pay_with_outputs(&mut game, &mut ctx).unwrap();
            let physical = if replace_with_life { 2 } else { 5 };
            assert_eq!(receipt.result, CostPaymentResult::Paid);
            assert_eq!(ctx.x_value, Some(7));
            assert_eq!(ctx.effect_outcomes[&RECEIPT].instruction_result().count_or_zero(), physical);
            let outputs = receipt.outputs.unwrap();
            assert_eq!(outputs.outcome.count_or_zero(), physical);
            assert_eq!(outputs.outcome.instruction_result().count_or_zero(), physical);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.counter_count(source, CounterType::Charge), if replace_with_life { 5 } else { 2 });
            assert_eq!(game.player(B).unwrap().life, if replace_with_life { 31 } else { 20 });
            assert_eq!(outputs.outcome.events_of_type::<ironsmith::events::LifeGainEvent>().count(), usize::from(replace_with_life));
        }
    }
}

struct InterruptedPaymentChoices { source: ObjectId, pending: bool }
impl DecisionMaker for InterruptedPaymentChoices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_counters(&mut self, _: &GameState, ctx: &CountersContext) -> Vec<(CounterType, u32)> {
        assert_eq!(ctx.player, B);
        assert_eq!(ctx.target, Target::Object(self.source));
        assert_eq!((ctx.min_total, ctx.max_total), (7, 7));
        vec![(CounterType::PlusOnePlusOne, 2), (CounterType::Charge, 5)]
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
        assert_eq!(ctx.player, B);
        self.pending = true;
        false
    }
}

#[derive(Debug, Clone)]
struct InterruptLaterCounterReplacement { source: ObjectId, suspend: bool }
impl ironsmith::effects::EffectExecutor for InterruptLaterCounterReplacement {
    fn execute(
        &self, game: &mut GameState, ctx: &mut ironsmith::effects::EffectContext,
    ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        assert_eq!(game.counter_count(self.source, CounterType::PlusOnePlusOne), 0,
            "the earlier group's original counter removal must have committed");
        game.player_mut(B).unwrap().life += 3;
        if self.suspend {
            let prompt = ironsmith::decisions::context::BooleanContext::new(B, Some(self.source), "Pause later counter replacement");
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(ironsmith::effect::EffectOutcome::resolved())
        } else {
            Err(ironsmith::effects::ExecutionError::InternalError("later counter replacement failed".into()))
        }
    }
}

#[test]
fn mixed_counter_payment_rolls_back_committed_originals_and_later_program_on_error_or_pending() {
    for suspend in [false, true] {
        for effect in routes(&RemoveAnyCountersFromSourceEffect::all(None)) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = object(&mut game, B, Zone::Battlefield);
            let requester = object(&mut game, A, Zone::Battlefield);
            game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 5);
            game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(source, B,
                    ironsmith::events::counters::matchers::WouldRemoveCountersMatcher::new(
                        ironsmith::target::ObjectFilter::specific(source), Some(CounterType::Charge)),
                    ironsmith::replacement::ReplacementAction::Instead(vec![Effect::new(
                        InterruptLaterCounterReplacement { source, suspend })])));
            let requesting = ironsmith_core::EventCause::from_spell_resolution(requester, A);
            let mut choices = InterruptedPaymentChoices { source, pending: false };
            let mut ctx = CostContext::new(source, B, &mut choices).with_reason(PaymentReason::Effect);
            ctx.requesting_effect_cause = Some(requesting.clone());
            ctx.effect_outcomes.insert(RECEIPT, ironsmith::effect::EffectOutcome::count(99));
            let cost = Cost::try_effect(Effect::with_id(RECEIPT.0, effect)).unwrap();
            let result = cost.pay_with_outputs(&mut game, &mut ctx);
            if suspend {
                let suspended = result.unwrap();
                assert_eq!(suspended.result, CostPaymentResult::Paid);
                assert!(suspended.outputs.is_none());
                assert!(ctx.decision_maker.awaiting_choice());
            } else {
                assert!(matches!(result, Err(ironsmith::cost::CostPaymentError::ExecutionFailed(
                    ironsmith::effects::ExecutionError::InternalError(ref detail))) if detail == "later counter replacement failed"));
            }
            assert_eq!(ctx.x_value, None);
            assert_eq!(ctx.reason, PaymentReason::Effect);
            assert_eq!(ctx.requesting_effect_cause, Some(requesting));
            assert_eq!(ctx.effect_outcomes[&RECEIPT].count_or_zero(), 99);
            assert_eq!(game.counter_count(source, CounterType::Charge), 5);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
            assert_eq!(game.player(B).unwrap().life, 20);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::MarkersChanged), 0);
        }
    }
}

#[test]
fn source_counter_payload_routes_reject_unpayable_x_and_missing_battlefield_source() {
    for effect in routes(&RemoveAnyCountersFromSourceEffect::x(Some(CounterType::Charge))) {
        for zone in [Zone::Battlefield, Zone::Graveyard] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = object(&mut game, B, zone);
            game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 2);
            let mut choices = SelectFirstDecisionMaker;
            let announced = if zone == Zone::Battlefield { 3 } else { 1 };
            let requesting = ironsmith_core::EventCause::from_spell_resolution(ObjectId::from_raw(900), A);
            let mut ctx = CostContext::new(source, B, &mut choices).with_x(announced).with_reason(PaymentReason::Effect);
            ctx.requesting_effect_cause = Some(requesting.clone());
            ctx.effect_outcomes.insert(RECEIPT, ironsmith::effect::EffectOutcome::count(99));
            let cost = Cost::try_effect(Effect::with_id(RECEIPT.0, effect.clone())).unwrap();
            assert!(cost.pay(&mut game, &mut ctx).is_err());
            assert_eq!(ctx.x_value, Some(announced));
            assert_eq!(ctx.requesting_effect_cause, Some(requesting));
            assert_eq!(ctx.reason, PaymentReason::Effect);
            assert_eq!(ctx.effect_outcomes[&RECEIPT].count_or_zero(), 99);
            assert_eq!(game.counter_count(source, CounterType::Charge), 2);
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[test]
fn source_counter_payload_routes_suspend_without_payment_then_resume_choice() {
    for effect in routes(&RemoveAnyCountersFromSourceEffect::any_number(Some(CounterType::Charge))) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = object(&mut game, B, Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 5);
        let mut choices = Choices { source, number: 2, maximum: 5, counters: vec![], number_calls: 0,
            counter_calls: 0, pause_number: true, pending: false };
        let cost = Cost::try_effect(Effect::with_id(RECEIPT.0, effect)).unwrap();
        {
            let requesting = ironsmith_core::EventCause::from_spell_resolution(ObjectId::from_raw(900), A);
            let mut ctx = CostContext::new(source, B, &mut choices).with_reason(PaymentReason::Effect);
            ctx.requesting_effect_cause = Some(requesting.clone());
            ctx.effect_outcomes.insert(RECEIPT, ironsmith::effect::EffectOutcome::count(99));
            let suspended = cost.pay_with_outputs(&mut game, &mut ctx).unwrap();
            // The existing adapter returns a neutral result while the decision
            // maker owns suspension; no payment receipt is acknowledged.
            assert_eq!(suspended.result, CostPaymentResult::Paid);
            assert!(suspended.outputs.is_none());
            assert!(ctx.decision_maker.awaiting_choice());
            assert_eq!(ctx.x_value, None);
            assert_eq!(ctx.effect_outcomes[&RECEIPT].count_or_zero(), 99);
            assert_eq!(ctx.requesting_effect_cause, Some(requesting));
            assert_eq!(ctx.reason, PaymentReason::Effect);
        }
        assert_eq!(game.counter_count(source, CounterType::Charge), 5);
        assert!(game.take_pending_trigger_events().is_empty());
        choices.pause_number = false;
        choices.pending = false;
        let mut ctx = CostContext::new(source, B, &mut choices);
        assert_eq!(cost.pay(&mut game, &mut ctx).unwrap(), CostPaymentResult::Paid);
        assert_eq!(ctx.x_value, Some(2));
        assert_eq!(ctx.effect_outcomes[&RECEIPT].count_or_zero(), 2);
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
    }
}
