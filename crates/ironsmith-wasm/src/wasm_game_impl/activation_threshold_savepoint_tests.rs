//! Authored, unrun. Gameplay recovery uses native branches or genesis replay.
use ironsmith::AbilityKind;
use super::*;
use ironsmith::effects::EffectContext as ExecutionContext;
fn definition(name: &str) -> ironsmith::cards::CardDefinition {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../../fixtures/activation_threshold_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|r| r["name"] == name).unwrap();
    ironsmith_registry_test::compile_to_runtime_definition(name, row["text"].as_str().unwrap(), false).unwrap()
}
fn pay(wasm: &mut WasmGame, source: ObjectId, player: PlayerId) {
    wasm.game.player_mut(player).unwrap().mana_pool.colorless += 1;
    ironsmith::special_actions::perform_activate_mana_ability(&mut wasm.game, player, source, 0, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
}
fn total(wasm: &WasmGame, source: ObjectId) -> u32 {
    let ability = wasm.game.current_ability(source, 0).unwrap();
    let AbilityKind::Activated(ability) = &ability.kind else { panic!("activation") };
    wasm.game.turn_store.turn_history.ability_activation_counts.as_ref().unwrap()
        .get(&(source, ironsmith::continuous::AbilityOrigin::Printed(0), ability.effects.activation_definition)).copied().unwrap_or(0)
}
#[test]
fn root_and_inactive_turn_lanes_keep_native_counts_without_a_wire_recovery_path() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    let seats: Vec<_> = (0..8).map(PlayerId::from_index).collect();
    wasm.game = GameState::new((0..8).map(|n| format!("P{n}")).collect(), 20);
    wasm.game.restore_grand_melee(seats).unwrap();
    let markers = wasm.game.grand_melee_marker_views();
    let root = markers[0].number; let other = markers[1].number;
    let a = markers[0].holder; let b = markers[1].holder;
    let first = wasm.game.create_object_from_definition(&definition("Farrelite Priest"), a, Zone::Battlefield);
    let second = wasm.game.create_object_from_definition(&definition("Initiates of the Ebon Hand"), b, Zone::Battlefield);
    pay(&mut wasm, first, a);
    wasm.game.select_grand_melee_turn_marker(other).unwrap();
    for _ in 0..3 { pay(&mut wasm, second, b); }
    wasm.game.select_grand_melee_turn_marker(root).unwrap();
    let saved = RuntimeSavepoint::capture(&wasm);
    assert_eq!(total(&wasm, first), 1); assert_eq!(total(&wasm, second), 0);
    wasm.game.turn_store.turn_history.ability_activation_counts = None;
    wasm.game.select_grand_melee_turn_marker(other).unwrap();
    assert_eq!(total(&wasm, second), 3);
    wasm.game.turn_store.turn_history.ability_activation_counts = None;
    saved.restore(&mut wasm);
    assert_eq!(total(&wasm, first), 1);
    wasm.game.select_grand_melee_turn_marker(other).unwrap();
    assert_eq!(total(&wasm, second), 3);
    pay(&mut wasm, second, b); assert_eq!(total(&wasm, second), 4);
    assert!(wasm.game.effect_store.delayed_triggers.iter().any(|trigger| trigger.controller == b));
    // Public audit v8 is a projection, never an importer or gameplay state owner.
    assert_eq!(serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap()["version"], 8);
}
#[test]
fn serialized_claim_snapshot_cannot_reconstruct_an_activation_acquisition() {
    let _ids = crate::test_id_counter_guard(); let mut wasm = WasmGame::new();
    wasm.game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = wasm.game.create_object_from_definition(&definition("Farrelite Priest"), PlayerId(0), Zone::Battlefield);
    for _ in 0..4 { pay(&mut wasm, source, PlayerId(0)); }
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(wasm.game.object(source).unwrap(), &wasm.game);
    assert!(snapshot.ability_origins.is_some());
    let wire = serde_json::to_value(&snapshot).unwrap();
    let restored: ironsmith::snapshot::ObjectSnapshot = serde_json::from_value(wire).unwrap();
    assert!(restored.ability_origins.is_none());
    let ctx = ExecutionContext::new_default(source, PlayerId(0)).with_ability_index(0).with_source_snapshot(restored);
    assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&wasm.game,
        &ironsmith::ConditionExpr::ThisAbilityActivatedThisTurnAtLeast(4), &ctx),
        Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
}
