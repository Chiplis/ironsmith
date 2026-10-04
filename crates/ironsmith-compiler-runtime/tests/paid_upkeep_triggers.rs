//! Successful whole upkeep payments, including explicitly accepted zero-cost echo.
//! Source-authored scenarios, unrun during the implementation-first campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::{
    CumulativeUpkeepEffect, EffectContext as ExecutionContext, EffectExecutor,
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
        "../../../fixtures/paid_upkeep_triggers.json.fixture"
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
        if ctx.player == B { 2 } else { 0 }
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

fn cast_source(g: &mut GameState, d: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let card = g.create_object_from_definition(d, A, Zone::Hand);
    let stable = g.object(card).unwrap().stable_id;
    action(
        g,
        A,
        LegalAction::CastSpell {
            spell_id: card,
            from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        },
        dm,
    );
    settle(g, dm);
    let source = g.find_object_by_stable_id(stable).unwrap();
    assert_eq!(g.object(source).unwrap().zone, Zone::Battlefield);
    source
}
fn upkeep(g: &mut GameState, dm: &mut Choices) -> usize {
    g.turn.phase = ironsmith::Phase::Beginning;
    g.turn.step = Some(ironsmith::game_state::Step::Upkeep);
    g.mark_upkeep_began(A);
    stack(
        g,
        vec![TriggerEvent::new(
            ironsmith::events::phase::BeginningOfUpkeepEvent::new(A),
            Default::default(),
        )],
        dm,
    )
}
fn payment_notices(
    out: &ironsmith::effect::EffectOutcome,
    kind: ironsmith::events::KeywordActionKind,
) -> usize {
    out.events
        .iter()
        .filter(|event| {
            event
                .downcast::<ironsmith::events::other::KeywordActionEvent>()
                .is_some_and(|event| event.action == kind)
        })
        .count()
}
#[test]
fn both_paid_upkeep_bodies_and_echo_mode_round_trip_without_loss() {
    assert_eq!(fixtures().len(), 2);
    for row in fixtures() {
        for d in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
        }
    }
    let old = serde_json::json!({"player":"You","payment":[],"failure":[]});
    let decoded: ironsmith_core::CumulativeUpkeepEffect<u8> = serde_json::from_value(old).unwrap();
    assert_eq!(
        decoded.kind,
        ironsmith_core::effect::UpkeepPaymentKind::Cumulative
    );
    let echo =
        ironsmith_core::CumulativeUpkeepEffect::<u8>::echo(PlayerFilter::You, vec![], vec![]);
    let decoded: ironsmith_core::CumulativeUpkeepEffect<u8> =
        serde_json::from_value(serde_json::to_value(&echo).unwrap()).unwrap();
    assert_eq!(decoded, echo);
}
#[test]
fn cumulative_upkeep_emits_one_notice_for_the_whole_payment_then_searches_current_age_count() {
    for d in definitions("Hibernation's End") {
        let mut g = game();
        let mut dm = Choices {
            accept: true,
            ..Default::default()
        };
        let source = cast_source(&mut g, &d, &mut dm);
        for (name, cost) in [("One mana creature", "{1}"), ("Three mana creature", "{3}")] {
            object(
                &mut g,
                A,
                Zone::Library,
                name,
                &format!("Mana cost: {cost}\nType: Creature — Beast\nPower/Toughness: 1/1"),
            );
        }
        let mana = g.player(A).unwrap().mana_pool.total();
        assert_eq!(upkeep(&mut g, &mut dm), 1);
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Age), 1);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 1);
        let mut q = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
        assert_eq!(g.stack.len(), 1);
        // The follow-up's X is the source's actual counter count when its
        // search resolves, not the preceding payment's captured installments.
        g.add_counters(source, ironsmith::CounterType::Age, 2);
        settle(&mut g, &mut dm);
        assert!(g.battlefield.iter().any(|id| {
            g.object(*id)
                .is_some_and(|object| object.name.as_ref() == "Three mana creature")
        }));
        assert_eq!(g.player(A).unwrap().library.len(), 1);

        let payment = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![ironsmith::Effect::new(
                ironsmith::effects::PayManaEffect::new(
                    ironsmith::mana::ManaCost::from_pips(vec![vec![
                        ironsmith::mana::ManaSymbol::Generic(1),
                    ]]),
                    ironsmith::target::ChooseSpec::SourceController,
                ),
            )],
            vec![ironsmith::Effect::sacrifice_source()],
        );
        let mana = g.player(A).unwrap().mana_pool.total();
        let out = payment
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 3);
        assert_eq!(
            payment_notices(
                &out,
                ironsmith::events::KeywordActionKind::CumulativeUpkeepPaid
            ),
            1
        );
        assert_eq!(
            stack(&mut g, out.events, &mut dm),
            1,
            "three installments are one payment event and captured receipts are idempotent"
        );
    }
}
#[test]
fn shah_requires_accepting_zero_echo_and_each_opponent_chooses_its_own_draw_count() {
    for d in definitions("Shah of Naar Isle") {
        for accept in [false, true] {
            let mut g = game();
            let mut dm = Choices {
                accept,
                ..Default::default()
            };
            let source = cast_source(&mut g, &d, &mut dm);
            assert!(g.current_has_static_ability_id(
                source,
                ironsmith::static_abilities::StaticAbilityId::Trample
            ));
            for p in [A, B, C] {
                for _ in 0..3 {
                    object(&mut g, p, Zone::Library, "Draw resource", "Type: Land");
                }
            }
            assert_eq!(upkeep(&mut g, &mut dm), 1);
            let mana = g.player(A).unwrap().mana_pool.total();
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), mana);
            assert_eq!(g.object(source).is_some(), accept);
            assert_eq!(g.player(A).unwrap().hand.len(), 0);
            assert_eq!(g.player(B).unwrap().hand.len(), if accept { 2 } else { 0 });
            assert_eq!(g.player(C).unwrap().hand.len(), 0);
            if accept {
                g.turn.turn_number += 1;
                assert_eq!(
                    upkeep(&mut g, &mut dm),
                    0,
                    "echo does not repeat without a new control interval"
                );
            }
        }
    }
}
#[test]
fn pending_zero_echo_payment_and_native_restore_publish_no_premature_paid_notice() {
    for d in definitions("Shah of Naar Isle") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let saved = g.clone();
        let payment = CumulativeUpkeepEffect::echo(
            PlayerFilter::You,
            vec![],
            vec![ironsmith::Effect::sacrifice_source()],
        );
        let mut dm = Choices {
            accept: true,
            pause: true,
            ..Default::default()
        };
        let out = payment
            .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
            .unwrap();
        assert!(out.events.is_empty());
        assert!(g.object(source).is_some());
        assert!(g.take_pending_trigger_events().is_empty());
        dm.pause = false;
        dm.pending = false;
        for restore in [false, true] {
            if restore {
                g = saved.clone();
            }
            let out = payment
                .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
                .unwrap();
            assert_eq!(
                payment_notices(&out, ironsmith::events::KeywordActionKind::EchoCostPaid),
                1
            );
            assert_eq!(stack(&mut g, out.events, &mut dm), 1);
            g.stack.clear();
        }
    }
}
#[test]
fn insufficient_or_declined_cumulative_payment_never_emits_a_success_notice() {
    for d in definitions("Hibernation's End") {
        for can_pay in [false, true] {
            let mut g = game();
            let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
            g.add_counters(source, ironsmith::CounterType::Age, 2);
            if !can_pay {
                g.player_mut(A).unwrap().mana_pool.empty();
            }
            let mut dm = Choices {
                accept: !can_pay,
                ..Default::default()
            };
            let payment = CumulativeUpkeepEffect::new(
                PlayerFilter::You,
                vec![ironsmith::Effect::new(
                    ironsmith::effects::PayManaEffect::new(
                        ironsmith::mana::ManaCost::from_pips(vec![vec![
                            ironsmith::mana::ManaSymbol::Generic(1),
                        ]]),
                        ironsmith::target::ChooseSpec::SourceController,
                    ),
                )],
                vec![ironsmith::Effect::sacrifice_source()],
            );
            let out = payment
                .execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm))
                .unwrap();
            assert_eq!(
                payment_notices(
                    &out,
                    ironsmith::events::KeywordActionKind::CumulativeUpkeepPaid
                ),
                0
            );
            assert!(g.object(source).is_none());
            assert_eq!(stack(&mut g, out.events, &mut dm), 0);
        }
    }
}
