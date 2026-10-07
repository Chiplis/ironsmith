//! Passive damage receipts: per-recipient totals versus a single source.
//! All scenarios are authored source proposals and remain unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::effects::{
    DealDamageBySourcesEffect, DealDamageEffect, EffectContext as ExecutionContext, EffectExecutor,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/passive_damage_recipients.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 30);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for p in [A, B, C] {
        for m in [
            ironsmith::mana::ManaSymbol::White,
            ironsmith::mana::ManaSymbol::Blue,
            ironsmith::mana::ManaSymbol::Black,
            ironsmith::mana::ManaSymbol::Red,
            ironsmith::mana::ManaSymbol::Green,
            ironsmith::mana::ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(m, 20);
        }
    }
    g
}
fn object(g: &mut GameState, p: PlayerId, z: Zone, name: &str, text: &str) -> ObjectId {
    let d = compile_to_runtime_definition(name, text, false).unwrap();
    g.create_object_from_definition(&d, p, z)
}
fn creature(g: &mut GameState, p: PlayerId, power: u32) -> ObjectId {
    object(
        g,
        p,
        Zone::Battlefield,
        "Damage creature",
        &format!("Type: Creature — Beast\nPower/Toughness: {power}/8"),
    )
}
fn resources(g: &mut GameState, p: PlayerId, z: Zone, n: usize) -> Vec<ObjectId> {
    (0..n)
        .map(|_| object(g, p, z, "Resource", "Type: Land"))
        .collect()
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    objects: Vec<ObjectId>,
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
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let ids = self
            .objects
            .iter()
            .copied()
            .filter(|id| {
                c.candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            })
            .take(c.min)
            .collect::<Vec<_>>();
        if ids.len() >= c.min {
            ids
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        }
    }
}
fn action(g: &mut GameState, p: PlayerId, a: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(p);
    let mut state = PriorityLoopState::new(g.players.len());
    let mut q = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(a),
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
fn stack(g: &mut GameState, events: Vec<TriggerEvent>, dm: &mut Choices) -> usize {
    for e in events {
        g.queue_trigger_event(Default::default(), e);
    }
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
    g.stack.len()
}
fn resolve(g: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(g, dm).unwrap();
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
}
fn settle(g: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        if g.stack.is_empty() {
            return;
        }
        resolve(g, dm);
    }
    panic!("unsettled stack");
}
fn damage(
    g: &mut GameState,
    source: ObjectId,
    target: ChooseSpec,
    amount: i32,
    combat: bool,
) -> Vec<TriggerEvent> {
    let controller = g.current_controller(source).unwrap_or(B);
    DealDamageEffect::new(amount, target)
        .with_combat(combat)
        .execute(g, &mut ExecutionContext::new_default(source, controller))
        .unwrap()
        .events
}
fn simultaneous(
    g: &mut GameState,
    observer: ObjectId,
    source_controller: PlayerId,
    target: ChooseSpec,
) -> Vec<TriggerEvent> {
    let mut filter = ObjectFilter::creature();
    filter.controller = Some(PlayerFilter::Specific(source_controller));
    DealDamageBySourcesEffect::new(
        vec![ChooseSpec::All(filter)],
        ironsmith::effect::Value::PowerOf(Box::new(ChooseSpec::Source)),
        target,
    )
    .execute(g, &mut ExecutionContext::new_default(observer, A))
    .unwrap()
    .events
}
fn tokens(g: &GameState, subtype: ironsmith::types::Subtype) -> Vec<ObjectId> {
    g.battlefield
        .iter()
        .copied()
        .filter(|id| {
            g.object(*id).is_some_and(|o| {
                o.kind == ironsmith::object::ObjectKind::Token
                    && g.calculated_subtypes(*id).contains(&subtype)
            })
        })
        .collect()
}
fn combat(
    g: &mut GameState,
    attackers: Vec<(ObjectId, ironsmith::combat_state::AttackTarget)>,
    blockers: Vec<(ObjectId, Vec<ObjectId>)>,
    dm: &mut Choices,
) -> usize {
    let mut combat = ironsmith::combat_state::CombatState {
        attackers: attackers
            .into_iter()
            .map(|(creature, target)| ironsmith::combat_state::AttackerInfo { creature, target })
            .collect(),
        blockers: blockers.into_iter().collect(),
        block_declaration_complete: true,
        ..Default::default()
    };
    combat.remember_blocked_attackers();
    combat.record_attacked_permanent_types(g);
    g.combat = Some(combat.clone());
    g.turn.phase = ironsmith::Phase::Combat;
    g.turn.step = Some(ironsmith::Step::CombatDamage);
    let events =
        ironsmith::game_loop::try_execute_combat_damage_step_with_dm(g, &combat, false, dm)
            .unwrap();
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::queue_combat_damage_triggers(g, &events, &mut q);
    put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
    g.stack.len()
}
#[test]
fn all_six_exact_full_cards_compile_and_materialize_without_loss() {
    assert_eq!(fixtures().len(), 6);
    for r in fixtures() {
        for d in definitions(r["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
        }
    }
}
#[test]
fn bystander_threshold_combines_simultaneous_sources_but_not_separate_instructions() {
    for d in definitions("Innocent Bystander") {
        for together in [false, true] {
            let mut g = game();
            let observer = g.create_object_from_definition(&d, A, Zone::Battlefield);
            let one = creature(&mut g, B, 1);
            let two = creature(&mut g, B, 2);
            let mut dm = Choices::default();
            g.take_pending_trigger_events();
            if together {
                let events =
                    simultaneous(&mut g, observer, B, ChooseSpec::SpecificObject(observer));
                assert_eq!(stack(&mut g, events, &mut dm), 1);
            } else {
                for (source, amount) in [(one, 1), (two, 2)] {
                    let events = damage(
                        &mut g,
                        source,
                        ChooseSpec::SpecificObject(observer),
                        amount,
                        false,
                    );
                    assert_eq!(stack(&mut g, events, &mut dm), 0);
                }
            }
            settle(&mut g, &mut dm);
            assert_eq!(
                tokens(&g, ironsmith::types::Subtype::Clue).len(),
                usize::from(together)
            );
        }
    }
}
#[test]
fn bystander_actual_combat_is_one_received_occurrence_and_clue_is_executable() {
    for d in definitions("Innocent Bystander") {
        let mut g = game();
        let observer = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let one = creature(&mut g, B, 1);
        let two = creature(&mut g, B, 2);
        resources(&mut g, A, Zone::Library, 1);
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        assert_eq!(
            combat(
                &mut g,
                vec![(observer, ironsmith::combat_state::AttackTarget::Player(B))],
                vec![(observer, vec![one, two])],
                &mut dm
            ),
            1
        );
        settle(&mut g, &mut dm);
        let clue = tokens(&g, ironsmith::types::Subtype::Clue)[0];
        assert_eq!(g.object(clue).unwrap().name, "Clue Token");
        action(
            &mut g,
            A,
            LegalAction::ActivateAbility {
                source: clue,
                ability_index: 0,
            },
            &mut dm,
        );
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 1);
        assert!(g.object(clue).is_none());
    }
}
#[test]
fn pain_magnification_requires_single_source_total_and_keeps_each_recipient() {
    for d in definitions("Pain Magnification") {
        for powers in [vec![1, 2], vec![3, 3]] {
            let mut g = game();
            let observer = g.create_object_from_definition(&d, A, Zone::Battlefield);
            for power in powers.iter() {
                creature(&mut g, B, *power);
            }
            resources(&mut g, B, Zone::Hand, 5);
            resources(&mut g, C, Zone::Hand, 5);
            let mut dm = Choices::default();
            g.take_pending_trigger_events();
            let events = simultaneous(
                &mut g,
                observer,
                B,
                ChooseSpec::Player(PlayerFilter::Specific(C)),
            );
            let count = if powers == [1, 2] { 0 } else { 2 };
            assert_eq!(stack(&mut g, events, &mut dm), count);
            settle(&mut g, &mut dm);
            assert_eq!(g.player(C).unwrap().hand.len(), 5 - count);
            assert_eq!(g.player(B).unwrap().hand.len(), 5);
        }
        let mut g = game();
        let observer = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let source = creature(&mut g, A, 3);
        for p in [A, B, C] {
            resources(&mut g, p, Zone::Hand, 3);
        }
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        let events = damage(
            &mut g,
            source,
            ChooseSpec::EachPlayer(PlayerFilter::Opponent),
            3,
            false,
        );
        assert_eq!(stack(&mut g, events, &mut dm), 2);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 3);
        assert_eq!(g.player(B).unwrap().hand.len(), 2);
        assert_eq!(g.player(C).unwrap().hand.len(), 2);
        let events = damage(&mut g, source, ChooseSpec::SpecificPlayer(B), 2, false);
        assert_eq!(stack(&mut g, events, &mut dm), 0);
        let events = damage(&mut g, source, ChooseSpec::SpecificPlayer(B), 1, false);
        assert_eq!(stack(&mut g, events, &mut dm), 0);
    }
}
#[test]
fn pain_thresholds_use_the_real_incremental_combat_owner() {
    for d in definitions("Pain Magnification") {
        for powers in [vec![1, 2], vec![3, 3]] {
            let mut g = game();
            g.create_object_from_definition(&d, A, Zone::Battlefield);
            let attackers = powers
                .iter()
                .map(|p| {
                    (
                        creature(&mut g, C, *p),
                        ironsmith::combat_state::AttackTarget::Player(B),
                    )
                })
                .collect();
            resources(&mut g, B, Zone::Hand, 5);
            let mut dm = Choices::default();
            g.take_pending_trigger_events();
            let count = if powers == [1, 2] { 0 } else { 2 };
            assert_eq!(combat(&mut g, attackers, vec![], &mut dm), count);
            settle(&mut g, &mut dm);
            assert_eq!(g.player(B).unwrap().hand.len(), 5 - count);
        }
    }
}
#[test]
fn smaug_aggregates_noncombat_amounts_and_keeps_combat_zero_and_prevention_negatives() {
    for d in definitions("Smaug the Impenetrable") {
        let mut g = game();
        let smaug = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let one = creature(&mut g, B, 1);
        creature(&mut g, B, 2);
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        let events = simultaneous(&mut g, smaug, B, ChooseSpec::SpecificObject(smaug));
        assert_eq!(stack(&mut g, events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(tokens(&g, ironsmith::types::Subtype::Treasure).len(), 3);
        for (amount, combat) in [(0, false), (1, true)] {
            let events = damage(
                &mut g,
                one,
                ChooseSpec::SpecificObject(smaug),
                amount,
                combat,
            );
            assert_eq!(stack(&mut g, events, &mut dm), 0);
        }
        g.add_counters(smaug, ironsmith::CounterType::Shield, 1);
        g.take_pending_trigger_events();
        let events = damage(&mut g, one, ChooseSpec::SpecificObject(smaug), 5, false);
        assert_eq!(stack(&mut g, events, &mut dm), 0);
        assert_eq!(tokens(&g, ironsmith::types::Subtype::Treasure).len(), 3);
    }
}
#[test]
fn risona_keeps_first_counter_trigger_and_removes_only_for_received_combat_damage() {
    for d in definitions("Risona, Asari Commander") {
        let mut g = game();
        let risona = g.create_object_from_definition(&d, A, Zone::Battlefield);
        let enemy = creature(&mut g, B, 1);
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        let events = damage(&mut g, risona, ChooseSpec::SpecificPlayer(B), 1, true);
        assert_eq!(stack(&mut g, events, &mut dm), 1);
        settle(&mut g, &mut dm);
        assert_eq!(
            g.counter_count(risona, ironsmith::CounterType::Indestructible),
            1
        );
        let events = damage(&mut g, risona, ChooseSpec::SpecificPlayer(B), 1, true);
        assert_eq!(stack(&mut g, events, &mut dm), 0);
        for (target, combat, expected) in [(B, true, 0), (A, false, 0), (A, true, 1)] {
            let events = damage(&mut g, enemy, ChooseSpec::SpecificPlayer(target), 1, combat);
            assert_eq!(stack(&mut g, events, &mut dm), expected);
            settle(&mut g, &mut dm);
        }
        assert_eq!(
            g.counter_count(risona, ironsmith::CounterType::Indestructible),
            0
        );
    }
}
#[test]
fn totem_keeps_mana_animation_intervening_creature_condition_and_actual_sacrifice_body() {
    for d in definitions("Phyrexian Totem") {
        for expires in [false, true] {
            let mut g = game();
            let totem = g.create_object_from_definition(&d, A, Zone::Battlefield);
            let lands = resources(&mut g, A, Zone::Battlefield, 3);
            let enemy = creature(&mut g, B, 1);
            let mut dm = Choices {
                objects: lands.clone(),
                ..Default::default()
            };
            g.take_pending_trigger_events();
            let mana=d.abilities.iter().position(|a|matches!(a.kind,ironsmith::ability::AbilityKind::Activated(ref ability) if ability.is_mana_ability())).unwrap();
            let before = g.player(A).unwrap().mana_pool.black;
            action(
                &mut g,
                A,
                LegalAction::ActivateManaAbility {
                    source: totem,
                    ability_index: mana,
                },
                &mut dm,
            );
            assert_eq!(g.player(A).unwrap().mana_pool.black, before + 1);
            let animate=d.abilities.iter().position(|a|matches!(a.kind,ironsmith::ability::AbilityKind::Activated(ref ability) if !ability.is_mana_ability())).unwrap();
            action(
                &mut g,
                A,
                LegalAction::ActivateAbility {
                    source: totem,
                    ability_index: animate,
                },
                &mut dm,
            );
            settle(&mut g, &mut dm);
            assert_eq!(g.current_power(totem), Some(5));
            assert!(g.current_has_static_ability_id(
                totem,
                ironsmith::static_abilities::StaticAbilityId::Trample
            ));
            let events = damage(&mut g, enemy, ChooseSpec::SpecificObject(totem), 2, false);
            assert_eq!(
                stack(&mut g, events, &mut dm),
                1,
                "abilities={:?}",
                g.current_abilities(totem)
            );
            if expires {
                ironsmith::turn::execute_cleanup_step(&mut g);
            }
            settle(&mut g, &mut dm);
            assert_eq!(
                lands.iter().filter(|id| g.object(**id).is_none()).count(),
                if expires { 0 } else { 2 }
            );
            assert!(g.object(totem).is_some());
        }
    }
}
#[test]
fn pharaoh_has_two_recipient_occurrences_but_never_follows_a_later_graveyard_incarnation() {
    for d in definitions("Vengeful Pharaoh") {
        let mut g = game();
        let pharaoh = g.create_object_from_definition(&d, A, Zone::Graveyard);
        let walker = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Walker",
            "Type: Planeswalker\nLoyalty: 9",
        );
        let attackers = (0..3).map(|_| creature(&mut g, B, 1)).collect::<Vec<_>>();
        let mut dm = Choices {
            target: Some(attackers[0]),
            ..Default::default()
        };
        g.take_pending_trigger_events();
        assert_eq!(
            combat(
                &mut g,
                vec![
                    (
                        attackers[0],
                        ironsmith::combat_state::AttackTarget::Player(A)
                    ),
                    (
                        attackers[1],
                        ironsmith::combat_state::AttackTarget::Player(A)
                    ),
                    (
                        attackers[2],
                        ironsmith::combat_state::AttackTarget::Planeswalker(walker)
                    )
                ],
                vec![],
                &mut dm
            ),
            2
        );
        resolve(&mut g, &mut dm);
        assert!(g.object(pharaoh).is_none());
        let library = *g.player(A).unwrap().library.last().unwrap();
        let new_grave = g.move_object_by_effect(library, Zone::Graveyard).unwrap();
        settle(&mut g, &mut dm);
        assert_eq!(g.object(new_grave).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            attackers
                .iter()
                .filter(|id| g.object(**id).is_none())
                .count(),
            1
        );
    }
}
#[test]
fn pharaoh_invalid_target_and_wrong_source_zone_do_not_move_the_card() {
    for d in definitions("Vengeful Pharaoh") {
        for zone in [Zone::Battlefield, Zone::Graveyard] {
            let mut g = game();
            let pharaoh = g.create_object_from_definition(&d, A, zone);
            let attacker = creature(&mut g, B, 1);
            let mut dm = Choices {
                target: Some(attacker),
                ..Default::default()
            };
            g.take_pending_trigger_events();
            assert_eq!(
                combat(
                    &mut g,
                    vec![(attacker, ironsmith::combat_state::AttackTarget::Player(A))],
                    vec![],
                    &mut dm
                ),
                usize::from(zone == Zone::Graveyard)
            );
            if zone == Zone::Graveyard {
                g.move_object_by_effect(attacker, Zone::Exile).unwrap();
                settle(&mut g, &mut dm);
            }
            assert_eq!(g.object(pharaoh).unwrap().zone, zone);
        }
    }
}
#[test]
fn old_damage_wire_defaults_and_new_threshold_flags_survive_round_trip() {
    let trigger = ironsmith_core::trigger_model::Trigger::damage_received(
        ChooseSpec::Player(PlayerFilter::Opponent),
        Some(false),
        Some(3),
        true,
    );
    let mut json = serde_json::to_value(&trigger).unwrap();
    let restored: ironsmith_core::trigger_model::Trigger =
        serde_json::from_value(json.clone()).unwrap();
    assert_eq!(trigger, restored);
    let payload = json["kind"]["IsDealtDamage"].as_object_mut().unwrap();
    payload.remove("minimum");
    payload.remove("single_source");
    let old: ironsmith_core::trigger_model::Trigger = serde_json::from_value(json).unwrap();
    assert!(matches!(
        old.kind,
        ironsmith_core::trigger_model::TriggerKind::IsDealtDamage {
            minimum: None,
            single_source: false,
            noncombat_only: true,
            ..
        }
    ));
}

#[test]
fn combat_damage_observers_are_captured_before_replacement_or_prevention_removes_them() {
    for definition in definitions("Innocent Bystander") {
        for prevention in [false, true] {
            let mut g = game();
            let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let blocker = creature(&mut g, B, 3);
            let tail = vec![Effect::exile(ChooseSpec::SpecificObject(observer))];
            if prevention {
                g.effect_store.prevention_effects.add_shield(
                    ironsmith::prevention::PreventionShield::prevent_all(
                        blocker,
                        B,
                        ironsmith::prevention::PreventionTarget::Permanent(blocker),
                    )
                    .with_follow_up_effects(tail),
                );
            } else {
                g.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(
                        blocker,
                        B,
                        ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                            ObjectFilter::specific(blocker),
                        ),
                        ironsmith::replacement::ReplacementAction::Additionally(tail),
                    ),
                );
            }
            let mut dm = Choices::default();
            g.take_pending_trigger_events();
            assert_eq!(
                combat(
                    &mut g,
                    vec![(observer, ironsmith::combat_state::AttackTarget::Player(B))],
                    vec![(observer, vec![blocker])],
                    &mut dm
                ),
                1
            );
            assert!(g.object(observer).is_none());
            settle(&mut g, &mut dm);
            assert_eq!(tokens(&g, ironsmith::types::Subtype::Clue).len(), 1);
            assert_eq!(
                g.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
                if prevention { 1 } else { 2 },
                "publishing the captured receipt must not duplicate history"
            );
        }
    }
}
#[test]
fn interrupted_damage_addition_rolls_back_pending_damage_receipts_and_replays_once() {
    struct Answers {
        pause: bool,
        pending: bool,
    }
    impl DecisionMaker for Answers {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    for definition in definitions("Innocent Bystander") {
        for pause in [false, true] {
            let mut g = game();
            let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let blocker = creature(&mut g, B, 3);
            let tail = if pause {
                Effect::may(vec![Effect::exile(ChooseSpec::SpecificObject(observer))])
            } else {
                Effect::lose_life(ironsmith::effect::Value::X)
            };
            let replacement = g.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    blocker,
                    B,
                    ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                        ObjectFilter::specific(blocker),
                    ),
                    ironsmith::replacement::ReplacementAction::Additionally(vec![tail]),
                ),
            );
            let state = ironsmith::combat_state::CombatState {
                attackers: vec![ironsmith::combat_state::AttackerInfo {
                    creature: observer,
                    target: ironsmith::combat_state::AttackTarget::Player(B),
                }],
                blockers: std::collections::BTreeMap::from([(observer, vec![blocker])]),
                ..Default::default()
            };
            g.take_pending_trigger_events();
            let mut answers = Answers {
                pause,
                pending: false,
            };
            let result = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
                &mut g,
                &state,
                false,
                &mut answers,
            );
            if pause {
                assert!(result.unwrap().is_empty());
                assert!(answers.pending);
            } else {
                assert!(result.is_err());
            }
            assert_eq!(g.damage_on(observer), 0);
            assert_eq!(g.damage_on(blocker), 0);
            assert!(g.object(observer).is_some());
            assert!(
                g.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some()
            );
            assert_eq!(
                g.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
                0
            );
            let mut dm = Choices::default();
            assert_eq!(
                stack(&mut g, vec![], &mut dm),
                0,
                "no captured trigger may survive rollback"
            );
            if pause {
                answers.pause = false;
                answers.pending = false;
                let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
                    &mut g,
                    &state,
                    false,
                    &mut answers,
                )
                .unwrap();
                let mut q = TriggerQueue::new();
                ironsmith::game_loop::queue_combat_damage_triggers(&mut g, &events, &mut q);
                put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
                assert_eq!(g.stack.len(), 1);
                settle(&mut g, &mut dm);
                assert_eq!(tokens(&g, ironsmith::types::Subtype::Clue).len(), 1);
                assert_eq!(
                    g.trigger_event_kind_count_this_turn(ironsmith::events::EventKind::Damage),
                    2
                );
            }
        }
    }
}

#[test]
fn damage_receipt_precedes_lifelink_replacement_programs() {
    for definition in definitions("Pain Magnification") {
        let mut g = game();
        let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = object(
            &mut g,
            C,
            Zone::Battlefield,
            "Lifelink attacker",
            "Type: Creature — Beast\nPower/Toughness: 3/3\nLifelink",
        );
        resources(&mut g, B, Zone::Hand, 2);
        g.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                attacker,
                C,
                ironsmith::events::WouldGainLifeMatcher::you(),
                ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::exile(
                    ChooseSpec::SpecificObject(observer),
                )]),
            ),
        );
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        assert_eq!(
            combat(
                &mut g,
                vec![(attacker, ironsmith::combat_state::AttackTarget::Player(B))],
                vec![],
                &mut dm
            ),
            1
        );
        assert!(g.object(observer).is_none());
        settle(&mut g, &mut dm);
        assert_eq!(g.player(B).unwrap().hand.len(), 1);
    }
}

#[test]
fn damage_result_life_loss_additions_wait_for_original_damage_trigger_capture() {
    // Concrete shared result-receipt prerequisite; intentionally unignored.
    // The life/counter result owner must not publish this addition before the
    // original damage occurrence and its observer set have been captured.
    for definition in definitions("Pain Magnification") {
        let mut g = game();
        let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = creature(&mut g, C, 3);
        resources(&mut g, B, Zone::Hand, 2);
        g.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                attacker,
                B,
                ironsmith::events::WouldLoseLifeMatcher::you(),
                ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::exile(
                    ChooseSpec::SpecificObject(observer),
                )]),
            ),
        );
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        assert_eq!(
            combat(
                &mut g,
                vec![(attacker, ironsmith::combat_state::AttackTarget::Player(B))],
                vec![],
                &mut dm
            ),
            1
        );
        assert!(g.object(observer).is_none());
        settle(&mut g, &mut dm);
        assert_eq!(g.player(B).unwrap().hand.len(), 1);
    }
}

#[test]
fn combat_wither_counter_addition_cannot_remove_its_original_damage_observer() {
    for definition in definitions("Innocent Bystander") {
        let mut g = game();
        let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let blocker = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Wither blocker",
            "Type: Creature — Beast\nPower/Toughness: 3/8\nWither",
        );
        g.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                blocker,
                B,
                ironsmith::events::counters::matchers::WouldPutCountersMatcher::new(
                    ObjectFilter::specific(observer),
                    Some(ironsmith::CounterType::MinusOneMinusOne),
                ),
                ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::exile(
                    ChooseSpec::SpecificObject(observer),
                )]),
            ),
        );
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        assert_eq!(
            combat(
                &mut g,
                vec![(observer, ironsmith::combat_state::AttackTarget::Player(B))],
                vec![(observer, vec![blocker])],
                &mut dm
            ),
            1
        );
        assert!(g.object(observer).is_none());
        settle(&mut g, &mut dm);
        assert_eq!(tokens(&g, ironsmith::types::Subtype::Clue).len(), 1);
    }
}

#[test]
fn combat_toxic_counter_addition_waits_for_damage_observer_capture() {
    for definition in definitions("Pain Magnification") {
        let mut g = game();
        let observer = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = object(
            &mut g,
            C,
            Zone::Battlefield,
            "Toxic attacker",
            "Type: Creature — Beast\nPower/Toughness: 3/3\nToxic 1",
        );
        resources(&mut g, B, Zone::Hand, 2);
        let mut replacement =
            ironsmith::static_abilities::StaticAbility::double_player_counters_replacement(
                PlayerFilter::Specific(B),
                Some(ironsmith::CounterType::Poison),
                "Poison receipt fixture".into(),
            )
            .generate_replacement_effect(attacker, C)
            .unwrap();
        replacement.replacement =
            ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::exile(
                ChooseSpec::SpecificObject(observer),
            )]);
        g.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        let mut dm = Choices::default();
        g.take_pending_trigger_events();
        assert_eq!(
            combat(
                &mut g,
                vec![(attacker, ironsmith::combat_state::AttackTarget::Player(B))],
                vec![],
                &mut dm
            ),
            1
        );
        assert_eq!(g.player(B).unwrap().poison_counters, 1);
        assert!(g.object(observer).is_none());
        settle(&mut g, &mut dm);
        assert_eq!(g.player(B).unwrap().hand.len(), 1);
    }
}

#[derive(Clone, Debug)]
struct GainAtLowLife;
impl ironsmith::events::ReplacementMatcher for GainAtLowLife {
    fn matches_prepared_event(
        &self,
        event: &dyn ironsmith::events::GameEventType,
        ctx: &ironsmith::events::context::PreparedEventContext,
    ) -> bool {
        ironsmith::events::downcast_event::<ironsmith::events::LifeGainEvent>(event)
            .is_some_and(|gain| gain.player == B && ctx.game.player(B).unwrap().life < 30)
    }
    fn display(&self) -> String {
        "Double Bob's gain while below 30 life".into()
    }
}
#[test]
fn combat_lifelink_prepares_before_results_and_loss_trigger_sees_completed_frame() {
    for conditional_gain in [false, true] {
        let mut g = game();
        let observer = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Completed life observer",
            "Type: Enchantment\nWhenever you lose life, if your life total is less than 30, draw a card.",
        );
        let attacker = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Trampler",
            "Type: Creature — Beast\nPower/Toughness: 6/6\nTrample",
        );
        let blocker = object(
            &mut g,
            B,
            Zone::Battlefield,
            "Lifelink blocker",
            "Type: Creature — Beast\nPower/Toughness: 3/3\nLifelink",
        );
        resources(&mut g, B, Zone::Library, 1);
        if conditional_gain {
            g.effect_store.replacement_effects.add_resolution_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    observer,
                    B,
                    GainAtLowLife,
                    ironsmith::replacement::ReplacementAction::Double,
                ),
            );
        }
        g.turn_store
            .combat_damage_assignments
            .insert(attacker, [(blocker, 3)].into_iter().collect());
        g.take_pending_trigger_events();
        let mut dm = Choices::default();
        assert_eq!(
            combat(
                &mut g,
                vec![(attacker, ironsmith::combat_state::AttackTarget::Player(B))],
                vec![(attacker, vec![blocker])],
                &mut dm
            ),
            0
        );
        assert_eq!(
            g.player(B).unwrap().life,
            30,
            "the gain replacement sees the original 30, not transient 27"
        );
        assert!(
            g.player(B).unwrap().hand.is_empty(),
            "loss trigger sees completed 30"
        );
    }
}
#[test]
fn toxic_occurs_once_per_source_and_final_player_after_split_redirection() {
    use ironsmith::replacement::{
        RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect,
    };
    for reconverge in [false, true] {
        let mut g = game();
        let source = object(
            &mut g,
            A,
            Zone::Battlefield,
            "Toxic attacker",
            "Type: Creature — Beast\nPower/Toughness: 3/3\nToxic 2",
        );
        g.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                B,
                ironsmith::events::DamageToPlayerMatcher::new(PlayerFilter::Specific(B)),
                ReplacementAction::RedirectDamageAmount {
                    target: RedirectTarget::ToPlayer(C),
                    which: RedirectWhich::First,
                    amount: 1,
                },
            ));
        if reconverge {
            g.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    C,
                    ironsmith::events::DamageToPlayerMatcher::new(PlayerFilter::Specific(C)),
                    ReplacementAction::Redirect {
                        target: RedirectTarget::ToPlayer(B),
                        which: RedirectWhich::First,
                    },
                ),
            );
        }
        let mut dm = Choices::default();
        combat(
            &mut g,
            vec![(source, ironsmith::combat_state::AttackTarget::Player(B))],
            vec![],
            &mut dm,
        );
        assert_eq!(
            g.player(B)
                .unwrap()
                .counter_count(ironsmith::CounterType::Poison),
            2
        );
        assert_eq!(
            g.player(C)
                .unwrap()
                .counter_count(ironsmith::CounterType::Poison),
            if reconverge { 0 } else { 2 }
        );
        assert_eq!(g.player(B).unwrap().life, if reconverge { 27 } else { 28 });
    }
}

#[test]
fn captured_combat_receipts_publish_history_once_even_when_reingested() {
    let mut g = game();
    let source = object(
        &mut g,
        A,
        Zone::Battlefield,
        "History lifelinker",
        "Type: Creature — Beast\nPower/Toughness: 3/3\nLifelink",
    );
    let combat = ironsmith::combat_state::CombatState {
        attackers: vec![ironsmith::combat_state::AttackerInfo {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
        ..Default::default()
    };
    let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
        &mut g,
        &combat,
        false,
        &mut Choices::default(),
    )
    .unwrap();
    for _ in 0..2 {
        let history = &g.turn_store.turn_history;
        assert_eq!(history.total_damage_to_player(B), 3);
        assert_eq!(history.total_life_lost_for_players(&[B]), 3);
        assert_eq!(history.total_life_gained_for_players(&[A]), 3);
        for kind in [
            ironsmith::events::EventKind::Damage,
            ironsmith::events::EventKind::LifeLoss,
            ironsmith::events::EventKind::LifeGain,
        ] {
            assert_eq!(history.event_kind_count(kind), 1);
        }
        ironsmith::game_loop::queue_combat_damage_triggers(
            &mut g,
            &events,
            &mut TriggerQueue::new(),
        );
    }
}
#[test]
fn ordinary_damage_output_does_not_restage_captured_originals_and_keeps_fresh_additions() {
    let mut g = game();
    let source = creature(&mut g, A, 3);
    g.effect_store.replacement_effects.add_one_shot_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(
            source,
            A,
            ironsmith::events::DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
            ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::gain_life(2)]),
        ),
    );
    let outcome = ironsmith::effects::execute_effect(
        &mut g,
        &Effect::new(DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(B))),
        &mut ExecutionContext::new_default(source, A),
    )
    .unwrap();
    assert_eq!(g.turn_store.turn_history.total_damage_to_player(B), 3);
    assert_eq!(
        g.turn_store.turn_history.total_life_lost_for_players(&[B]),
        3
    );
    assert_eq!(
        g.turn_store
            .turn_history
            .total_life_gained_for_players(&[A]),
        2
    );
    stack(&mut g, outcome.events, &mut Choices::default());
    assert_eq!(g.turn_store.turn_history.total_damage_to_player(B), 3);
    assert_eq!(
        g.turn_store.turn_history.total_life_lost_for_players(&[B]),
        3
    );
    assert_eq!(
        g.turn_store
            .turn_history
            .total_life_gained_for_players(&[A]),
        2
    );
    for kind in [
        ironsmith::events::EventKind::Damage,
        ironsmith::events::EventKind::LifeLoss,
        ironsmith::events::EventKind::LifeGain,
    ] {
        assert_eq!(g.turn_store.turn_history.event_kind_count(kind), 1);
    }
}
