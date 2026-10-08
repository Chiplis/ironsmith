//! Frozen Bioshift whole-body source scenarios. Authored UNRUN; no admission claim.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{DecisionContext, NumberContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::replacement::{ReplacementAction, ReplacementEffect, EventModification};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CounterType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const KIND: CounterType = CounterType::PlusOnePlusOne;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/bioshift_source_gate.json.fixture")).unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["name"], "Bioshift");
    assert_eq!(row["oracle_id"], "3da604fc-68e9-4749-98f9-6fbcebcab9b6");
    assert_eq!(row["id"], "6e18f7a9-2af6-467a-8f62-5f7da83a3c92");
    assert_eq!(row["mana_cost"], "{G/U}");
    assert_eq!(row["type_line"], "Instant");
    assert_eq!(row["oracle_text"], "Move any number of +1/+1 counters from target creature onto another target creature with the same controller.");
    assert_eq!(row["colors"], serde_json::json!(["G", "U"]));
    assert_eq!(row["color_identity"], serde_json::json!(["G", "U"]));
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    // These are independent calls: the artifact compiler's accompanying runtime
    // value is deliberately discarded rather than presented as a direct path.
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Bioshift", &text, false));
    let direct = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Bioshift", &text, false));
    let (artifact, _) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        assert!(definition.spell_effect.is_some());
    }
    [direct, materialized]
}
fn setup(definition: &CardDefinition, owner: PlayerId) -> (GameState, ObjectId, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
    let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Bioshift endpoint")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
    let from = game.create_object_from_definition(&creature, owner, Zone::Battlefield);
    let to = game.create_object_from_definition(&creature, owner, Zone::Battlefield);
    game.object_mut(from).unwrap().counters.insert(KIND, 4);
    game.object_mut(from).unwrap().counters.insert(CounterType::Charge, 3);
    game.object_mut(to).unwrap().counters.insert(CounterType::Charge, 5);
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    (game, from, to, spell)
}
fn announce(game: &mut GameState, spell: ObjectId, from: ObjectId, to: ObjectId, reject: bool) {
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let action = LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand,
        casting_method: ironsmith::alternative_cast::CastingMethod::Normal };
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    let mut saw_targets = false;
    for _ in 0..32 {
        if !state.has_pending_action() && !game.stack.is_empty() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("cast must expose its pending decision"); };
        // Preserve the native pending-cast representation rather than JSON game state.
        *game = game.clone(); state = state.clone();
        if let DecisionContext::Targets(context) = &context {
            saw_targets = true;
            assert_eq!(context.requirements.len(), 2);
            assert!(context.requirements.iter().all(|r| r.min_targets == 1 && r.max_targets == Some(1)));
            let result = apply_priority_response_with_dm(game, &mut queue, &mut state,
                &PriorityResponse::Targets(vec![Target::Object(from), Target::Object(to)]), &mut dm);
            if reject {
                assert!(result.is_err()); assert!(game.stack.is_empty());
                assert_eq!(game.counter_count(from, KIND), 4);
                return;
            }
            progress = result.unwrap();
        } else {
            progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
        }
    }
    assert!(!reject, "invalid declaration must be rejected at target submission");
    assert!(saw_targets); assert!(!state.has_pending_action());
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].targets, vec![Target::Object(from), Target::Object(to)]);
    assert_eq!(game.stack[0].target_assignments.len(), 2);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}
#[derive(Default)]
struct Amount { chosen: u32, pause: bool, pending: bool, bounds: Vec<(u32, u32)> }
impl DecisionMaker for Amount {
    fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
        self.bounds.push((context.min, context.max)); self.pending = self.pause;
        assert!(self.chosen >= context.min && self.chosen <= context.max); self.chosen
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn untouched_other_counters(game: &GameState, from: ObjectId, to: ObjectId) {
    assert_eq!(game.counter_count(from, CounterType::Charge), 3);
    assert_eq!(game.counter_count(to, CounterType::Charge), 5);
}
#[test]
fn frozen_bioshift_zero_partial_all_and_opponent_owned_pair_pay_and_resolve() {
    for definition in definitions() {
        for owner in [A, B] {
            for chosen in [0, 1, 4] {
                let (mut game, from, to, spell) = setup(&definition, owner);
                announce(&mut game, spell, from, to, false);
                for mut branch in [game.clone(), game] {
                    let mut dm = Amount { chosen, ..Default::default() };
                    resolve_stack_entry_with(&mut branch, &mut dm).unwrap();
                    assert_eq!(dm.bounds, vec![(0, 4)]);
                    assert!(branch.stack.is_empty());
                    assert_eq!(branch.counter_count(from, KIND), 4 - chosen);
                    assert_eq!(branch.counter_count(to, KIND), chosen);
                    untouched_other_counters(&branch, from, to);
                }
            }
        }
    }
}
#[test]
fn frozen_bioshift_refuses_same_endpoint_and_different_controllers() {
    for definition in definitions() {
        for same in [false, true] {
            let (mut game, from, to, spell) = setup(&definition, A);
            if !same { game.object_mut(to).unwrap().initial_controller = B; }
            announce(&mut game, spell, from, if same { from } else { to }, true);
            assert_eq!(game.counter_count(to, KIND), 0);
            untouched_other_counters(&game, from, to);
        }
    }
}
#[test]
fn frozen_bioshift_rechecks_departure_and_current_controllers_at_resolution() {
    for definition in definitions() {
        for scenario in ["donor left", "recipient left", "donor changed", "recipient changed", "both changed"] {
            let (mut game, from, to, spell) = setup(&definition, A);
            announce(&mut game, spell, from, to, false);
            match scenario {
                "donor left" => { game.move_object_by_effect(from, Zone::Graveyard).unwrap(); }
                "recipient left" => { game.move_object_by_effect(to, Zone::Graveyard).unwrap(); }
                "donor changed" => { game.object_mut(from).unwrap().initial_controller = B; }
                "recipient changed" => { game.object_mut(to).unwrap().initial_controller = B; }
                _ => { game.object_mut(from).unwrap().initial_controller = B; game.object_mut(to).unwrap().initial_controller = B; }
            }
            for mut branch in [game.clone(), game] {
                let mut dm = Amount { chosen: 1, ..Default::default() };
                resolve_stack_entry_with(&mut branch, &mut dm).unwrap();
                assert!(branch.stack.is_empty());
                let moved = scenario == "both changed";
                assert_eq!(dm.bounds.len(), usize::from(moved), "{scenario}");
                if scenario != "donor left" { assert_eq!(branch.counter_count(from, KIND), if moved { 3 } else { 4 }, "{scenario}"); }
                if scenario != "recipient left" { assert_eq!(branch.counter_count(to, KIND), u32::from(moved), "{scenario}"); }
            }
        }
    }
}
#[test]
fn frozen_bioshift_pending_choice_rolls_back_stack_and_one_shot_then_retries_natively() {
    for definition in definitions() {
        let (mut game, from, to, spell) = setup(&definition, A);
        announce(&mut game, spell, from, to, false);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(to, A,
            ironsmith::events::counters::matchers::WouldPutCountersMatcher::any(),
            ReplacementAction::Modify(EventModification::Multiply(2))));
        let mut dm = Amount { chosen: 1, pause: true, ..Default::default() };
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Object(from), Target::Object(to)]);
        assert_eq!(game.stack[0].target_assignments.len(), 2);
        assert_eq!(game.counter_count(from, KIND), 4); assert_eq!(game.counter_count(to, KIND), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "suspended resolution must not undo paid casting");
        untouched_other_counters(&game, from, to);
        for mut branch in [game.clone(), game] {
            let mut retry = Amount { chosen: 1, ..Default::default() };
            resolve_stack_entry_with(&mut branch, &mut retry).unwrap();
            assert_eq!(retry.bounds, vec![(0, 4)]); assert!(branch.stack.is_empty());
            assert_eq!(branch.counter_count(from, KIND), 3); assert_eq!(branch.counter_count(to, KIND), 2);
            assert!(branch.effect_store.replacement_effects.get_effect(shield).is_none());
            untouched_other_counters(&branch, from, to);
        }
    }
}
#[test]
fn frozen_bioshift_empty_donor_and_zero_choice_leave_replacement_unused() {
    for definition in definitions() {
        for empty in [false, true] {
            let (mut game, from, to, spell) = setup(&definition, A);
            if empty { game.object_mut(from).unwrap().counters.remove(&KIND); }
            announce(&mut game, spell, from, to, false);
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(to, A,
                ironsmith::events::counters::matchers::WouldPutCountersMatcher::any(),
                ReplacementAction::Modify(EventModification::Multiply(2))));
            let mut dm = Amount::default();
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.bounds.len(), usize::from(!empty));
            assert_eq!(game.counter_count(from, KIND), if empty { 0 } else { 4 });
            assert_eq!(game.counter_count(to, KIND), 0); assert!(game.stack.is_empty());
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            untouched_other_counters(&game, from, to);
        }
    }
}
#[test]
fn frozen_bioshift_failed_placement_restores_removal_stack_and_replacement() {
    for definition in definitions() {
        let (mut game, from, to, spell) = setup(&definition, A);
        game.object_mut(to).unwrap().counters.insert(KIND, u32::MAX);
        announce(&mut game, spell, from, to, false);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(to, A,
            ironsmith::events::counters::matchers::WouldPutCountersMatcher::any(),
            ReplacementAction::Modify(EventModification::Multiply(2))));
        let mut dm = Amount { chosen: 1, ..Default::default() };
        assert!(resolve_stack_entry_with(&mut game, &mut dm).is_err());
        assert!(!dm.pending); assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Object(from), Target::Object(to)]);
        assert_eq!(game.stack[0].target_assignments.len(), 2);
        assert_eq!(game.counter_count(from, KIND), 4); assert_eq!(game.counter_count(to, KIND), u32::MAX);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        untouched_other_counters(&game, from, to);
        // A native retry with zero is legal and must not replay the failed removal.
        resolve_stack_entry_with(&mut game, &mut Amount::default()).unwrap();
        assert!(game.stack.is_empty()); assert_eq!(game.counter_count(from, KIND), 4);
        assert_eq!(game.counter_count(to, KIND), u32::MAX);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
    }
}
#[test]
fn frozen_bioshift_returned_endpoint_is_not_rebound_to_its_new_incarnation() {
    for definition in definitions() {
        for donor_returns in [false, true] {
            let (mut game, from, to, spell) = setup(&definition, A);
            announce(&mut game, spell, from, to, false);
            let old = if donor_returns { from } else { to };
            let stable = game.object(old).unwrap().stable_id;
            let departed = game.move_object_by_effect(old, Zone::Graveyard).unwrap();
            let returned = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
            assert_ne!(departed, old); assert_ne!(returned, old); assert_ne!(returned, departed);
            assert_eq!(game.object(returned).unwrap().stable_id, stable);
            assert!(game.object(old).is_none());
            // Give the new incarnation counters so an incorrect stable-ID rebind
            // would visibly mutate a currently legal matching-controller pair.
            game.object_mut(returned).unwrap().counters.insert(KIND, 7);
            game.object_mut(returned).unwrap().counters.insert(CounterType::Charge, 9);
            assert_eq!(game.stack[0].targets, vec![Target::Object(from), Target::Object(to)]);
            for mut branch in [game.clone(), game] {
                let mut dm = Amount { chosen: 1, ..Default::default() };
                resolve_stack_entry_with(&mut branch, &mut dm).unwrap();
                assert!(branch.stack.is_empty());
                assert!(dm.bounds.is_empty(), "a stale declared role must not bind its returned incarnation");
                assert_eq!(branch.counter_count(returned, KIND), 7);
                assert_eq!(branch.counter_count(returned, CounterType::Charge), 9);
                let unchanged = if donor_returns { to } else { from };
                assert_eq!(branch.counter_count(unchanged, KIND), if donor_returns { 0 } else { 4 });
                assert_eq!(branch.counter_count(unchanged, CounterType::Charge), if donor_returns { 5 } else { 3 });
            }
        }
    }
}
