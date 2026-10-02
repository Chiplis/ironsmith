use super::*;
use ironsmith::decisions::context::BooleanContext;

fn setup_game() -> GameState {
    GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
}

#[test]
fn suspended_draw_keeps_its_private_opening_and_hand_view_after_transaction_rollback() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    wasm.game = setup_game();
    let hidden = wasm
        .game
        .create_hidden_card_placeholder(alice, Zone::Library, 0, "draw".into());
    let physical = wasm.game.clone();
    let before = wasm.capture_crypto_audit_state();
    let drawn = wasm.game.draw_cards(alice, 1);
    assert_eq!(drawn.len(), 1);
    let mut dm = WasmReplayDecisionMaker::new(&[]);
    dm.decide_boolean(
        &wasm.game,
        &BooleanContext::new(alice, None, "Put a land onto the battlefield?"),
    );
    let (context, views, audit_views, pending_game) = dm.finish();
    wasm.game = physical;
    wasm.pending_decision = context;
    wasm.pending_decision_game = pending_game;
    wasm.active_viewed_cards = views;
    wasm.active_audit_viewed_cards = audit_views;
    wasm.update_crypto_requirements_from(before);
    assert!(!wasm.is_replay_checkpoint_boundary());
    assert!(
        wasm.game.player(alice).unwrap().hand.is_empty(),
        "the transaction remains uncommitted"
    );
    assert_eq!(
        wasm.game.player(alice).unwrap().library.as_slice(),
        &[hidden]
    );
    assert!(
        wasm.last_crypto_requirements
            .iter()
            .any(|r| r.requirement_type == "private_open"
                && r.viewer == Some(0)
                && r.slot == Some(0)
                && r.object_id == Some(drawn[0].0))
    );
    let snapshot: serde_json::Value =
        serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
    assert_eq!(snapshot["players"][0]["hand_size"], 1);
    assert_eq!(
        snapshot["players"][1]["hand_cards"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let savepoint = RuntimeSavepoint::capture(&wasm);
    wasm.pending_decision_game = None;
    wasm.last_crypto_requirements.clear();
    savepoint.restore(&mut wasm);
    assert_eq!(
        wasm.pending_decision_game
            .as_ref()
            .unwrap()
            .player(alice)
            .unwrap()
            .hand
            .len(),
        1
    );
    assert!(
        wasm.last_crypto_requirements
            .iter()
            .any(|r| r.requirement_type == "private_open")
    );

    // Replaying the transaction prefix for the next prompt must not ask for
    // another draw opening. The previous prompt is the audit boundary.
    let before_next = wasm.capture_crypto_audit_state();
    wasm.game = *wasm.pending_decision_game.take().unwrap();
    wasm.update_crypto_requirements_from(before_next);
    assert!(
        !wasm
            .last_crypto_requirements
            .iter()
            .any(|r| r.requirement_type == "private_open" || r.requirement_type == "hidden_move")
    );
}

#[test]
fn suspended_random_output_is_audited_once_across_prompts() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.game = setup_game();
    let physical = wasm.game.clone();
    let before = wasm.capture_crypto_audit_state();
    let mut values = [1, 2, 3];
    wasm.game.shuffle_slice(&mut values);
    let mut dm = WasmReplayDecisionMaker::new(&[]);
    dm.decide_boolean(
        &wasm.game,
        &BooleanContext::new(PlayerId::from_index(0), None, "Continue?"),
    );
    let (context, _, _, pending_game) = dm.finish();
    wasm.game = physical;
    wasm.pending_decision = context;
    wasm.pending_decision_game = pending_game;
    wasm.update_crypto_requirements_from(before);
    assert_eq!(
        wasm.last_crypto_requirements
            .iter()
            .filter(|r| r.requirement_type == "fair_random")
            .count(),
        1
    );
    let before_next = wasm.capture_crypto_audit_state();
    wasm.game = *wasm.pending_decision_game.take().unwrap();
    wasm.update_crypto_requirements_from(before_next);
    assert!(
        !wasm
            .last_crypto_requirements
            .iter()
            .any(|r| r.requirement_type == "fair_random")
    );
    assert_eq!(wasm.game.irreversible_random_count(), 1);
}
