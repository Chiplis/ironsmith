use super::*;
use ironsmith::game_state::{Phase, Step};
use ironsmith::turn_runner::{TurnRunner, TurnState};
use ironsmith_registry_test::compile_to_runtime_definition;

fn card(name: &str, text: &str) -> ironsmith::CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap_or_else(|error| panic!("{name}: {error:?}"))
}

/// Eleven Exalted Sunborns and Followed Footsteps: the upkeep copy becomes
/// 2^11 = 2,048 tokens. Times every dispatch the browser would make.
#[test]
#[ignore = "timing probe; run with --ignored --nocapture"]
fn sunborn_footsteps_upkeep_probe() {
    let _guard = crate::test_id_counter_guard();
    let count: usize = std::env::var("SUNBORNS").ok().and_then(|v| v.parse().ok()).unwrap_or(11);
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let sunborn = card(
        "Exalted Sunborn",
        "Mana cost: {3}{W}{W}\nType: Creature — Angel Wizard\nPower/Toughness: 4/5\nFlying, lifelink\nIf one or more tokens would be created under your control, twice that many of those tokens are created instead.\nWarp {1}{W} (You may cast this card from your hand for its warp cost. Exile this creature at the beginning of the next end step, then you may cast it from exile on a later turn.)",
    );
    let footsteps = card(
        "Followed Footsteps",
        "Mana cost: {3}{U}{U}\nType: Enchantment — Aura\nEnchant creature\nAt the beginning of your upkeep, create a token that's a copy of enchanted creature.",
    );
    let first = wasm.game.create_object_from_definition(&sunborn, alice, Zone::Battlefield);
    for _ in 1..count {
        wasm.game.create_object_from_definition(&sunborn, alice, Zone::Battlefield);
    }
    let aura = wasm.game.create_object_from_definition(&footsteps, alice, Zone::Battlefield);
    assert!(wasm.game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(first)));
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Upkeep);
    wasm.game.refresh_continuous_state().unwrap();
    wasm.runner = Some(TurnRunner::from_state_for_sync(TurnState::Upkeep));

    let t = std::time::Instant::now();
    wasm.advance_until_decision().expect("upkeep trigger reaches priority");
    eprintln!("advance to priority: {:?} stack={}", t.elapsed(), wasm.game.stack.len());
    let t = std::time::Instant::now();
    wasm.snapshot_json().expect("snapshot");
    eprintln!("snapshot: {:?}", t.elapsed());

    let before = wasm.game.battlefield.len();
    for step in 0..6 {
        if wasm.game.stack.is_empty() && step > 0 {
            break;
        }
        let t = std::time::Instant::now();
        pass(&mut wasm);
        eprintln!(
            "pass #{step}: {:?} stack={} battlefield={}",
            t.elapsed(),
            wasm.game.stack.len(),
            wasm.game.battlefield.len()
        );
        let t = std::time::Instant::now();
        wasm.snapshot_json().expect("snapshot");
        eprintln!("snapshot: {:?}", t.elapsed());
    }
    eprintln!("created {}", wasm.game.battlefield.len() - before);
    let effects = wasm.game.all_continuous_effects();
    eprintln!("continuous effects after: {}", effects.len());
    for effect in effects.iter().take(5) { eprintln!("  {:?} -> {:?}", effect.source, std::mem::discriminant(&effect.modification)); }
}

fn pass(wasm: &mut WasmGame) {
    let index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PassPriority))
            .expect("pass priority is offered"),
        other => panic!("expected priority, got {other:?}"),
    };
    wasm.dispatch_typed_command(
        crate::UiCommand::PriorityAction { action_index: Some(index), action_ref: None },
        0.0,
    )
    .expect("pass priority");
}

