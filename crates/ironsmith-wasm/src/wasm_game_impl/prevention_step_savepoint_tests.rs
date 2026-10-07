//! Authored, UNRUN. Step evidence is native state, never synthesized from public JSON.
use super::*;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;

#[test]
fn attacked_step_evidence_survives_native_roots_replay_checkpoint_and_exchange() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Declaration witness")
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
    let attacker = wasm.game.create_object_from_card(&card, b, Zone::Battlefield);
    wasm.game.remove_summoning_sickness(attacker);
    wasm.game.turn.active_player = b;
    wasm.game.turn.phase = ironsmith::game_state::Phase::Combat;
    wasm.game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    wasm.game.mark_combat_phase_started();
    ironsmith::game_loop::apply_attacker_declarations(&mut wasm.game, &mut CombatState::default(),
        &mut wasm.trigger_queue, &[AttackerDeclaration { creature: attacker, target: AttackTarget::Player(a) }]).unwrap();
    wasm.pending_decision_game = Some(Box::new(wasm.game.clone()));
    wasm.pending_action_checkpoint = Some(wasm.capture_replay_checkpoint());
    let mut saved = RuntimeSavepoint::capture(&wasm);
    let cloned = saved.clone();
    wasm.game.combat.as_mut().unwrap().last_attack_declaration_step_players = None;
    wasm.pending_decision_game = None;
    wasm.pending_action_checkpoint = None;
    saved.exchange(&mut wasm);
    let expected = Some([a].into_iter().collect());
    assert_eq!(wasm.game.combat.as_ref().unwrap().last_attack_declaration_step_players, expected);
    assert_eq!(wasm.pending_decision_game.as_ref().unwrap().combat.as_ref().unwrap().last_attack_declaration_step_players, expected);
    assert_eq!(wasm.pending_action_checkpoint.as_ref().unwrap().game.combat.as_ref().unwrap().last_attack_declaration_step_players, expected);
    saved.exchange(&mut wasm);
    assert!(wasm.game.combat.as_ref().unwrap().last_attack_declaration_step_players.is_none());
    cloned.restore(&mut wasm);
    assert_eq!(wasm.game.combat.as_ref().unwrap().last_attack_declaration_step_players, expected);
}
