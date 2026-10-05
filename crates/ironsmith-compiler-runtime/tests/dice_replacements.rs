//! Exact extra-die replacements and their real class/trigger secondary bodies.
//! Source-authored scenarios, unrun during the implementation-first campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::{
    EffectContext as ExecutionContext, EffectExecutor, RollDiceChooseResultEffect, RollDieEffect,
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
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/dice_replacements.json.fixture"
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
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 30);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for p in [A, B] {
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
}
impl DecisionMaker for Choices {
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

fn completed_results(outcome: &ironsmith::effect::EffectOutcome) -> Vec<(u32, u32)> {
    outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<ironsmith::events::other::DieRolledEvent>())
        .map(|event| (event.natural_result, event.result))
        .collect()
}
fn force(g: &mut GameState, results: &[u32]) {
    for &result in results {
        g.force_next_die_roll(result);
    }
}
fn single(
    g: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    dm: &mut impl DecisionMaker,
) -> ironsmith::effect::EffectOutcome {
    RollDieEffect::new(PlayerFilter::Specific(player), 6)
        .execute(g, &mut ExecutionContext::new(source, B, dm))
        .unwrap()
}
fn counters(g: &GameState, id: ObjectId) -> u32 {
    g.counter_count(id, ironsmith::CounterType::PlusOnePlusOne)
}
#[test]
fn three_exact_replacement_cards_keep_full_direct_and_artifact_programs() {
    assert_eq!(fixtures().len(), 3);
    for row in fixtures() {
        for d in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
            assert!(d.abilities.iter().any(|ability| matches!(&ability.kind,
                ironsmith::ability::AbilityKind::Static(ability) if ability.id() == ironsmith::static_abilities::StaticAbilityId::ExtraDieIgnoreLowest)));
        }
    }
}
#[test]
fn pixie_uses_the_actual_roller_and_current_controller_and_does_not_exist_while_phased() {
    for d in definitions("Pixie Guide") {
        let mut g = game();
        let pixie = g.create_object_from_definition(&d, A, Zone::Battlefield);
        assert!(g.current_has_static_ability_id(
            pixie,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        let producer = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Foreign producer",
            "Type: Artifact",
        );
        let mut dm = Choices::default();
        force(&mut g, &[1, 6]);
        let out = single(&mut g, producer, A, &mut dm);
        assert_eq!(completed_results(&out), [(6, 6)]);
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 1);
        force(&mut g, &[2, 5]);
        let out = single(&mut g, producer, B, &mut dm);
        assert_eq!(completed_results(&out), [(2, 2)]);
        assert_eq!(
            g.take_forced_die_roll(),
            Some(5),
            "opponent consumes only the authored die"
        );
        g.phase_out(pixie);
        force(&mut g, &[1, 4]);
        let out = single(&mut g, producer, A, &mut dm);
        assert_eq!(completed_results(&out), [(1, 1)]);
        assert_eq!(g.take_forced_die_roll(), Some(4));
        g.phase_in(pixie);
        ironsmith::effects::GainControlEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(pixie),
            ironsmith::effect::Until::EndOfTurn,
        )
        .execute(&mut g, &mut ExecutionContext::new_default(producer, B))
        .unwrap();
        force(&mut g, &[1, 6]);
        assert_eq!(
            completed_results(&single(&mut g, producer, B, &mut dm)),
            [(6, 6)]
        );
        force(&mut g, &[2, 5]);
        assert_eq!(
            completed_results(&single(&mut g, producer, A, &mut dm)),
            [(2, 2)]
        );
        assert_eq!(g.take_forced_die_roll(), Some(5));
    }
}
#[test]
fn distinct_replacements_apply_once_and_ignored_dice_do_not_reach_grouped_or_per_die_observers() {
    for (pixie, wyll) in definitions("Pixie Guide")
        .into_iter()
        .zip(definitions("Wyll, Blade of Frontiers"))
    {
        let mut g = game();
        for _ in 0..2 {
            g.create_object_from_definition(&pixie, A, Zone::Battlefield);
        }
        let wyll = g.create_object_from_definition(&wyll, A, Zone::Battlefield);
        let ignored_observer = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Ignored low observer",
            "Type: Creature — Elf\nPower/Toughness: 1/1\nWhenever you roll a 1 or 2, put that many +1/+1 counters on this creature.",
        );
        let mut dm = Choices::default();
        force(&mut g, &[1, 2, 3, 5, 6]);
        let out = RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 6)
            .execute(&mut g, &mut ExecutionContext::new(wyll, A, &mut dm))
            .unwrap();
        assert_eq!(
            out.count_or_zero(),
            5,
            "the explicit choice uses one of the two retained results"
        );
        assert_eq!(completed_results(&out), [(5, 5), (6, 6)]);
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 2);
        assert_eq!(g.turn_store.turn_history.die_rolls_this_turn[&A], [5, 6]);
        assert_eq!(
            stack(&mut g, out.events, &mut dm),
            1,
            "Wyll is once per completed batch; ignored low rolls cause no triggers"
        );
        settle(&mut g, &mut dm);
        assert_eq!(counters(&g, wyll), 1);
        assert_eq!(counters(&g, ignored_observer), 0);
    }
}
struct RollChoices {
    pause: bool,
    pending: bool,
    calls: usize,
    contexts: Vec<SelectOptionsContext>,
}
impl DecisionMaker for RollChoices {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_options(&mut self, _: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        assert_eq!(context.player, A);
        self.calls += 1;
        self.pending = self.pause;
        self.contexts.push(context.clone());
        vec![context.options.last().unwrap().index]
    }
}
#[test]
fn replacement_order_and_tied_lowest_choices_suspend_atomically_and_replay_same_attempt() {
    for d in definitions("Pixie Guide") {
        for replacement_count in [1, 2] {
            let mut g = game();
            let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
            if replacement_count == 2 {
                g.create_object_from_definition(&d, A, Zone::Battlefield);
            }
            force(&mut g, &vec![3; replacement_count + 1]);
            let saved = g.clone();
            let mut dm = RollChoices {
                pause: true,
                pending: false,
                calls: 0,
                contexts: vec![],
            };
            let out = single(&mut g, source, A, &mut dm);
            assert!(out.events.is_empty());
            assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 0);
            assert!(g.take_pending_trigger_events().is_empty());
            dm.pause = false;
            dm.pending = false;
            let out = single(&mut g, source, A, &mut dm);
            assert_eq!(completed_results(&out), [(3, 3)]);
            assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 1);
            assert!(dm.contexts.iter().any(|context| {
                context
                    .options
                    .iter()
                    .any(|option| option.description.starts_with("Ignore die"))
            }));
            if replacement_count == 2 {
                // Display text may be redacted at the decision boundary;
                // both replacement occurrences still need independent choices.
                assert_eq!(
                    dm.contexts[0]
                        .options
                        .iter()
                        .filter(|option| option.legal)
                        .map(|option| option.index)
                        .collect::<Vec<_>>(),
                    vec![0, 1]
                );
            }
            // Native restore also retains the same original attempt inputs.
            g = saved;
            let replay = single(&mut g, source, A, &mut dm);
            assert_eq!(completed_results(&replay), [(3, 3)]);
        }
    }
}
fn class_level_index(g: &GameState, source: ObjectId, level: u32) -> usize {
    g.current_abilities(source)
        .unwrap()
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability)
                if ability
                    .additional_restrictions
                    .iter()
                    .any(|text| text == &format!("__ironsmith_class_level:{level}")) =>
            {
                Some(index)
            }
            _ => None,
        })
        .unwrap()
}
#[test]
fn barbarian_paid_class_levels_gate_the_grouped_pump_and_haste_bodies() {
    for d in definitions("Barbarian Class") {
        let mut g = game();
        let class = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let creature = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Your creature",
            "Type: Creature — Beast\nPower/Toughness: 2/2",
        );
        let other = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Other creature",
            "Type: Creature — Beast\nPower/Toughness: 2/2",
        );
        let mut dm = Choices {
            target: Some(creature),
            ..Default::default()
        };
        force(&mut g, &[1, 4]);
        let out = single(&mut g, class, A, &mut dm);
        assert_eq!(stack(&mut g, out.events, &mut dm), 0);
        let index = class_level_index(&g, class, 2);
        let mana = g.player(A).unwrap().mana_pool.total();
        action(
            &mut g,
            A,
            LegalAction::ActivateAbility {
                source: class,
                ability_index: index,
            },
            &mut dm,
        );
        settle(&mut g, &mut dm);
        assert_eq!(g.class_level(class), 2);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 2);
        force(&mut g, &[1, 4, 6]);
        let out = RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 6)
            .execute(&mut g, &mut ExecutionContext::new(class, A, &mut dm))
            .unwrap();
        assert_eq!(completed_results(&out), [(4, 4), (6, 6)]);
        assert_eq!(stack(&mut g, out.events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.current_power(creature), Some(4));
        assert_eq!(g.current_power(other), Some(2));
        assert!(g.current_has_static_ability_id(
            creature,
            ironsmith::static_abilities::StaticAbilityId::Menace
        ));
        let index = class_level_index(&g, class, 3);
        let mana = g.player(A).unwrap().mana_pool.total();
        action(
            &mut g,
            A,
            LegalAction::ActivateAbility {
                source: class,
                ability_index: index,
            },
            &mut dm,
        );
        settle(&mut g, &mut dm);
        assert_eq!(g.class_level(class), 3);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 3);
        assert!(g.current_has_static_ability_id(
            creature,
            ironsmith::static_abilities::StaticAbilityId::Haste
        ));
        assert!(!g.current_has_static_ability_id(
            other,
            ironsmith::static_abilities::StaticAbilityId::Haste
        ));
        ironsmith::turn::execute_cleanup_step(&mut g);
        assert_eq!(g.current_power(creature), Some(2));
        assert!(!g.current_has_static_ability_id(
            creature,
            ironsmith::static_abilities::StaticAbilityId::Menace
        ));
        assert!(g.current_has_static_ability_id(
            creature,
            ironsmith::static_abilities::StaticAbilityId::Haste
        ));
    }
}
#[test]
fn ignored_rolls_receive_no_result_modifier_and_keep_natural_result_separate() {
    for d in definitions("Pixie Guide") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        object(
            &mut g,
            A,
            Zone::Battlefield,
            "Numeric modifier",
            "Type: Artifact\nAfter you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.",
        );
        let life = g.player(A).unwrap().life;
        let mut dm = Choices {
            accept: true,
            ..Default::default()
        };
        force(&mut g, &[1, 5]);
        let out = single(&mut g, source, A, &mut dm);
        assert_eq!(completed_results(&out), [(5, 6)]);
        assert_eq!(g.player(A).unwrap().life, life - 1);
        assert_eq!(g.turn_store.turn_history.die_rolls_this_turn[&A], [6]);
    }
}

#[test]
fn replacement_history_is_not_reapplied_when_a_modifier_rerolls_the_same_original_die() {
    for d in definitions("Pixie Guide") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        object(
            &mut g,
            A,
            Zone::Battlefield,
            "Reroll modifier",
            "Type: Artifact\nOnce each turn, you may pay {1} to reroll one or more dice you rolled.",
        );
        let mut dm = Choices {
            accept: true,
            ..Default::default()
        };
        let mana = g.player(A).unwrap().mana_pool.total();
        force(&mut g, &[1, 5, 2, 6]);
        let out = single(&mut g, source, A, &mut dm);
        assert_eq!(completed_results(&out), [(2, 2)]);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 1);
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 1);
        assert_eq!(
            g.take_forced_die_roll(),
            Some(6),
            "reroll changes this same event's result rather than replacing it again"
        );
    }
}
#[test]
fn replacement_count_overflow_is_a_typed_atomic_resource_error_before_randomness() {
    let mut g = game();
    let d = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Large replacement",
    )
    .card_types(vec![ironsmith::CardType::Artifact])
    .with_ability(ironsmith::ability::Ability::static_ability(
        ironsmith::static_abilities::StaticAbility::from_model(
            ironsmith::static_abilities::CompiledStaticAbility::extra_die_ignore_lowest(
                PlayerFilter::You,
                u32::MAX,
                "large replacement",
            ),
        ),
    ))
    .build();
    let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
    force(&mut g, &[6]);
    let before = g.irreversible_random_count();
    let error = RollDieEffect::new(PlayerFilter::You, 6)
        .execute(&mut g, &mut ExecutionContext::new_default(source, A))
        .unwrap_err();
    assert!(matches!(
        error,
        ironsmith::effects::ExecutionError::ResourceLimitExceeded {
            resource: "physical replacement dice",
            ..
        }
    ));
    assert!(error.is_incomplete_execution());
    assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 0);
    assert_eq!(g.irreversible_random_count(), before);
    assert_eq!(g.take_forced_die_roll(), Some(6));
}
#[test]
fn real_attraction_rolls_use_the_retained_die_and_planar_rolls_ignore_numeric_replacements() {
    for pixie in definitions("Pixie Guide") {
        let mut g = game();
        let source = g.create_object_from_definition(&pixie, A, Zone::Battlefield);
        let attraction = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::CardId::new(),
            "Visit on six",
        )
        .card_types(vec![ironsmith::CardType::Artifact])
        .subtypes(vec![ironsmith::types::Subtype::Attraction])
        .attraction_lights(vec![6])
        .with_spell_effect(vec![ironsmith::Effect::gain_life(1)])
        .build();
        g.enable_attractions(vec![(
            A,
            ironsmith::game_state::AttractionDeckFormat::Limited,
            vec![attraction.clone(), attraction.clone(), attraction],
        )])
        .unwrap();
        ironsmith::effects::OpenAttractionEffect::new()
            .execute(&mut g, &mut ExecutionContext::new_default(source, A))
            .unwrap();
        let life = g.player(A).unwrap().life;
        force(&mut g, &[1, 6]);
        let mut q = TriggerQueue::new();
        let mut dm = Choices::default();
        assert_eq!(
            ironsmith::game_loop::roll_to_visit_attractions_with_dm(&mut g, &mut q, &mut dm)
                .unwrap(),
            Some(6)
        );
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, life + 1);
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 1);
        assert_eq!(g.turn_store.turn_history.die_rolls_this_turn[&A], [6]);

        let deck = || {
            (0..10)
                .map(|i| {
                    (
                        ironsmith::cards::builders::CardDefinitionBuilder::new(
                            ironsmith::CardId::new(),
                            format!("Plane {i}"),
                        )
                        .card_types(vec![ironsmith::CardType::Plane])
                        .build(),
                        ironsmith::PlanarCardKind::Plane,
                    )
                })
                .collect()
        };
        g.enable_planechase(vec![(A, deck()), (B, deck())]).unwrap();
        g.reveal_starting_plane().unwrap();
        force(&mut g, &[2, 6]);
        assert_eq!(
            g.roll_planar_die(A, true).unwrap(),
            ironsmith::PlanarDieFace::Chaos
        );
        assert_eq!(
            g.take_forced_die_roll(),
            Some(6),
            "a planar face has no numerical lowest result"
        );
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 2);
        assert_eq!(g.turn_store.turn_history.die_rolls_this_turn[&A], [6]);
    }
}

#[test]
fn duplicate_ability_occurrences_on_one_source_are_not_coalesced_by_shared_model_identity() {
    let mut g = game();
    let ability = ironsmith::ability::Ability::static_ability(
        ironsmith::static_abilities::StaticAbility::from_model(
            ironsmith::static_abilities::CompiledStaticAbility::extra_die_ignore_lowest(
                PlayerFilter::You,
                1,
                "extra numerical die",
            ),
        ),
    );
    let d = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Two static occurrences",
    )
    .card_types(vec![ironsmith::CardType::Artifact])
    .with_ability(ability.clone())
    .with_ability(ability)
    .build();
    let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
    let mut dm = Choices::default();
    force(&mut g, &[1, 3, 6]);
    assert_eq!(
        completed_results(&single(&mut g, source, A, &mut dm)),
        [(6, 6)]
    );
    assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 1);
}
#[test]
fn wyll_keeps_the_existing_background_commander_pairing_descriptor() {
    for d in definitions("Wyll, Blade of Frontiers") {
        assert!(d.abilities.iter().any(|ability|matches!(&ability.kind,
            ironsmith::ability::AbilityKind::Static(ability) if ability.id() == ironsmith::static_abilities::StaticAbilityId::Partner
                && ability.display() == "Choose a Background")));
    }
}
