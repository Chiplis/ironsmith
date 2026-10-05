//! Six exact full bodies, with completed physical/grouped die receipt evidence.
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
        "../../../fixtures/dice_receipt_triggers.json.fixture"
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
fn roll(
    g: &mut GameState,
    source: ObjectId,
    p: PlayerId,
    sides: u32,
    result: u32,
    dm: &mut Choices,
) -> usize {
    g.force_next_die_roll(result);
    let events = RollDieEffect::new(PlayerFilter::Specific(p), sides)
        .execute(g, &mut ExecutionContext::new(source, B, dm))
        .unwrap()
        .events;
    stack(g, events, dm)
}
fn batch(
    g: &mut GameState,
    source: ObjectId,
    p: PlayerId,
    results: &[u32],
    dm: &mut Choices,
) -> usize {
    for &result in results {
        g.force_next_die_roll(result);
    }
    let events =
        RollDiceChooseResultEffect::new(PlayerFilter::Specific(p), results.len() as u32, 20)
            .execute(g, &mut ExecutionContext::new(source, B, dm))
            .unwrap()
            .events;
    stack(g, events, dm)
}
fn count(g: &GameState, id: ObjectId) -> u32 {
    g.counter_count(id, ironsmith::CounterType::PlusOnePlusOne)
}
fn keyword(g: &GameState, id: ObjectId, key: ironsmith::static_abilities::StaticAbilityId) -> bool {
    g.current_has_static_ability_id(id, key)
}
#[test]
fn six_exact_cards_have_complete_direct_and_artifact_programs() {
    assert_eq!(fixtures().len(), 6);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}
#[test]
fn atomwheel_paid_activation_and_per_die_results_keep_the_observer_as_counter_subject() {
    for d in definitions("Atomwheel Acrobats") {
        for result in [1, 2, 3] {
            let mut g = game();
            let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
            let index = d
                .abilities
                .iter()
                .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
                .unwrap();
            let before = g.player(A).unwrap().mana_pool.total();
            g.force_next_die_roll(result);
            let mut dm = Choices::default();
            action(
                &mut g,
                A,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: index,
                },
                &mut dm,
            );
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), before - 3);
            assert_eq!(count(&g, source), if result <= 2 { result } else { 0 });
            let roller = object(
                &mut g,
                B,
                Zone::Battlefield,
                "Other roll source",
                "Type: Artifact",
            );
            batch(&mut g, roller, A, &[1, 2], &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(count(&g, source), if result <= 2 { result + 3 } else { 3 });
            assert_eq!(count(&g, roller), 0);
            roll(&mut g, roller, B, 6, 2, &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(count(&g, source), if result <= 2 { result + 3 } else { 3 });
        }
    }
}
#[test]
fn critical_hit_cast_body_and_natural_graveyard_return_use_exact_incarnation() {
    for d in definitions("Critical Hit") {
        let mut g = game();
        let card = g.create_object_from_definition(&d, A, Zone::Hand);
        let creature = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Strike target",
            "Type: Creature — Beast\nPower/Toughness: 2/2",
        );
        let mut dm = Choices {
            target: Some(creature),
            ..Default::default()
        };
        action(
            &mut g,
            A,
            LegalAction::CastSpell {
                spell_id: card,
                from_zone: Zone::Hand,
                casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
            },
            &mut dm,
        );
        settle(&mut g, &mut dm);
        assert!(keyword(
            &g,
            creature,
            ironsmith::static_abilities::StaticAbilityId::DoubleStrike
        ));
        let grave = *g.player(A).unwrap().graveyard.last().unwrap();
        assert_eq!(
            roll(&mut g, creature, A, 6, 6, &mut dm),
            0,
            "highest d6 result is not a natural20"
        );
        assert_eq!(roll(&mut g, creature, B, 20, 20, &mut dm), 0);
        assert_eq!(roll(&mut g, creature, A, 20, 20, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert!(g.object(grave).is_none());
        assert!(
            g.player(A)
                .unwrap()
                .hand
                .iter()
                .any(|id| g.object(*id).unwrap().name == "Critical Hit")
        );
        let mut g = game();
        let card = g.create_object_from_definition(&d, A, Zone::Graveyard);
        let roller = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Roll source",
            "Type: Artifact",
        );
        assert_eq!(roll(&mut g, roller, A, 20, 20, &mut dm), 1);
        let exile = g.move_object_by_effect(card, Zone::Exile).unwrap();
        let returned = g.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        settle(&mut g, &mut dm);
        assert_eq!(
            g.object(returned).unwrap().zone,
            Zone::Graveyard,
            "a later incarnation is not the triggered source"
        );
    }
}
#[test]
fn critical_hit_natural_result_does_not_follow_numerical_adjustment() {
    for d in definitions("Critical Hit") {
        for natural in [19, 20] {
            let mut g = game();
            g.create_object_from_definition(&d, A, Zone::Graveyard);
            let modifier = object(
                &mut g,
                A,
                Zone::Battlefield,
                "Roll modifier",
                "Type: Enchantment\nAfter you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.",
            );
            let mut dm = Choices {
                accept: true,
                ..Default::default()
            };
            assert_eq!(
                roll(&mut g, modifier, A, 20, natural, &mut dm),
                if natural == 20 { 1 } else { 0 }
            );
            assert_eq!(g.player(A).unwrap().life, 29);
            settle(&mut g, &mut dm);
            assert_eq!(
                g.player(A).unwrap().hand.len(),
                if natural == 20 { 1 } else { 0 }
            );
        }
    }
}
#[test]
fn third_die_uses_completed_turn_ordinal_through_batch_and_native_restore() {
    for d in definitions("Resolute Veggiesaur") {
        let mut g = game();
        let roller = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Roll source",
            "Type: Artifact",
        );
        let mut dm = Choices::default();
        batch(&mut g, roller, A, &[1, 2], &mut dm);
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let saved = g.clone();
        for _ in 0..2 {
            g = saved.clone();
            assert_eq!(batch(&mut g, roller, A, &[3, 4], &mut dm), 1);
            roll(&mut g, roller, A, 6, 5, &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(count(&g, source), 1);
        }
        g.turn_store.turn_history.clear_for_new_turn();
        g.turn.turn_number += 1;
        batch(&mut g, roller, A, &[2, 3, 4], &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(count(&g, source), 2);
    }
}
#[test]
fn monoxa_threshold_tail_reads_its_roll_and_modifies_only_source_until_cleanup() {
    for d in definitions("Monoxa, Midway Manager") {
        for result in [2, 3, 4, 5] {
            let mut g = game();
            let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
            let roller = object(
                &mut g,
                B,
                Zone::Battlefield,
                "Roll source",
                "Type: Creature — Beast\nPower/Toughness: 2/2",
            );
            let mut dm = Choices::default();
            roll(&mut g, roller, A, 6, result, &mut dm);
            settle(&mut g, &mut dm);
            for (key, threshold) in [
                (ironsmith::static_abilities::StaticAbilityId::FirstStrike, 3),
                (ironsmith::static_abilities::StaticAbilityId::Menace, 4),
                (ironsmith::static_abilities::StaticAbilityId::Lifelink, 5),
            ] {
                assert_eq!(keyword(&g, source, key), result >= threshold);
                assert!(!keyword(&g, roller, key));
            }
            ironsmith::turn::execute_cleanup_step(&mut g);
            assert!(!keyword(
                &g,
                source,
                ironsmith::static_abilities::StaticAbilityId::FirstStrike
            ));
            let index = d
                .abilities
                .iter()
                .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
                .unwrap();
            g.force_next_die_roll(5);
            g.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 6);
            action(
                &mut g,
                A,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: index,
                },
                &mut dm,
            );
            settle(&mut g, &mut dm);
            assert!(keyword(
                &g,
                source,
                ironsmith::static_abilities::StaticAbilityId::Lifelink
            ));
        }
    }
}
#[test]
fn grouped_results_preserve_any_and_sum_without_reading_a_later_roll() {
    for d in definitions("Farideh, Devil's Chosen") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let roller = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Roll source",
            "Type: Artifact",
        );
        for _ in 0..4 {
            object(&mut g, A, Zone::Library, "Draw card", "Type: Land");
        }
        let mut dm = Choices::default();
        assert_eq!(batch(&mut g, roller, A, &[2, 12], &mut dm), 1);
        assert_eq!(roll(&mut g, roller, A, 6, 1, &mut dm), 2);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 1);
        assert!(keyword(
            &g,
            source,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        assert!(keyword(
            &g,
            source,
            ironsmith::static_abilities::StaticAbilityId::Menace
        ));
    }
    for d in definitions("Vexing Puzzlebox") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let roller = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Roll source",
            "Type: Artifact",
        );
        let mut dm = Choices::default();
        assert_eq!(batch(&mut g, roller, A, &[2, 12], &mut dm), 1);
        roll(&mut g, roller, A, 6, 1, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Charge), 15);
        roll(&mut g, roller, B, 20, 20, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Charge), 15);
    }
}
#[test]
fn puzzlebox_mana_roll_and_hundred_counter_search_are_real_paid_abilities() {
    for d in definitions("Vexing Puzzlebox") {
        let mut g = game();
        let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let mut dm = Choices::default();
        let mana=d.abilities.iter().position(|a|matches!(&a.kind,ironsmith::ability::AbilityKind::Activated(ability) if ability.is_mana_ability())).unwrap();
        let search=d.abilities.iter().position(|a|matches!(&a.kind,ironsmith::ability::AbilityKind::Activated(ability) if !ability.is_mana_ability())).unwrap();
        let before = g.player(A).unwrap().mana_pool.total();
        g.force_next_die_roll(20);
        action(
            &mut g,
            A,
            LegalAction::ActivateManaAbility {
                source,
                ability_index: mana,
            },
            &mut dm,
        );
        assert_eq!(g.player(A).unwrap().mana_pool.total(), before + 1);
        assert_eq!(g.stack.len(), 1, "only the roll observer uses the stack");
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Charge), 20);
        g.add_counters(source, ironsmith::CounterType::Charge, 80);
        g.untap(source);
        object(
            &mut g,
            A,
            Zone::Library,
            "Search artifact",
            "Type: Artifact",
        );
        action(
            &mut g,
            A,
            LegalAction::ActivateAbility {
                source,
                ability_index: search,
            },
            &mut dm,
        );
        assert_eq!(g.counter_count(source, ironsmith::CounterType::Charge), 0);
        assert!(g.is_tapped(source));
        settle(&mut g, &mut dm);
        assert!(
            g.battlefield
                .iter()
                .any(|id| g.object(*id).unwrap().name == "Search artifact")
        );
    }
}

#[test]
fn actual_planar_roll_counts_for_ordinal_but_has_no_numeric_batch_result() {
    for route in 0..2 {
        let mut g = game();
        let make_deck = || {
            (0..10)
                .map(|index| {
                    (
                        compile_to_runtime_definition(
                            &format!("Numeric-free plane {index}"),
                            "Type: Plane",
                            false,
                        )
                        .unwrap(),
                        ironsmith::PlanarCardKind::Plane,
                    )
                })
                .collect::<Vec<_>>()
        };
        g.enable_planechase(vec![(A, make_deck()), (B, make_deck())])
            .unwrap();
        let dinosaur = g.create_object_from_definition(
            &definitions("Resolute Veggiesaur")[route],
            A,
            Zone::Battlefield,
        );
        let puzzle = g.create_object_from_definition(
            &definitions("Vexing Puzzlebox")[route],
            A,
            Zone::Battlefield,
        );
        let farideh = g.create_object_from_definition(
            &definitions("Farideh, Devil's Chosen")[route],
            A,
            Zone::Battlefield,
        );
        object(&mut g, A, Zone::Library, "Draw card", "Type: Land");
        let roller = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Roll source",
            "Type: Artifact",
        );
        let mut dm = Choices::default();
        roll(&mut g, roller, A, 6, 1, &mut dm);
        settle(&mut g, &mut dm);
        g.force_next_die_roll(6);
        g.roll_planar_die(A, false).unwrap();
        stack(&mut g, Vec::new(), &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.counter_count(puzzle, ironsmith::CounterType::Charge), 1);
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 2);
        assert!(g.player(A).unwrap().hand.is_empty());
        assert!(keyword(
            &g,
            farideh,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        roll(&mut g, roller, A, 6, 2, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(count(&g, dinosaur), 1);
        assert_eq!(g.counter_count(puzzle, ironsmith::CounterType::Charge), 3);
    }
}

#[test]
fn paid_reroll_discards_old_result_and_counts_only_one_completed_die() {
    for route in 0..2 {
        let mut g = game();
        let roller = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Roll source",
            "Type: Artifact",
        );
        let mut dm = Choices::default();
        batch(&mut g, roller, A, &[1, 1], &mut dm);
        let dinosaur = g.create_object_from_definition(
            &definitions("Resolute Veggiesaur")[route],
            A,
            Zone::Battlefield,
        );
        let acrobat = g.create_object_from_definition(
            &definitions("Atomwheel Acrobats")[route],
            A,
            Zone::Battlefield,
        );
        let mut modifier = CardDefinition::new(
            ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Reroll permission")
                .card_types(vec![ironsmith::CardType::Enchantment])
                .build(),
        );
        modifier.abilities.push(ironsmith::Ability::static_ability(
            ironsmith::StaticAbility::die_roll_reroll(
                PlayerFilter::You,
                ironsmith::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(1)]),
                true,
                "You may pay one mana to reroll.",
            ),
        ));
        let modifier = g.create_object_from_definition(&modifier, A, Zone::Battlefield);
        g.force_next_die_roll(2);
        g.force_next_die_roll(5);
        dm.accept = true;
        let before = g.player(A).unwrap().mana_pool.total();
        let events = RollDieEffect::new(PlayerFilter::You, 6)
            .execute(&mut g, &mut ExecutionContext::new(modifier, A, &mut dm))
            .unwrap()
            .events;
        stack(&mut g, events, &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), before - 1);
        assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 3);
        assert_eq!(
            g.turn_store.turn_history.die_rolls_this_turn[&A],
            vec![1, 1, 5]
        );
        assert_eq!(count(&g, dinosaur), 1);
        assert_eq!(count(&g, acrobat), 0);
    }
}

fn program_definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
#[test]
fn local_roll_predicate_uses_the_exact_completed_roll_after_an_unrelated_action() {
    for definition in program_definitions(
        "Local roll comparison",
        "Mana cost: {0}\nType: Sorcery\nRoll a six-sided die. You gain 1 life. If the roll was 4 or higher, draw a card.",
    ) {
        for result in [1, 4, 6] {
            let mut g = game();
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            object(&mut g, A, Zone::Library, "Drawn card", "Type: Land");
            let life = g.player(A).unwrap().life;
            let mut dm = Choices::default();
            g.force_next_die_roll(result);
            action(
                &mut g,
                A,
                LegalAction::CastSpell {
                    spell_id: spell,
                    from_zone: Zone::Hand,
                    casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
                },
                &mut dm,
            );
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().life, life + 1);
            assert_eq!(g.player(A).unwrap().hand.len(), usize::from(result >= 4));
        }
    }
}
#[test]
fn a_triggered_program_local_roll_overrides_the_original_triggering_die_result() {
    for (original, local, draws) in [(1, 6, 1), (5, 1, 0)] {
        let text = format!(
            "Type: Artifact\nWhenever you roll a {original}, roll a six-sided die. If the roll was 4 or higher, draw a card."
        );
        for definition in program_definitions("Local triggered roll", &text) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            object(&mut g, A, Zone::Library, "Drawn card", "Type: Land");
            let mut dm = Choices::default();
            assert_eq!(roll(&mut g, source, A, 6, original, &mut dm), 1);
            g.force_next_die_roll(local);
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().hand.len(), draws);
            assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 2);
        }
    }
}
#[test]
fn unsupported_die_predicate_contexts_fail_closed_instead_of_inventing_event_values() {
    for text in [
        "Type: Sorcery\nIf the roll was 4 or higher, draw a card.",
        "Type: Sorcery\nIf any of those results was 10 or higher, draw a card.",
        "Type: Artifact\nWhenever you gain life, if any of those results was 10 or higher, draw a card.",
        "Type: Artifact\nWhenever you roll one or more dice, if the roll was 4 or higher, draw a card.",
        "Type: Artifact\nWhenever you roll one or more dice, roll a six-sided die. If any of those results was 10 or higher, draw a card.",
    ] {
        assert!(
            compile_to_artifact("Unbound roll predicate", text, false).is_err(),
            "{text}"
        );
    }
}
