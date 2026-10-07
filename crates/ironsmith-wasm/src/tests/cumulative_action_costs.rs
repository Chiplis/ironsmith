//! Exercise the existing resolution replay owner, including its cancellation
//! boundary and separate private prompt state. This is not a claim about the
//! unrelated private casting-cost transport.
use super::*;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};

#[test]
fn psychic_vortex_draw_payment_replays_accepted_choice_without_cancel_or_public_disclosure() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../../fixtures/cumulative_action_costs.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Psychic Vortex").unwrap();
    let definition = ironsmith_registry_test::compile_to_runtime_definition(
        "Psychic Vortex", row["text"].as_str().unwrap(), false,
    ).unwrap();
    let source = wasm.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for n in 0..3 {
        let definition = ironsmith_registry_test::compile_to_runtime_definition(&format!("Private draw {n}"), "Type: Land", false).unwrap();
        wasm.game.create_object_from_definition(&definition, alice, Zone::Library);
    }
    wasm.game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, ironsmith::events::cards::matchers::WouldDrawCardMatcher::you(),
        ReplacementAction::Additionally(vec![ironsmith::Effect::may(vec![ironsmith::Effect::gain_life(2)])]),
    ));
    wasm.game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::phase::BeginningOfUpkeepEvent::new(alice), Default::default(),
    ));
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut wasm.game, &mut wasm.trigger_queue, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(wasm.game.stack.len(), 1);
    let checkpoint = wasm.capture_replay_checkpoint();
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Boolean(true)]);
    ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut dm).unwrap();
    let (pending, viewed, audit, pending_game) = dm.finish();
    assert!(matches!(pending, Some(DecisionContext::Boolean(_))));
    assert_eq!(wasm.game.player(alice).unwrap().hand.len(), 0, "native game is rolled back while a replacement choice is pending");
    assert_eq!(pending_game.as_ref().unwrap().player(alice).unwrap().hand.len(), 1, "the private prompt state retains the drawn card");
    assert!(viewed.is_none(), "an ordinary draw publishes no viewed-card window");
    assert!(audit.is_empty(), "an ordinary draw publishes no public card opening");
    wasm.pending_decision = pending;
    wasm.pending_decision_game = pending_game;
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: vec![ReplayDecisionAnswer::Boolean(true)],
    });
    assert!(!wasm.is_cancelable(), "accepted upkeep cannot be undone after learning a drawn card");
    let mut replay = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Boolean(true), ReplayDecisionAnswer::Boolean(false)]);
    ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut replay).unwrap();
    assert!(replay.finish().0.is_none());
    assert_eq!(wasm.game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(wasm.game.player(alice).unwrap().library.len(), 2);
    assert_eq!(wasm.game.counter_count(source, ironsmith::CounterType::Age), 1);
    assert!(wasm.game.battlefield.contains(&source));
}
