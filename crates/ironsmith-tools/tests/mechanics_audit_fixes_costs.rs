//! September 25, 2026 CR regressions for audit F16–F20 and F22.
use ironsmith::ability::Ability;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder as B};
use ironsmith::cost::{OptionalCostKind, TotalCost};
use ironsmith::costs::{Cost, CostContext};
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::effect::{Effect, Until, Value};
use ironsmith::effects::{
    CostExecutableEffect, CrewCostEffect, EffectContext, EffectExecutor, PutCountersEffect,
    SaddleCostEffect,
};
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{Phase, Target};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
#[derive(Default)]
struct Dm {
    target: Option<ObjectId>,
    chosen: Option<ObjectId>,
}
impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.chosen
            .filter(|id| c.candidates.iter().any(|x| x.id == *id))
            .map(|id| vec![id])
            .unwrap_or_else(|| c.candidates.iter().take(c.min).map(|x| x.id).collect())
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        if let Some(id) = self.target {
            return vec![Target::Object(id)];
        }
        c.requirements
            .iter()
            .flat_map(|r| r.legal_targets.iter().take(r.min_targets).copied())
            .collect()
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g
}
fn creature(name: &str, p: i32, t: i32) -> CardDefinition {
    B::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build()
}
fn custom(name: &str, types: Vec<CardType>, text: &str) -> CardDefinition {
    let name = name.to_owned();
    let text = text.to_owned();
    // Grammar and compiler-to-runtime adapters hold large typed enum values
    // in debug builds. Keep their stack separate from the gameplay scenario.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            ironsmith_registry::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(types)
                .mana_cost(ManaCost::new())
                .parse_text(text)
                .unwrap()
        })
        .unwrap()
        .join()
        .unwrap()
}
fn act(g: &mut GameState, action: LegalAction, dm: &mut Dm) {
    let before = g.stack.len();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..32 {
        if g.stack.len() > before {
            break;
        }
        if let GameProgress::NeedsDecisionCtx(c) = progress {
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                g, &mut queue, &mut state, &c, dm,
            )
            .unwrap();
        } else {
            break;
        }
    }
    ironsmith::game_loop::advance_priority_with_dm(g, &mut queue, dm).unwrap();
}
fn settle(g: &mut GameState, dm: &mut Dm) {
    for _ in 0..32 {
        let mut q = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(g, &mut q);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
        if g.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(g, dm).unwrap();
    }
    panic!("did not settle")
}
fn activate(g: &mut GameState, id: ObjectId, dm: &mut Dm) {
    let action = compute_legal_actions(g, A).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source,..} if *source==id))
        .expect("activation legal");
    act(g, action, dm);
}
fn add_type(g: &mut GameState, id: ObjectId, kind: CardType) {
    let effect = ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(id),
        ironsmith::continuous::Modification::AddCardTypes(vec![kind]),
        Until::Forever,
    );
    effect
        .execute(g, &mut EffectContext::new_default(id, A))
        .unwrap();
    g.refresh_continuous_state();
}
#[test]
fn f16_improvise_uses_live_artifact_types_and_excludes_phased_permanents() {
    let mut g = game();
    let id = g.create_object_from_definition(&creature("Helper", 2, 2), A, Zone::Battlefield);
    let spell = B::new(CardId::new(), "Improvise")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new().add_generic(1))
        .with_ability(Ability::static_ability(StaticAbility::improvise()))
        .build();
    let spell = g.create_object_from_definition(&spell, A, Zone::Hand);
    let legal = |g: &GameState| {
        compute_legal_actions(g, A).expect("fixture has complete replacement state")
            .iter()
            .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell))
    };
    assert!(!legal(&g));
    add_type(&mut g, id, CardType::Artifact);
    assert!(legal(&g));
    g.phase_out(id);
    assert!(ironsmith::decision::get_improvise_artifacts(&g, A).is_empty());
    assert!(!legal(&g));
}
#[test]
fn f17_ward_uses_current_then_departure_power_on_actual_stack() {
    for (leave, mode) in [false, true]
        .into_iter()
        .flat_map(|leave| (0..3).map(move |mode| (leave, mode)))
    {
        let mut g = game();
        let power_life = TotalCost::from_cost(Cost::effect(
            ironsmith::effects::PayLifeEffect::you(Value::SourcePower),
        ));
        let ward_cost = match mode {
            0 => power_life,
            1 => {
                g.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 5);
                TotalCost::from_cost(Cost::dynamic_mana(
                    ironsmith_core::DynamicManaCost::generic_equal_to(Value::SourcePower),
                ))
            }
            _ => TotalCost::one_of(vec![TotalCost::from_cost(Cost::life(40)), power_life]),
        };
        let ward = B::new(CardId::new(), "Ward")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::ward(ward_cost)))
            .build();
        let target = g.create_object_from_definition(&ward, BOB, Zone::Battlefield);
        let spell = g.create_object_from_definition(
            &custom(
                "Targeting Probe",
                vec![CardType::Instant],
                "Tap target creature.",
            ),
            A,
            Zone::Hand,
        );
        let action = compute_legal_actions(&g, A).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
            .unwrap();
        let mut dm = Dm {
            target: Some(target),
            ..Default::default()
        };
        act(&mut g, action, &mut dm);
        assert_eq!(g.stack.len(), 2, "ward must be a real trigger");
        g.add_counters(target, CounterType::PlusOnePlusOne, 3);
        g.refresh_continuous_state();
        if leave {
            g.move_object_by_effect(target, Zone::Graveyard);
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(
            g.player(A).unwrap().life,
            if mode == 1 { 20 } else { 15 },
            "uses five power at resolution or departure"
        );
        if mode == 1 {
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        }
        assert_eq!(g.stack.len(), 1, "ward payment preserves spell");
    }
}
#[test]
fn f18_parsed_teamwork_has_no_crew_substitutions_events_or_history() {
    let mut g = game();
    let spell = custom(
        "Cooperative Probe",
        vec![CardType::Instant],
        "Teamwork 3\nYou gain 1 life.",
    );
    let optional = spell
        .optional_costs
        .iter()
        .find(|o| o.kind == OptionalCostKind::Teamwork)
        .unwrap();
    let crew = optional.cost.costs()[0]
        .effect_ref()
        .unwrap()
        .downcast_ref::<CrewCostEffect>()
        .unwrap();
    assert!(crew.teamwork);
    let helper = B::new(CardId::new(), "Pilot")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(Ability::static_ability(StaticAbility::keyword_marker(
            "This creature crews vehicles as though its power were 2 greater.",
        )))
        .build();
    let id = g.create_object_from_definition(&helper, A, Zone::Battlefield);
    let source = g.create_object_from_definition(&spell, A, Zone::Stack);
    assert!(CostExecutableEffect::can_execute_as_cost(crew, &g, source, A).is_err());
    assert!(
        CostExecutableEffect::can_execute_as_cost(&CrewCostEffect::new(3), &g, source, A).is_ok()
    );
    g.add_counters(id, CounterType::PlusOnePlusOne, 2);
    g.refresh_continuous_state();
    let mut dm = Dm::default();
    let outcome = crew
        .execute(&mut g, &mut EffectContext::new(source, A, &mut dm))
        .unwrap();
    assert!(g.is_tapped(id));
    assert!(
        outcome
            .events
            .iter()
            .all(|e| e.downcast::<KeywordActionEvent>().is_none())
    );
    assert!(g.turn_store.turn_history.crewed_this_turn.is_empty());
}
#[test]
fn f19_unchosen_negative_power_never_reduces_affordability() {
    let mut g = game();
    let good = g.create_object_from_definition(&creature("Good", 2, 2), A, Zone::Battlefield);
    let bad = g.create_object_from_definition(&creature("Negative", -3, 4), A, Zone::Battlefield);
    let source = g.new_object_id();
    for cost in [CrewCostEffect::new(2), CrewCostEffect::teamwork(2)] {
        assert!(CostExecutableEffect::can_execute_as_cost(&cost, &g, source, A).is_ok());
        let mut dm = Dm {
            chosen: Some(good),
            ..Default::default()
        };
        cost.execute(&mut g, &mut EffectContext::new(source, A, &mut dm))
            .unwrap();
        assert!(g.is_tapped(good));
        assert!(!g.is_tapped(bad));
        g.untap(good);
    }
    let cost = SaddleCostEffect::new(2);
    assert!(CostExecutableEffect::can_execute_as_cost(&cost, &g, source, A).is_ok());
    let mut dm = Dm {
        chosen: Some(good),
        ..Default::default()
    };
    cost.execute(&mut g, &mut EffectContext::new(source, A, &mut dm))
        .unwrap();
    assert!(g.is_tapped(good));
    assert!(!g.is_tapped(bad));
}
#[test]
fn f20_waterbend_triggers_once_for_mana_taps_and_mixed_payments() {
    for taps in 0..=2 {
        let mut g = game();
        let observer = custom(
            "Observer",
            vec![CardType::Enchantment],
            "Whenever you waterbend, you gain 3 life.",
        );
        g.create_object_from_definition(&observer, A, Zone::Battlefield);
        let source = g.create_object_from_definition(
            &custom(
                "Bender",
                vec![CardType::Enchantment],
                "Waterbend {2}: You gain 1 life.",
            ),
            A,
            Zone::Battlefield,
        );
        g.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2 - taps);
        for _ in 0..taps {
            let d = B::new(CardId::new(), "Resource")
                .card_types(vec![CardType::Artifact])
                .build();
            g.create_object_from_definition(&d, A, Zone::Battlefield);
        }
        let mut dm = Dm::default();
        activate(&mut g, source, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(
            g.player(A).unwrap().life,
            24,
            "one completion trigger for {taps} taps"
        );
    }
}
#[test]
fn f20_blight_action_and_cost_trigger_after_counter_placement() {
    for as_cost in [false, true] {
        let mut g = game();
        let id =
            g.create_object_from_definition(&creature("Recipient", 4, 4), A, Zone::Battlefield);
        let observer = custom(
            "Observer",
            vec![CardType::Enchantment],
            "Whenever you blight, you gain 3 life.",
        );
        g.create_object_from_definition(&observer, A, Zone::Battlefield);
        let line = if as_cost {
            "Blight 1: You gain 1 life."
        } else {
            "{0}: Blight 1."
        };
        let source = g.create_object_from_definition(
            &custom("Blighter", vec![CardType::Artifact], line),
            A,
            Zone::Battlefield,
        );
        let mut dm = Dm::default();
        activate(&mut g, source, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(id, CounterType::MinusOneMinusOne), 1);
        assert_eq!(g.player(A).unwrap().life, if as_cost { 24 } else { 23 });
    }
}
#[test]
fn f20_blight_completion_survives_prevented_counters_and_no_creatures() {
    for recipient in [false, true] {
        let mut g = game();
        if recipient {
            let d = B::new(CardId::new(), "Protected")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .with_ability(Ability::static_ability(
                    StaticAbility::cant_have_counters_placed(),
                ))
                .build();
            g.create_object_from_definition(&d, A, Zone::Battlefield);
        }
        g.refresh_continuous_state();
        let source = g.new_object_id();
        let effect = PutCountersEffect::new(
            CounterType::MinusOneMinusOne,
            1,
            ChooseSpec::Object(ObjectFilter::creature().you_control()),
        )
        .with_completion_action(KeywordActionKind::Blight);
        let out = effect
            .execute(&mut g, &mut EffectContext::new_default(source, A))
            .unwrap();
        assert_eq!(
            out.events
                .iter()
                .filter(|e| e
                    .downcast::<KeywordActionEvent>()
                    .is_some_and(|e| e.action == KeywordActionKind::Blight))
                .count(),
            1
        );
    }
}
#[test]
fn f22_counter_cost_prohibitions_affect_legal_actions_and_blight_selection() {
    let mut g = game();
    let prohibited = B::new(CardId::new(), "Prohibited")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_ability(Ability::static_ability(
            StaticAbility::cant_have_counters_placed(),
        ))
        .with_ability(Ability::activated(
            TotalCost::from_cost(Cost::effect(PutCountersEffect::on_source(
                CounterType::MinusOneMinusOne,
                1,
            ))),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let bad = g.create_object_from_definition(&prohibited, A, Zone::Battlefield);
    g.refresh_continuous_state();
    assert!(
        !compute_legal_actions(&g, A).expect("fixture has complete replacement state")
            .iter()
            .any(|a| matches!(a,LegalAction::ActivateAbility{source,..}if *source==bad))
    );
    let b = g.create_object_from_definition(
        &custom(
            "Blighter",
            vec![CardType::Artifact],
            "Blight 1: You gain 1 life.",
        ),
        A,
        Zone::Battlefield,
    );
    assert!(
        !compute_legal_actions(&g, A).expect("fixture has complete replacement state")
            .iter()
            .any(|a| matches!(a,LegalAction::ActivateAbility{source,..}if *source==b))
    );
    let good = g.create_object_from_definition(&creature("Eligible", 3, 3), A, Zone::Battlefield);
    let mut dm = Dm {
        chosen: Some(bad),
        ..Default::default()
    };
    activate(&mut g, b, &mut dm);
    settle(&mut g, &mut dm);
    assert_eq!(g.counter_count(good, CounterType::MinusOneMinusOne), 1);
    assert_eq!(g.counter_count(bad, CounterType::MinusOneMinusOne), 0);
}

#[test]
fn f20_declining_blight_or_failing_counter_cost_emits_no_completion() {
    struct Decline;
    impl DecisionMaker for Decline {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            false
        }
    }
    let mut g = game();
    let id = g.create_object_from_definition(&creature("Recipient", 3, 3), A, Zone::Battlefield);
    let blight = PutCountersEffect::new(
        CounterType::MinusOneMinusOne,
        1,
        ChooseSpec::Object(ObjectFilter::creature().you_control()),
    )
    .with_completion_action(KeywordActionKind::Blight);
    let optional = ironsmith::effects::MayEffect::new(vec![Effect::new(blight.clone())]);
    let mut dm = Decline;
    let out = optional
        .execute(&mut g, &mut EffectContext::new(id, A, &mut dm))
        .unwrap();
    assert!(out.events.is_empty());
    assert_eq!(g.counter_count(id, CounterType::MinusOneMinusOne), 0);
    g.effect_store
        .cant_effects
        .cant_have_counter_types_placed
        .insert((id, CounterType::MinusOneMinusOne));
    let mut dm = Dm::default();
    let cost = Cost::effect(blight);
    assert!(
        cost.pay(&mut g, &mut CostContext::new(id, A, &mut dm))
            .is_err()
    );
    assert!(
        g.take_pending_trigger_events()
            .iter()
            .all(|e| e.downcast::<KeywordActionEvent>().is_none())
    );
    let positive = PutCountersEffect::on_source(CounterType::PlusOnePlusOne, 1);
    assert!(CostExecutableEffect::can_execute_as_cost(&positive, &g, id, A).is_ok());
}
#[test]
fn f20_impossible_optional_blight_does_not_prompt_or_emit() {
    struct NoPrompt;
    impl DecisionMaker for NoPrompt {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            panic!("cannot offer impossible blight")
        }
    }
    let mut g = game();
    let source = g.new_object_id();
    let optional = ironsmith::effects::MayEffect::new(vec![Effect::new(
        PutCountersEffect::new(
            CounterType::MinusOneMinusOne,
            1,
            ChooseSpec::Object(ObjectFilter::creature().you_control()),
        )
        .with_completion_action(KeywordActionKind::Blight),
    )]);
    let mut dm = NoPrompt;
    let out = optional
        .execute(&mut g, &mut EffectContext::new(source, A, &mut dm))
        .unwrap();
    assert!(out.events.is_empty());
}

#[test]
fn f20_optional_blight_excludes_prohibited_recipients_when_another_is_legal() {
    let mut g = game();
    let prohibited = B::new(CardId::new(), "Prohibited Recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .with_ability(Ability::static_ability(
            StaticAbility::cant_have_counters_placed(),
        ))
        .build();
    let bad = g.create_object_from_definition(&prohibited, A, Zone::Battlefield);
    let good = g.create_object_from_definition(&creature("Eligible", 4, 4), A, Zone::Battlefield);
    g.create_object_from_definition(
        &custom(
            "Observer",
            vec![CardType::Enchantment],
            "Whenever you blight, you gain 3 life.",
        ),
        A,
        Zone::Battlefield,
    );
    let source = g.create_object_from_definition(
        &custom(
            "Optional Blighter",
            vec![CardType::Artifact],
            "{0}: You may blight 1.",
        ),
        A,
        Zone::Battlefield,
    );
    g.refresh_continuous_state();
    // Attempt the prohibited choice if it is incorrectly offered.
    let mut dm = Dm {
        chosen: Some(bad),
        ..Default::default()
    };
    activate(&mut g, source, &mut dm);
    settle(&mut g, &mut dm);
    assert_eq!(g.counter_count(good, CounterType::MinusOneMinusOne), 1);
    assert_eq!(g.counter_count(bad, CounterType::MinusOneMinusOne), 0);
    assert_eq!(g.player(A).unwrap().life, 23);

    // With only the prohibited creature left, the optional action is unavailable.
    g.move_object_by_effect(good, Zone::Graveyard);
    activate(&mut g, source, &mut dm);
    settle(&mut g, &mut dm);
    assert_eq!(g.counter_count(bad, CounterType::MinusOneMinusOne), 0);
    assert_eq!(g.player(A).unwrap().life, 23);
}

#[test]
fn f20_failed_waterbend_payment_has_no_completion_event() {
    let mut g = game();
    let source = g.create_object_from_definition(
        &custom(
            "Failure Probe",
            vec![CardType::Enchantment],
            "Waterbend {1}: You gain 1 life.",
        ),
        A,
        Zone::Battlefield,
    );
    let helper =
        g.create_object_from_definition(&creature("Payment creature", 2, 2), A, Zone::Battlefield);
    let action = compute_legal_actions(&g, A).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
        .unwrap_or_else(|| panic!("missing ability: {:?}", g.object(source).unwrap().abilities));
    g.phase_out(helper);
    let mut dm = Dm::default();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..8 {
        let Ok(GameProgress::NeedsDecisionCtx(context)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut g, &mut queue, &mut state, &context, &mut dm,
        );
    }
    assert!(g.take_pending_trigger_events().iter().all(|e| {
        !e.downcast::<KeywordActionEvent>()
            .is_some_and(|e| e.action == KeywordActionKind::Waterbend)
    }));
    assert!(g.stack.is_empty());
}

#[test]
fn f18_f20_compiled_keyword_surfaces_follow_typed_costs() {
    for (text, expected) in [
        ("Teamwork 3\nYou gain 1 life.", "Teamwork 3"),
        ("Waterbend {2}: You gain 1 life.", "Waterbend {2}"),
        ("Blight 1: You gain 1 life.", "Blight 1"),
        ("Ward—Blight 1", "Ward—Blight 1"),
        (
            "As an additional cost to cast this spell, you may blight 1.\nYou gain 1 life.",
            "As an additional cost to cast this spell, you may blight 1.",
        ),
    ] {
        let def = custom("Compiled Keyword Probe", vec![CardType::Sorcery], text);
        let output = ironsmith::compiled_text::compiled_text_lines(&def).join("\n");
        assert!(output.contains(expected), "{text}: {output}");
    }
}
