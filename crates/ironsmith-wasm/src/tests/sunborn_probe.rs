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
    let mut wasm = sunborn_footsteps_game(count);

    run_probe(&mut wasm);
}

/// Eleven doublers propose 2,048 same-name copies in one batch; the engine
/// creates 512 of them.
#[test]
fn same_name_token_batch_is_capped_at_512() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = sunborn_footsteps_game(11);
    wasm.advance_until_decision().expect("upkeep trigger reaches priority");
    let before = wasm.game.battlefield.len();
    while !wasm.game.stack.is_empty() {
        pass(&mut wasm);
    }
    assert_eq!(wasm.game.battlefield.len() - before, 512);
}

fn sunborn_footsteps_game(count: usize) -> WasmGame {
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
    wasm
}

fn run_probe(wasm: &mut WasmGame) {
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
        pass(wasm);
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
    if matches!(wasm.pending_decision.as_ref(), Some(DecisionContext::Attackers(_))) {
        wasm.dispatch_typed_command(crate::UiCommand::DeclareAttackers { declarations: Vec::new(), bands: Vec::new() }, 0.0)
            .expect("declare no attackers");
        return;
    }
    if matches!(wasm.pending_decision.as_ref(), Some(DecisionContext::Blockers(_))) {
        wasm.dispatch_typed_command(crate::UiCommand::DeclareBlockers { declarations: Vec::new() }, 0.0)
            .expect("declare no blockers");
        return;
    }
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


/// Two consecutive upkeeps: the second Followed Footsteps trigger sees the
/// 512 token copies from the first as 512 more doublers. Passes priority
/// through the whole turn cycle in between, timing every dispatch.
#[test]
#[ignore = "timing probe; run with --ignored --nocapture"]
fn sunborn_footsteps_second_batch_probe() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = sunborn_footsteps_game(11);
    let plains = card("Plains", "Type: Basic Land — Plains\n({T}: Add {W}.)");
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for _ in 0..20 {
            wasm.game.create_object_from_definition(&plains, player, Zone::Library);
        }
    }
    let t = std::time::Instant::now();
    wasm.advance_until_decision().expect("upkeep trigger reaches priority");
    eprintln!("advance: {:?}", t.elapsed());
    let start = wasm.game.turn.turn_number;
    let mut slow = std::time::Duration::ZERO;
    for _ in 0..400 {
        let before = wasm.game.battlefield.len();
        let (turn, step) = (wasm.game.turn.turn_number, wasm.game.turn.step);
        let t = std::time::Instant::now();
        pass(&mut wasm);
        let elapsed = t.elapsed();
        let t2 = std::time::Instant::now();
        wasm.snapshot_json().expect("snapshot");
        let snap = t2.elapsed();
        slow += elapsed + snap;
        if elapsed.as_millis() > 20 || snap.as_millis() > 20 || wasm.game.battlefield.len() != before {
            eprintln!("turn {turn} {step:?} pass {elapsed:?} snapshot {snap:?} battlefield {} stack {}", wasm.game.battlefield.len(), wasm.game.stack.len());
        }
        if wasm.game.turn.turn_number >= start + 2 && wasm.game.stack.is_empty() && wasm.game.battlefield.len() > 600 {
            break;
        }
    }
    eprintln!("total dispatch+snapshot: {slow:?}");
}

/// Times what the browser does after each Followed Footsteps batch: the
/// resolving dispatch (deferred priority analysis, as in the browser) and the
/// background priority analysis over every source on the board.
#[test]
#[ignore = "timing probe; run with --ignored --nocapture"]
fn sunborn_footsteps_analysis_probe() {
    let _guard = crate::test_id_counter_guard();
    let batches: usize = std::env::var("BATCHES").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
    let analyse_from: usize = std::env::var("ANALYSE_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let previous = ironsmith::game_loop::priority_analysis_deferred();
    let mut wasm = sunborn_footsteps_game(11);
    wasm.set_deferred_priority_analysis(true);
    let plains = card("Plains", "Type: Basic Land — Plains\n({T}: Add {W}.)");
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for _ in 0..20 {
            wasm.game.create_object_from_definition(&plains, player, Zone::Library);
        }
    }
    wasm.advance_until_decision().expect("upkeep trigger reaches priority");
    for batch in 1..=batches {
        // Pass until the trigger resolves (batch 1 is already on the stack).
        let mut resolve = std::time::Duration::ZERO;
        loop {
            let had_stack = !wasm.game.stack.is_empty();
            let before = wasm.game.battlefield.len();
            let t = std::time::Instant::now();
            pass(&mut wasm);
            if had_stack {
                resolve += t.elapsed();
            }
            if wasm.game.battlefield.len() > before + 100 {
                break;
            }
        }
        eprintln!("batch {batch}: resolve dispatches {resolve:?} battlefield {}", wasm.game.battlefield.len());
        if batch >= analyse_from {
            let t = std::time::Instant::now();
            assert!(wasm.begin_priority_analysis("probe".into()), "priority decision is pending");
            let mut steps = 0usize;
            loop {
                steps += 1;
                match wasm.advance_priority_analysis_step("probe", 8).expect("analysis step") {
                    Some(step) if step.finished => break,
                    Some(_) => {}
                    None => break,
                }
            }
            eprintln!("batch {batch}: priority analysis {:?} over {steps} steps", t.elapsed());
        }
        if batch < batches {
            // Pass to the next upkeep trigger.
            while wasm.game.stack.is_empty() {
                pass(&mut wasm);
            }
        }
    }
    ironsmith::game_loop::set_priority_analysis_deferred(previous);
}

/// CR 614.12: permanents entering simultaneously don't apply their
/// "enters with" replacements to each other. Two doubled copies of a
/// counter-granting creature each get one counter, from the original only.
#[test]
fn simultaneous_token_copies_do_not_apply_entry_replacements_to_each_other() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let sunborn = card(
        "Exalted Sunborn",
        "Mana cost: {3}{W}{W}\nType: Creature — Angel Wizard\nPower/Toughness: 4/5\nFlying, lifelink\nIf one or more tokens would be created under your control, twice that many of those tokens are created instead.",
    );
    let counter_giver = card(
        "Counter Giver",
        "Mana cost: {2}{G}\nType: Creature — Beast\nPower/Toughness: 2/2\nEach other creature you control enters with an additional +1/+1 counter on it.",
    );
    let footsteps = card(
        "Followed Footsteps",
        "Mana cost: {3}{U}{U}\nType: Enchantment — Aura\nEnchant creature\nAt the beginning of your upkeep, create a token that's a copy of enchanted creature.",
    );
    wasm.game.create_object_from_definition(&sunborn, alice, Zone::Battlefield);
    let giver = wasm.game.create_object_from_definition(&counter_giver, alice, Zone::Battlefield);
    let aura = wasm.game.create_object_from_definition(&footsteps, alice, Zone::Battlefield);
    assert!(wasm.game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(giver)));
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Upkeep);
    wasm.game.refresh_continuous_state().unwrap();
    wasm.runner = Some(TurnRunner::from_state_for_sync(TurnState::Upkeep));
    wasm.advance_until_decision().expect("upkeep trigger reaches priority");
    let before: std::collections::HashSet<_> = wasm.game.battlefield.iter().copied().collect();
    while !wasm.game.stack.is_empty() {
        pass(&mut wasm);
    }
    let counters: Vec<u32> = wasm
        .game
        .battlefield
        .iter()
        .filter(|id| !before.contains(id))
        .map(|id| {
            wasm.game
                .object(*id)
                .and_then(|object| object.counters.get(&ironsmith::object::CounterType::PlusOnePlusOne).copied())
                .unwrap_or(0)
        })
        .collect();
    assert_eq!(counters, vec![1, 1], "each copy gets only the original's counter");
}

/// An upkeep with `cards` on Alice's battlefield and a "create two copies of
/// enchanted creature" Aura on `cards[enchanted]`, passed until the stack is
/// empty. Returns each new permanent's name and +1/+1 counters.
fn twin_copy_upkeep(cards: &[(&str, &str)], enchanted: usize) -> Vec<(String, u32)> {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let ids: Vec<_> = cards
        .iter()
        .map(|(name, text)| wasm.game.create_object_from_definition(&card(name, text), alice, Zone::Battlefield))
        .collect();
    let aura = card(
        "Twin Footsteps",
        "Mana cost: {3}{U}{U}\nType: Enchantment — Aura\nEnchant creature\nAt the beginning of your upkeep, create two tokens that are copies of enchanted creature.",
    );
    let aura = wasm.game.create_object_from_definition(&aura, alice, Zone::Battlefield);
    assert!(wasm.game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(ids[enchanted])));
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::Beginning;
    wasm.game.turn.step = Some(Step::Upkeep);
    wasm.game.refresh_continuous_state().unwrap();
    wasm.runner = Some(TurnRunner::from_state_for_sync(TurnState::Upkeep));
    wasm.advance_until_decision().expect("upkeep trigger reaches priority");
    let before: std::collections::HashSet<_> = wasm.game.battlefield.iter().copied().collect();
    while !wasm.game.stack.is_empty() {
        pass(&mut wasm);
    }
    let mut created: Vec<(String, u32)> = wasm
        .game
        .battlefield
        .iter()
        .filter(|id| !before.contains(id))
        .filter_map(|id| wasm.game.object(*id))
        .map(|object| {
            (
                object.name.to_string(),
                object.counters.get(&ironsmith::object::CounterType::PlusOnePlusOne).copied().unwrap_or(0),
            )
        })
        .collect();
    created.sort();
    created
}

const BEAR: (&str, &str) = ("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");

/// A replacement that applies to every copy is matched for each of them,
/// not only the first.
#[test]
fn grouped_token_copies_each_receive_an_applying_entry_replacement() {
    let _guard = crate::test_id_counter_guard();
    let giver = (
        "Counter Giver",
        "Mana cost: {2}{G}\nType: Creature — Beast\nPower/Toughness: 2/2\nEach other creature you control enters with an additional +1/+1 counter on it.",
    );
    assert_eq!(
        twin_copy_upkeep(&[BEAR, giver], 0),
        vec![("Grizzly Bears".to_string(), 1), ("Grizzly Bears".to_string(), 1)],
    );
}

/// Tokens a replacement adds are not members of the copies' group: an
/// entry replacement that applies only to them still applies, although the
/// copies matched none.
#[test]
fn replacement_added_tokens_are_matched_apart_from_the_grouped_copies() {
    let _guard = crate::test_id_counter_guard();
    let squirrel_maker = (
        "Squirrel Maker",
        "Mana cost: {2}{B}{G}\nType: Creature — Squirrel Warrior\nPower/Toughness: 3/3\nIf one or more tokens would be created under your control, those tokens plus that many 1/1 green Squirrel creature tokens are created instead.",
    );
    let patron = (
        "Squirrel Patron",
        "Mana cost: {1}{G}\nType: Creature — Elf\nPower/Toughness: 1/1\nEach other Squirrel you control enters with an additional +1/+1 counter on it.",
    );
    assert_eq!(
        twin_copy_upkeep(&[BEAR, squirrel_maker, patron], 0),
        vec![
            ("Grizzly Bears".to_string(), 0),
            ("Grizzly Bears".to_string(), 0),
            ("Squirrel Token".to_string(), 1),
            ("Squirrel Token".to_string(), 1),
        ],
    );
}
