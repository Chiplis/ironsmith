//! Source-authored native savepoint scenarios; intentionally unrun.
use super::*;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::grant_registry::{GrantSource, PlayFromConstraints};
use ironsmith_core::value_model::ManaSpendMode;

fn pending() -> (WasmGame, ObjectId, CastingMethod) {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    let player = PlayerId::from_index(0);
    wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    wasm.game.turn.active_player = player;
    wasm.game.turn.priority_player = Some(player);
    let source = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Exact source").card_types(vec![CardType::Enchantment]).build();
    let source = wasm.game.create_object_from_definition(&source, player, Zone::Battlefield);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Exact pending spell")
        .card_types(vec![CardType::Sorcery]).mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue])).build();
    let card = wasm.game.create_object_from_definition(&definition, player, Zone::Exile);
    wasm.game.player_mut(player).unwrap().mana_pool.red = 1;
    wasm.game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, player,
        PlayFromConstraints { cast_mana_spend_mode: ManaSpendMode::AnyColor, ..Default::default() },
        GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    let action = ironsmith::decision::compute_legal_actions(&wasm.game, player).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::ExactPermission { .. }, .. } if *spell_id == card)).unwrap();
    let LegalAction::CastSpell { casting_method: method, .. } = action.clone() else { unreachable!() };
    let progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut wasm.game, &mut wasm.trigger_queue,
        &mut wasm.priority_state, &PriorityResponse::PriorityAction(action), &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    if let GameProgress::NeedsDecisionCtx(context) = progress { wasm.pending_decision = Some(context); }
    let stack = wasm.priority_state.pending_cast.as_ref().unwrap().spell_id;
    assert!(wasm.game.object(stack).unwrap().cast_play_permission.is_some());
    (wasm, stack, method)
}

#[test]
fn exact_pending_authority_survives_root_replay_inactive_lane_and_suspended_host_savepoints() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, stack, method) = pending();
    let receipt = wasm.game.object(stack).unwrap().cast_play_permission.clone();
    wasm.pending_decision_game = Some(Box::new(wasm.game.clone()));
    wasm.pending_action_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.grand_melee_host_lanes.insert(7, GrandMeleeHostLane { runner: None, runner_awaiting_priority: true,
        trigger_queue: wasm.trigger_queue.clone(), priority_state: wasm.priority_state.clone() });
    wasm.suspended_subgame_hosts.push((None, true, wasm.trigger_queue.clone(), wasm.priority_state.clone(), wasm.grand_melee_host_lanes.clone()));
    let mut retained = RuntimeSavepoint::capture(&wasm);
    let copied = retained.clone();
    wasm.game.object_mut(stack).unwrap().cast_play_permission = None;
    wasm.priority_state.pending_cast = None;
    wasm.pending_decision_game = None;
    wasm.pending_action_checkpoint = None;
    wasm.grand_melee_host_lanes.clear();
    wasm.suspended_subgame_hosts.clear();
    retained.exchange(&mut wasm);
    assert_eq!(wasm.game.object(stack).unwrap().cast_play_permission, receipt);
    assert_eq!(wasm.priority_state.pending_cast.as_ref().unwrap().casting_method, method);
    assert_eq!(wasm.pending_decision_game.as_ref().unwrap().object(stack).unwrap().cast_play_permission, receipt);
    assert_eq!(wasm.pending_action_checkpoint.as_ref().unwrap().game.object(stack).unwrap().cast_play_permission, receipt);
    assert_eq!(wasm.grand_melee_host_lanes[&7].priority_state.pending_cast.as_ref().unwrap().casting_method, method);
    assert_eq!(wasm.suspended_subgame_hosts[0].3.pending_cast.as_ref().unwrap().casting_method, method);
    assert_eq!(wasm.suspended_subgame_hosts[0].4[&7].priority_state.pending_cast.as_ref().unwrap().casting_method, method);
    retained.exchange(&mut wasm);
    assert!(wasm.game.object(stack).unwrap().cast_play_permission.is_none());
    copied.restore(&mut wasm);
    assert_eq!(wasm.game.object(stack).unwrap().cast_play_permission, receipt);
    assert_eq!(wasm.priority_state.pending_cast.as_ref().unwrap().casting_method, method);
    let policy = wasm.game.mana_spend_policy_for_cast(PlayerId::from_index(0), Some(stack));
    assert!(wasm.game.can_pay_mana_cost_with_policy(PlayerId::from_index(0), Some(stack),
        &ManaCost::from_symbols(vec![ManaSymbol::Blue]), 0, ironsmith::costs::PaymentReason::CastSpell, &policy));
}
