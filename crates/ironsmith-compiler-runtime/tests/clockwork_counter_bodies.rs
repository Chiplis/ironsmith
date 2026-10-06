//! Exact frozen bodies and shared counter/history semantics. Authored during
//! the implementation-first campaign; execution is deliberately deferred.
use ironsmith::ability::{AbilityKind, ActivationTiming};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, BooleanContext};
use ironsmith::effect::{Effect, Condition, Value};
use ironsmith::effects::{EffectContext, EffectExecutor, PutCountersEffect};
use ironsmith::game_loop::{apply_attacker_declarations, apply_blocker_declarations, generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with, PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
use ironsmith::game_state::{Phase, Step};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CounterType, GameState, ObjectId, PlayerId, Zone, GameProgress};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const COUNTER: CounterType = CounterType::PlusOnePlusZero;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/clockwork_counter_bodies.json.fixture")).unwrap()
}
fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct_result, direct_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    let direct = direct_result.unwrap_or_else(|error| panic!("{name} direct: {error}"));
    assert!(!direct_loss.is_lossy(), "{name} direct: {}", direct_loss.reasons_text());
    let (artifact_result, artifact_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, text, false)
    });
    let (artifact, _) = artifact_result.unwrap_or_else(|error| panic!("{name} artifact: {error}"));
    assert!(!artifact_loss.is_lossy(), "{name} artifact: {}", artifact_loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.turn_number = 4;
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 30);
    g
}
fn enter(g: &mut GameState, d: &CardDefinition, controller: PlayerId) -> ObjectId {
    let old = g.create_object_from_definition(d, controller, Zone::Hand);
    let stable = g.object(old).unwrap().stable_id;
    ironsmith::effects::PutOntoBattlefieldEffect::new(ChooseSpec::SpecificObject(old), false, PlayerFilter::You)
        .execute(g, &mut EffectContext::new_default(old, controller)).unwrap();
    let id = g.find_object_by_stable_id(stable).unwrap();
    g.remove_summoning_sickness(id);
    id
}
fn refill(d: &CardDefinition) -> PutCountersEffect {
    fn find(effect: &Effect) -> Option<PutCountersEffect> {
        if let Some(put) = effect.downcast_ref::<PutCountersEffect>() { return Some(put.clone()); }
        let mut result = None;
        effect.visit_child_effects(&mut |child| { if result.is_none() { result = find(child); } });
        result
    }
    d.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => activated.effects.all_effects().into_iter().find_map(find),
        _ => None,
    }).expect("full body retains refill")
}
fn activated(d: &CardDefinition) -> usize {
    d.abilities.iter().position(|ability| matches!(&ability.kind, AbilityKind::Activated(_))).unwrap()
}
fn end_combat(g: &mut GameState) -> usize {
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::EndCombat);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(g, &mut queue);
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(g, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    while !g.stack_is_empty() { resolve_stack_entry_with(g, &mut SelectFirstDecisionMaker).unwrap(); }
    count
}
fn history(g: &GameState, source: ObjectId, controller: PlayerId) -> bool {
    ironsmith::condition_eval::evaluate_condition_resolution(g,
        &Condition::SourceAttackedOrBlockedThisCombat, &EffectContext::new_default(source, controller)).unwrap()
}
fn declare(g: &mut GameState, source: ObjectId, blocks: bool) {
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareAttackers);
    g.mark_combat_phase_started();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let attacker = if blocks {
        let d = compile_to_runtime_definition("Combat partner", "Type: Creature\nPower/Toughness: 1/10", false).unwrap();
        let id = g.create_object_from_definition(&d, A, Zone::Battlefield);
        g.remove_summoning_sickness(id);
        id
    } else { source };
    apply_attacker_declarations(g, &mut combat, &mut queue, &[AttackerDeclaration { creature: attacker, target: AttackTarget::Player(B) }]).unwrap();
    if blocks {
        g.turn.step = Some(Step::DeclareBlockers);
        apply_blocker_declarations(g, &mut combat, &mut queue, &[BlockerDeclaration { blocker: source, blocking: attacker }], B).unwrap();
    }
    assert!(queue.entries.is_empty(), "decay happens at end of combat");
}
#[derive(Default)]
struct Choices { x: u32, number: u32, pause: bool, pending: bool, maximum: Option<u32> }
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value { return self.x.min(context.max); }
        self.maximum = Some(context.max);
        if self.pause { self.pending = true; }
        self.number.min(context.max)
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        if self.pause { self.pending = true; }
        true
    }
}
fn execute_refill(g: &mut GameState, source: ObjectId, put: &PutCountersEffect, dm: &mut Choices) -> ironsmith::effect::EffectOutcome {
    let mut ctx = EffectContext::new(source, A, dm);
    ctx.x_value = Some(9);
    put.execute(g, &mut ctx).unwrap()
}
fn activate(g: &mut GameState, source: ObjectId, index: usize, dm: &mut Choices) {
    let action = LegalAction::ActivateAbility { source, ability_index: index };
    assert!(compute_legal_actions(g, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(g, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..50 {
        if state.pending_activation.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("activation must yield its required choice") };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(g.stack.len(), 1);
}

#[test]
fn four_complete_frozen_bodies_retain_entry_timing_ceiling_and_combat_condition() {
    assert_eq!(fixtures().len(), 4);
    for row in fixtures() {
        for d in definitions(&row) {
            let maximum = row["ceiling"].as_u64().unwrap() as u32;
            let put = refill(&d);
            assert_eq!(put.maximum_total, Some(maximum));
            assert_eq!(put.counter_type, COUNTER);
            assert_eq!(put.amount.unhinted(), &Value::X);
            assert!(put.amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::UpTo));
            assert!(d.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Activated(a) if a.timing == ActivationTiming::DuringYourUpkeep)));
            assert!(d.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(t) if t.intervening_if == Some(Condition::SourceAttackedOrBlockedThisCombat))));
            let mut g = game();
            let source = enter(&mut g, &d, A);
            assert_eq!(g.counter_count(source, COUNTER), maximum);
            assert_eq!(g.calculated_power(source), Some(maximum as i32));
            let compiled = ironsmith_text::canonical_compiled_lines(&d).join("\n");
            assert!(compiled.contains("this combat"));
            assert!(compiled.contains("This ability can't cause the total number"));
        }
        let mut renamed = row.clone();
        renamed["name"] = "Synthetic Counter Construct".into();
        for d in definitions(&renamed) { assert_eq!(refill(&d).maximum_total, Some(row["ceiling"].as_u64().unwrap() as u32)); }
    }
}

#[test]
fn attack_and_block_declarations_survive_removal_and_native_restore_but_not_another_combat() {
    for row in fixtures() {
        for d in definitions(&row) {
            for blocks in [false, true] {
                let mut g = game();
                let controller = if blocks { B } else { A };
                let source = enter(&mut g, &d, controller);
                let original = g.counter_count(source, COUNTER);
                declare(&mut g, source, blocks);
                assert!(history(&g, source, controller));
                ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(source))
                    .execute(&mut g, &mut EffectContext::new_default(source, controller)).unwrap();
                let native = g.clone();
                for mut branch in [g, native] {
                    assert!(history(&branch, source, controller));
                    assert_eq!(end_combat(&mut branch), 1);
                    assert_eq!(branch.counter_count(source, COUNTER), original - 1);
                    branch.mark_combat_phase_started();
                    branch.combat = Some(CombatState::default());
                    assert!(!history(&branch, source, controller));
                    assert_eq!(end_combat(&mut branch), 0);
                    assert_eq!(branch.counter_count(source, COUNTER), original - 1);
                }
            }
        }
    }
}

#[test]
fn current_attacking_status_and_old_incarnations_do_not_count_as_declarations() {
    for row in fixtures() {
        for d in definitions(&row) {
            let mut g = game();
            let source = enter(&mut g, &d, A);
            declare(&mut g, source, false);
            let token_result = ironsmith::effects::CreateTokenEffect::one(d.clone()).tapped().attacking()
                .execute(&mut g, &mut EffectContext::new_default(source, A)).unwrap();
            let ironsmith::effect::OutcomeValue::Objects(ids) = token_result.value else { panic!("created token") };
            assert!(!history(&g, ids[0], A));
            let stable = g.object(source).unwrap().stable_id;
            g.move_object_by_effect(source, Zone::Exile).unwrap();
            let exiled = g.find_object_by_stable_id(stable).unwrap();
            ironsmith::effects::PutOntoBattlefieldEffect::new(ChooseSpec::SpecificObject(exiled), false, PlayerFilter::You)
                .execute(&mut g, &mut EffectContext::new_default(exiled, A)).unwrap();
            let returned = g.find_object_by_stable_id(stable).unwrap();
            assert_ne!(source, returned);
            assert!(!history(&g, returned, A));
            assert_eq!(end_combat(&mut g), 0);
            assert_eq!(g.counter_count(returned, COUNTER), row["ceiling"].as_u64().unwrap() as u32);
        }
    }
}

#[test]
fn x_tap_activation_is_upkeep_only_and_resolution_respects_player_choice_and_new_headroom() {
    for row in fixtures() {
        for d in definitions(&row) {
            let mut g = game();
            let source = enter(&mut g, &d, A);
            let index = activated(&d);
            let maximum = row["ceiling"].as_u64().unwrap() as u32;
            let action = LegalAction::ActivateAbility { source, ability_index: index };
            for (active, phase, step, legal) in [(A, Phase::FirstMain, None, false), (B, Phase::Beginning, Some(Step::Upkeep), false), (A, Phase::Beginning, Some(Step::Upkeep), true)] {
                g.turn.active_player = active; g.turn.phase = phase; g.turn.step = step;
                assert_eq!(compute_legal_actions(&g, A).unwrap().contains(&action), legal);
            }
            g.object_mut(source).unwrap().counters.insert(COUNTER, 1);
            let mut dm = Choices { x: 3, number: 1, ..Default::default() };
            activate(&mut g, source, index, &mut dm);
            assert!(g.is_tapped(source));
            let native = g.clone();
            for mut branch in [g, native] {
                resolve_stack_entry_with(&mut branch, &mut dm).unwrap();
                assert_eq!(branch.counter_count(source, COUNTER), 2);
                assert_eq!(dm.maximum, Some(3.min(maximum - 1)));
            }
        }
    }
}

#[test]
fn ceilings_limit_only_this_refill_even_after_counter_doubling_and_never_remove_excess() {
    for row in fixtures() {
        for d in definitions(&row) {
            let maximum = row["ceiling"].as_u64().unwrap() as u32;
            let put = refill(&d);
            for before in [0, maximum - 1, maximum, maximum + 2] {
                let mut g = game();
                let source = enter(&mut g, &d, A);
                g.object_mut(source).unwrap().counters.insert(COUNTER, before);
                let replacement = ironsmith::static_abilities::StaticAbility::double_counters_replacement(
                    ironsmith::target::ObjectFilter::creature(), Some(COUNTER), "Counter modifier".into())
                    .generate_replacement_effect(source, A).unwrap();
                g.effect_store.replacement_effects.add_resolution_effect(replacement);
                let out = execute_refill(&mut g, source, &put, &mut Choices { number: 9, ..Default::default() });
                assert_eq!(g.counter_count(source, COUNTER), before.max(maximum));
                assert_eq!(out.count_or_zero(), i64::from(maximum.saturating_sub(before)));
                PutCountersEffect::new(COUNTER, 2, ChooseSpec::Source)
                    .execute(&mut g, &mut EffectContext::new_default(source, A)).unwrap();
                assert_eq!(g.counter_count(source, COUNTER), before.max(maximum) + 4);
            }
        }
    }
}

#[test]
fn pending_refill_and_failed_replacement_restore_counters_context_and_one_shot() {
    for row in fixtures() {
        for d in definitions(&row) {
            let put = refill(&d);
            let mut g = game();
            let source = enter(&mut g, &d, A);
            g.object_mut(source).unwrap().counters.insert(COUNTER, 1);
            let native = g.clone();
            let mut dm = Choices { number: 1, pause: true, ..Default::default() };
            let out = execute_refill(&mut g, source, &put, &mut dm);
            assert!(dm.pending); assert!(out.events.is_empty()); assert!(out.execution_facts.is_empty());
            assert_eq!(g.counter_count(source, COUNTER), 1);
            assert!(g.take_pending_trigger_events().is_empty());
            dm.pause = false; dm.pending = false;
            for mut branch in [g, native.clone()] {
                assert_eq!(execute_refill(&mut branch, source, &put, &mut dm).count_or_zero(), 1);
                assert_eq!(branch.counter_count(source, COUNTER), 2);
            }
            for pending in [false, true] {
                let mut branch = native.clone();
                let mut replacement = ironsmith::static_abilities::StaticAbility::double_counters_replacement(
                    ironsmith::target::ObjectFilter::creature(), Some(COUNTER), "Replace counter placement".into())
                    .generate_replacement_effect(source, A).unwrap();
                replacement.replacement = ironsmith::replacement::ReplacementAction::Instead(vec![
                    Effect::gain_life(2),
                    if pending { Effect::may(vec![Effect::gain_life(1)]) } else { Effect::gain_life(Value::X) },
                ]);
                let one_shot = branch.effect_store.replacement_effects.add_one_shot_effect(replacement);
                let mut fixed = put.clone(); fixed.amount = Value::Fixed(1);
                let mut dm = Choices { pause: pending, ..Default::default() };
                let mut ctx = EffectContext::new(source, A, &mut dm);
                ctx.set_tagged_players("existing", vec![A]);
                let result = fixed.execute(&mut branch, &mut ctx);
                assert_eq!(result.is_err(), !pending);
                assert_eq!(ctx.get_tagged_players("existing"), Some(&vec![A]));
                if pending { let out = result.unwrap(); assert!(out.events.is_empty()); }
                assert_eq!(branch.counter_count(source, COUNTER), 1);
                assert_eq!(branch.player(A).unwrap().life, 20);
                assert!(branch.effect_store.replacement_effects.get_effect(one_shot).is_some());
                assert!(branch.take_pending_trigger_events().is_empty());
            }
        }
    }
}

#[test]
fn full_companion_flying_and_blocker_exclusions_remain_live() {
    for row in fixtures() {
        for d in definitions(&row) {
            let mut g = game(); let source = enter(&mut g, &d, A);
            for (types, flying, allowed) in match row["name"].as_str().unwrap() {
                "Clockwork Avian" => vec![("Creature — Bear", false, false), ("Creature — Bird", true, true)],
                "Clockwork Steed" => vec![("Artifact Creature — Golem", false, false), ("Creature — Bear", false, true)],
                "Clockwork Swarm" => vec![("Creature — Wall", false, false), ("Creature — Bear", false, true)],
                _ => vec![("Creature — Bear", false, true)],
            } {
                let blocker = compile_to_runtime_definition("Blocker", format!("Type: {types}\nPower/Toughness: 1/10\n{}", if flying { "Flying" } else { "" }), false).unwrap();
                let blocker = g.create_object_from_definition(&blocker, B, Zone::Battlefield);
                assert_eq!(ironsmith::rules::combat::can_block(g.object(source).unwrap(), g.object(blocker).unwrap(), &g), allowed);
            }
        }
    }
}

#[test]
fn effect_schema_retains_local_limit_and_old_counter_payloads_default_to_unbounded() {
    let put = PutCountersEffect::new(COUNTER, 2, ChooseSpec::Source).with_maximum_total(4);
    let mut payload = serde_json::to_value(&put).unwrap();
    assert_eq!(serde_json::from_value::<PutCountersEffect>(payload.clone()).unwrap(), put);
    payload.as_object_mut().unwrap().remove("maximum_total");
    assert_eq!(serde_json::from_value::<PutCountersEffect>(payload).unwrap().maximum_total, None);
}

#[test]
fn optional_zero_and_resolution_time_headroom_are_preserved_across_native_stack_clone() {
    for row in fixtures() {
        for d in definitions(&row) {
            let maximum = row["ceiling"].as_u64().unwrap() as u32;
            for (x, chosen, counters_at_resolution, expected) in [
                (0, 0, 1, 1),
                (3, 0, 1, 1),
                (3, 3, maximum - 1, maximum),
                (3, 3, maximum + 2, maximum + 2),
            ] {
                let mut g = game();
                g.turn.phase = Phase::Beginning; g.turn.step = Some(Step::Upkeep);
                let source = enter(&mut g, &d, A);
                g.object_mut(source).unwrap().counters.insert(COUNTER, 1);
                let mut dm = Choices { x, number: chosen, ..Default::default() };
                activate(&mut g, source, activated(&d), &mut dm);
                g.object_mut(source).unwrap().counters.insert(COUNTER, counters_at_resolution);
                let native = g.clone();
                for mut branch in [g, native] {
                    resolve_stack_entry_with(&mut branch, &mut dm).unwrap();
                    assert_eq!(branch.counter_count(source, COUNTER), expected);
                }
            }
        }
    }
}

#[test]
fn missing_declaration_phase_is_incomplete_evidence_not_current_combat_or_a_negative_fact() {
    for row in fixtures() {
        for d in definitions(&row) {
            let mut g = game();
            let source = enter(&mut g, &d, A);
            g.mark_combat_phase_started();
            g.turn_store.turn_history.event_records.push(ironsmith::turn_history::TurnEventRecord {
                event: ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::combat::CreatureAttackedEvent::new(source,
                        ironsmith::triggers::AttackEventTarget::Player(B)), Default::default()),
                object_snapshot: None,
                source_snapshot: None,
            });
            let native = g.clone();
            for branch in [g, native] {
                assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&branch,
                    &Condition::SourceAttackedOrBlockedThisCombat,
                    &EffectContext::new_default(source, A)),
                    Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
            }
        }
    }
}
