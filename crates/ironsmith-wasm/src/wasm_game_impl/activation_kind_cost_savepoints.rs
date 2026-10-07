//! Authored and UNRUN. Native continuations retain the actual priced ability;
//! public audit projections are not gameplay recovery containers.
use super::*;
use ironsmith_core::ActivatedAbilityKeyword;

fn pending_kind_activation() -> (WasmGame, ObjectId) {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    let player = PlayerId::from_index(0);
    wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    wasm.game.turn.active_player = player;
    wasm.game.turn.priority_player = Some(player);
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/activation_kind_costs.json.fixture"
    )).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Boom Scholar").unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap());
    let scholar = ironsmith_registry_test::compile_to_runtime_definition("Boom Scholar", &text, false).unwrap();
    wasm.game.create_object_from_definition(&scholar, player, Zone::Battlefield);
    let source = ironsmith_registry_test::compile_to_runtime_definition("Native exhaust witness",
        "Type: Artifact\nExhaust — {X}: Draw a card. (Activate each exhaust ability only once.)", false).unwrap();
    let source = wasm.game.create_object_from_definition(&source, player, Zone::Battlefield);
    wasm.game.player_mut(player).unwrap().mana_pool.colorless = 3;
    let action = ironsmith::decision::compute_legal_actions(&wasm.game, player).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
    let progress = ironsmith::game_loop::apply_priority_response_with_dm(&mut wasm.game,
        &mut wasm.trigger_queue, &mut wasm.priority_state, &PriorityResponse::PriorityAction(action),
        &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    let GameProgress::NeedsDecisionCtx(context @ DecisionContext::Number(_)) = progress else {
        panic!("expected announced X with retained ability");
    };
    wasm.pending_decision = Some(context);
    assert_eq!(retained(&wasm.priority_state).0, Some(ActivatedAbilityKeyword::Exhaust));
    (wasm, source)
}

fn retained(state: &PriorityLoopState) -> (Option<ActivatedAbilityKeyword>, String, Option<PlayerId>, Option<usize>) {
    let owner = state.pending_activation.as_ref().unwrap().announced_cost.as_ref().unwrap();
    assert_eq!(owner.ability.keyword, owner.facts.keyword);
    (owner.facts.keyword, owner.ability.mana_cost.display(), owner.facts.activator, owner.facts.ability_index)
}

#[test]
fn pending_original_and_pricing_facts_survive_native_root_lane_and_branch_exchange() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, source) = pending_kind_activation();
    let original = retained(&wasm.priority_state);
    wasm.pending_action_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.grand_melee_host_lanes.insert(7, GrandMeleeHostLane {
        runner: None, runner_awaiting_priority: true,
        trigger_queue: wasm.trigger_queue.clone(), priority_state: wasm.priority_state.clone(),
    });
    wasm.suspended_subgame_hosts.push((None, true, wasm.trigger_queue.clone(),
        wasm.priority_state.clone(), wasm.grand_melee_host_lanes.clone()));
    let mut saved = RuntimeSavepoint::capture(&wasm);
    let copied = saved.clone();
    wasm.priority_state.pending_activation.as_mut().unwrap().announced_cost = None;
    wasm.pending_action_checkpoint = None;
    wasm.grand_melee_host_lanes.clear();
    wasm.suspended_subgame_hosts.clear();
    wasm.game.player_mut(PlayerId::from_index(0)).unwrap().mana_pool.colorless = 0;
    saved.exchange(&mut wasm);
    assert_eq!(retained(&wasm.priority_state), original);
    assert_eq!(retained(&wasm.pending_action_checkpoint.as_ref().unwrap().priority_state), original);
    assert_eq!(retained(&wasm.grand_melee_host_lanes[&7].priority_state), original);
    assert_eq!(retained(&wasm.suspended_subgame_hosts[0].3), original);
    assert_eq!(retained(&wasm.suspended_subgame_hosts[0].4[&7].priority_state), original);
    assert!(wasm.game.object(source).is_some());
    assert_eq!(wasm.game.player(PlayerId::from_index(0)).unwrap().mana_pool.colorless, 3);
    saved.exchange(&mut wasm);
    assert!(wasm.priority_state.pending_activation.as_ref().unwrap().announced_cost.is_none());
    copied.restore(&mut wasm);
    assert_eq!(retained(&wasm.priority_state), original);
    assert!(matches!(wasm.pending_decision, Some(DecisionContext::Number(_))));
}
