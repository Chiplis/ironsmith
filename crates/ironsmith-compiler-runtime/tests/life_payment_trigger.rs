//! Font of Agonies: full body plus actual successful life-payment producers.
//! Source-authored scenarios, unrun during the implementation-first campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::{
    EffectContext as ExecutionContext, EffectExecutor, PayAnyLifeEffect, PayLifeEffect,
    RollDieEffect,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/life_payment_trigger.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 30);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for p in [A, B, C] {
        for symbol in [
            ironsmith::mana::ManaSymbol::Red,
            ironsmith::mana::ManaSymbol::Green,
            ironsmith::mana::ManaSymbol::Blue,
            ironsmith::mana::ManaSymbol::Black,
            ironsmith::mana::ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 30);
        }
    }
    g
}
fn object(g: &mut GameState, p: PlayerId, z: Zone, name: &str, text: &str) -> ObjectId {
    g.create_object_from_definition(
        &compile_to_runtime_definition(name, text, false).unwrap(),
        p,
        z,
    )
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    option: usize,
    accept: bool,
    pending: bool,
    pause: bool,
    number: u32,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let _ = ctx;
        self.number
    }

    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.target
            .filter(|id| {
                c.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&Target::Object(*id)))
            })
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_targets(g, c))
    }
    fn decide_objects(&mut self, _g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        c.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(c.max.unwrap_or(c.candidates.len()))
            .map(|candidate| candidate.id)
            .collect()
    }
    fn decide_mana_payment(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: c.plan.id,
            request_hash: c.plan.request_hash,
        }
    }
    fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
        vec![self.option]
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.pending = self.pause;
        self.accept
    }
}
fn stack(g: &mut GameState, events: Vec<TriggerEvent>, dm: &mut Choices) -> usize {
    for event in events {
        g.queue_trigger_event(Default::default(), event);
    }
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
    g.stack.len()
}
fn settle(g: &mut GameState, dm: &mut Choices) {
    for _ in 0..32 {
        if g.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(g, dm).unwrap();
        put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
    }
    panic!("unsettled stack")
}
fn action(g: &mut GameState, p: PlayerId, action: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(p);
    let mut state = PriorityLoopState::new(g.players.len());
    let mut q = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(c) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &c, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
}

fn font(g: &mut GameState, d: &CardDefinition) -> ObjectId {
    g.create_object_from_definition(d, A, Zone::Battlefield)
}
#[test]
fn complete_card_materializes_with_typed_paid_life_trigger_and_paid_blood_counter_activation() {
    for d in definitions("Font of Agonies") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
        let mut g = game();
        let source = font(&mut g, &d);
        let mut dm = Choices::default();
        PayLifeEffect::you(2)
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert_eq!(stack(&mut g, vec![], &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Blood), 2);
        dm.number = 2;
        PayAnyLifeEffect::new(ironsmith::ChooseSpec::Player(PlayerFilter::You), 0)
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert_eq!(stack(&mut g, vec![], &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Blood), 4);
        assert_eq!(g.player(A).unwrap().life, 26);
        let victim = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Payment victim",
            "Type: Creature — Beast\nPower/Toughness: 2/2",
        );
        dm.target = Some(victim);
        let ability_index = g
            .current_abilities(source)
            .unwrap()
            .iter()
            .position(|ability| {
                matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_))
            })
            .unwrap();
        let before = g.player(A).unwrap().mana_pool.total();
        action(
            &mut g,
            A,
            LegalAction::ActivateAbility {
                source,
                ability_index,
            },
            &mut dm,
        );
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Blood), 0);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), before - 2);
        settle(&mut g, &mut dm);
        assert!(g.object(victim).is_none());
        assert_eq!(g.player(B).unwrap().graveyard.len(), 1);
    }
}
#[test]
fn ordinary_loss_damage_and_other_payers_are_not_payment_events() {
    for d in definitions("Font of Agonies") {
        let mut g = game();
        let source = font(&mut g, &d);
        let mut dm = Choices::default();
        let outcome = ironsmith::effects::LoseLifeEffect::you(3)
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert_eq!(stack(&mut g, outcome.events, &mut dm), 0);
        let outcome =
            ironsmith::Effect::deal_damage(2, ironsmith::ChooseSpec::Player(PlayerFilter::You))
                .0
                .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
                .unwrap();
        assert_eq!(stack(&mut g, outcome.events, &mut dm), 0);
        assert!(g.pay_life(B, 4).unwrap());
        assert_eq!(stack(&mut g, vec![], &mut dm), 0);
        assert!(g.pay_life(A, 3).unwrap());
        assert_eq!(stack(&mut g, vec![], &mut dm), 1);
        g.lose_life(A, 5);
        settle(&mut g, &mut dm);
        assert_eq!(
            g.counter_count(source, ironsmith::CounterType::Blood),
            3,
            "captured payment amount is independent of later losses"
        );
    }
}
#[test]
fn actual_bulk_life_mana_payment_retains_one_completed_amount_and_queries_do_not_publish() {
    for d in definitions("Font of Agonies") {
        let mut g = game();
        let source = font(&mut g, &d);
        let mut dm = Choices::default();
        let cost = ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::mana::ManaSymbol::Life(2),
            ironsmith::mana::ManaSymbol::Life(2),
        ]);
        let request = ironsmith::mana_payment::ManaPaymentRequest::new(
            A,
            source,
            ironsmith::costs::PaymentReason::CastSpell,
            cost.clone(),
        );
        let _ = ironsmith::mana_payment::plan_mana_payment(&g, &request).unwrap();
        assert_eq!(g.player(A).unwrap().life, 30);
        assert_eq!(stack(&mut g, vec![], &mut dm), 0);
        assert!(
            g.try_pay_mana_cost_with_reason(
                A,
                Some(source),
                &cost,
                0,
                ironsmith::costs::PaymentReason::CastSpell
            )
            .unwrap()
        );
        assert_eq!(g.player(A).unwrap().life, 26);
        assert_eq!(stack(&mut g, vec![], &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Blood), 4);
    }
}
#[test]
fn actual_shock_entry_acceptance_decline_and_pending_have_distinct_payment_results() {
    for d in definitions("Font of Agonies") {
        for (accept, pause) in [(true, false), (false, false), (true, true)] {
            let mut g = game();
            let source = font(&mut g, &d);
            let mut dm = Choices {
                accept,
                pause,
                ..Default::default()
            };
            let land = ironsmith::cards::builders::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Shock entry fixture",
            )
            .card_types(vec![ironsmith::CardType::Land])
            .with_ability(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::pay_life_or_enter_tapped(2),
            ))
            .build();
            let old = g.create_object_from_definition(&land, A, Zone::Hand);
            let receipt = g
                .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, &mut dm)
                .unwrap();
            if pause {
                assert!(receipt.pending);
                assert_eq!(g.player(A).unwrap().life, 30);
                assert!(g.object(old).is_some());
                dm.pending = false;
                dm.pause = false;
                assert_eq!(stack(&mut g, vec![], &mut dm), 0);
                continue;
            }
            assert!(!receipt.pending);
            assert!(receipt.programs.is_empty());
            let new = receipt.original.into_result().unwrap().new_id;
            assert_eq!(g.is_tapped(new), !accept);
            assert_eq!(g.player(A).unwrap().life, if accept { 28 } else { 30 });
            assert_eq!(stack(&mut g, vec![], &mut dm), usize::from(accept));
            settle(&mut g, &mut dm);
            assert_eq!(
                g.counter_count(source, ironsmith::CounterType::Blood),
                if accept { 2 } else { 0 }
            );
        }
    }
}
#[test]
fn paid_die_modifier_uses_the_same_success_receipt_and_roll_pause_rolls_it_back() {
    for d in definitions("Font of Agonies") {
        let mut g = game();
        let source = font(&mut g, &d);
        let mut dm = Choices {
            accept: true,
            ..Default::default()
        };
        let modifier =
            ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Paid die modifier")
                .card_types(vec![ironsmith::CardType::Artifact])
                .with_ability(ironsmith::ability::Ability::static_ability(
                    ironsmith::static_abilities::StaticAbility::die_roll_result_adjustment(
                        PlayerFilter::You,
                        2,
                        1,
                        false,
                        "pay 2 life to adjust a die",
                    ),
                ))
                .build();
        g.create_object_from_definition(&modifier, A, Zone::Battlefield);
        g.force_next_die_roll(3);
        let outcome = RollDieEffect::new(PlayerFilter::You, 6)
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert_eq!(outcome.as_count(), Some(4));
        assert_eq!(g.player(A).unwrap().life, 28);
        assert_eq!(stack(&mut g, outcome.events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Blood), 2);
        dm.pause = true;
        g.force_next_die_roll(3);
        RollDieEffect::new(PlayerFilter::You, 6)
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert!(dm.pending);
        assert_eq!(g.player(A).unwrap().life, 28);
        dm.pause = false;
        dm.pending = false;
        assert_eq!(stack(&mut g, vec![], &mut dm), 0);
    }
}

#[test]
fn font_counts_the_accepted_payment_even_when_its_life_loss_action_is_replaced() {
    for d in definitions("Font of Agonies") {
        for replace in [false, true] {
            let mut g = game();
            let source = font(&mut g, &d);
            let mut dm = Choices::default();
            let replacement = if replace {
                ironsmith::replacement::ReplacementAction::Instead(vec![
                    ironsmith::Effect::gain_life(3),
                ])
            } else {
                ironsmith::replacement::ReplacementAction::Double
            };
            g.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    source,
                    A,
                    ironsmith::events::life::matchers::WouldLoseLifeMatcher::you(),
                    replacement,
                ),
            );
            let cost = ironsmith::costs::Cost::life(2);
            cost.pay(
                &mut g,
                &mut ironsmith::costs::CostContext::new(source, A, &mut dm),
            )
            .unwrap();
            assert_eq!(g.player(A).unwrap().life, if replace { 33 } else { 26 });
            assert_eq!(stack(&mut g, vec![], &mut dm), 1);
            settle(&mut g, &mut dm);
            assert_eq!(g.counter_count(source, ironsmith::CounterType::Blood), 2);
        }
    }
}
