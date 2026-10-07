//! Authored, UNRUN. Exact native event receipts are never wire recovery input.
use super::*;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::events::PlayerAttackDeclarationEvent;

fn receipt(queue: &TriggerQueue) -> &PlayerAttackDeclarationEvent {
    queue.entries[0].triggering_event.downcast::<PlayerAttackDeclarationEvent>().unwrap()
}

#[test]
fn completed_declaration_survives_root_exchange_replay_root_and_inactive_host_queue() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into(), "C".into()], 20, 1);
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/combat_participant_conditions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Ever-Watching Threshold").unwrap();
    let definition = ironsmith_registry_test::compile_to_runtime_definition(
        "Ever-Watching Threshold", row["text"].as_str().unwrap(), false).unwrap();
    wasm.game.create_object_from_definition(&definition, a, Zone::Battlefield);
    let creature = ironsmith_registry_test::compile_to_runtime_definition(
        "Attacker", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
    let attacker = wasm.game.create_object_from_definition(&creature, b, Zone::Battlefield);
    wasm.game.remove_summoning_sickness(attacker);
    wasm.game.turn.active_player = b;
    wasm.game.turn.phase = ironsmith::game_state::Phase::Combat;
    wasm.game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    wasm.game.mark_combat_phase_started();
    ironsmith::game_loop::apply_attacker_declarations(&mut wasm.game, &mut CombatState::default(),
        &mut wasm.trigger_queue, &[AttackerDeclaration { creature: attacker, target: AttackTarget::Player(a) }]).unwrap();
    assert_eq!(wasm.trigger_queue.entries.len(), 1);
    wasm.pending_decision_game = Some(Box::new(wasm.game.clone()));
    wasm.pending_action_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.grand_melee_host_lanes.insert(7, GrandMeleeHostLane { runner: None, runner_awaiting_priority: true,
        trigger_queue: wasm.trigger_queue.clone(), priority_state: wasm.priority_state.clone() });
    let mut saved = RuntimeSavepoint::capture(&wasm);
    let clone = saved.clone();
    wasm.trigger_queue.entries.clear();
    wasm.grand_melee_host_lanes.clear();
    wasm.pending_decision_game = None;
    wasm.pending_action_checkpoint = None;
    saved.exchange(&mut wasm);
    for queue in [&wasm.trigger_queue, &wasm.grand_melee_host_lanes[&7].trigger_queue] {
        let event = receipt(queue);
        assert_eq!(event.attacker, b);
        assert_eq!(event.defender, a);
        assert!(event.directly_attacked_player);
        let declaration = event.declaration.as_ref().unwrap();
        assert_eq!(declaration.len(), 1);
        assert_eq!(declaration[0].creature, attacker);
        assert_eq!(declaration[0].controller, b);
        assert_eq!(declaration[0].defending_player, a);
    }
    assert!(wasm.pending_action_checkpoint.as_ref().unwrap().game.turn_store.turn_history.event_records.iter()
        .any(|record| record.event.downcast::<PlayerAttackDeclarationEvent>()
            .is_some_and(|event| event.declaration.is_some())));
    saved.exchange(&mut wasm);
    assert!(wasm.trigger_queue.entries.is_empty());
    clone.restore(&mut wasm);
    assert_eq!(receipt(&wasm.trigger_queue).declaration.as_ref().unwrap()[0].creature, attacker);
}
