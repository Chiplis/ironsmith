use super::*;
use crate::ReplayDecisionAnswer;
use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::AttackerDeclaration;
use ironsmith::game_state::{Phase, Step};
use ironsmith::turn_runner::{TurnRunner, TurnState};
use ironsmith_registry_test::cards::definitions::{basic_island, basic_mountain, ornithopter};
use ironsmith_registry_test::compile_to_runtime_definition;

fn attacking_kavu(graveyard_card_zone: Zone) -> (WasmGame, ObjectId, ObjectId) {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let kavu = compile_to_runtime_definition(
        "Territorial Kavu",
        "Type: Creature — Kavu\nPower/Toughness: */*\nDomain — This creature's power and toughness are each equal to the number of basic land types among lands you control.\nWhenever this creature attacks, choose one —\n• Discard a card. If you do, draw a card.\n• Exile up to one target card from a graveyard.",
        false,
    )
    .expect("Territorial Kavu should compile");
    let kavu_id = wasm
        .game
        .create_object_from_definition(&kavu, alice, Zone::Battlefield);
    wasm.game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);
    let graveyard_id =
        wasm.game
            .create_object_from_definition(&ornithopter(), bob, graveyard_card_zone);
    let hand_id = wasm
        .game
        .create_object_from_definition(&ornithopter(), alice, Zone::Hand);
    wasm.game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
    wasm.game
        .create_object_from_definition(&basic_island(), alice, Zone::Library);
    wasm.game.remove_summoning_sickness(kavu_id);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::Combat;
    wasm.game.turn.step = Some(Step::DeclareAttackers);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareAttackersApply);
    runner.respond_attackers(vec![AttackerDeclaration {
        creature: kavu_id,
        target: AttackTarget::Player(bob),
    }]);
    wasm.runner = Some(runner);
    wasm.advance_until_decision()
        .expect("declaring Kavu as an attacker should reach its mode decision");
    (wasm, graveyard_id, hand_id)
}

fn mode_checkpoint(wasm: &WasmGame) -> crate::ReplayCheckpoint {
    let ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected Kavu's mode choice before priority, got {other:?}"),
    };
    assert_eq!(ctx.options.len(), 2);
    assert!(ctx.options[0].description.contains("Discard a card"));
    assert!(ctx.options[1].description.contains("Exile up to one"));
    assert!(
        wasm.game.stack.is_empty(),
        "choose the mode before stacking"
    );
    wasm.pending_replay_action
        .as_ref()
        .expect("the attack trigger should use a replayable UI decision")
        .checkpoint
        .clone()
}

#[test]
fn territorial_kavu_attack_surfaces_modes_then_optional_graveyard_target() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, graveyard_id, hand_id) = attacking_kavu(Zone::Graveyard);
    let graveyard_stable_id = wasm.game.object(graveyard_id).unwrap().stable_id;
    let checkpoint = mode_checkpoint(&wasm);
    let chosen_mode = ReplayDecisionAnswer::Options(vec![1]);
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[chosen_mode.clone()])
        .expect("selecting exile mode should ask for targets");
    let ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected the exile mode's target decision, got {other:?}"),
    };
    assert_eq!(ctx.requirements.len(), 1);
    assert_eq!(ctx.requirements[0].min_targets, 0);
    assert!(
        ctx.requirements[0]
            .legal_targets
            .contains(&Target::Object(graveyard_id))
    );
    let outcome = wasm
        .execute_with_replay(
            &checkpoint,
            &ReplayRoot::Advance,
            &[
                chosen_mode,
                ReplayDecisionAnswer::Targets(vec![Target::Object(graveyard_id)]),
            ],
        )
        .expect("choosing a graveyard target should stack the trigger");
    assert!(matches!(
        outcome,
        ReplayOutcome::Complete(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_)))
    ));
    assert_eq!(wasm.game.stack.len(), 1);
    assert_eq!(wasm.game.stack[0].chosen_modes.as_deref(), Some(&[1][..]));
    assert_eq!(
        wasm.game.stack[0].targets,
        vec![Target::Object(graveyard_id)]
    );
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game)
        .expect("the exile mode should resolve");
    let exiled_id = wasm
        .game
        .find_object_by_stable_id(graveyard_stable_id)
        .unwrap();
    assert_eq!(wasm.game.object(exiled_id).unwrap().zone, Zone::Exile);
    assert_eq!(wasm.game.object(hand_id).unwrap().zone, Zone::Hand);
}

#[test]
fn territorial_kavu_exile_mode_accepts_no_target_including_empty_graveyards() {
    let _guard = crate::test_id_counter_guard();
    for graveyard_card_zone in [Zone::Graveyard, Zone::Exile] {
        let (mut wasm, graveyard_id, hand_id) = attacking_kavu(graveyard_card_zone);
        let checkpoint = mode_checkpoint(&wasm);
        let outcome = wasm
            .execute_with_replay(
                &checkpoint,
                &ReplayRoot::Advance,
                &[
                    ReplayDecisionAnswer::Options(vec![1]),
                    ReplayDecisionAnswer::Targets(Vec::new()),
                ],
            )
            .expect("the exile mode permits zero targets");
        assert!(matches!(
            outcome,
            ReplayOutcome::Complete(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_)))
        ));
        assert_eq!(wasm.game.stack.len(), 1);
        assert_eq!(wasm.game.stack[0].chosen_modes.as_deref(), Some(&[1][..]));
        assert!(wasm.game.stack[0].targets.is_empty());
        ironsmith::game_loop::resolve_stack_entry(&mut wasm.game)
            .expect("the zero-target exile mode should resolve");
        assert!(wasm.game.stack.is_empty());
        assert_eq!(
            wasm.game.object(graveyard_id).unwrap().zone,
            graveyard_card_zone
        );
        assert_eq!(wasm.game.object(hand_id).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn territorial_kavu_discard_mode_defers_discard_until_resolution_and_draws() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, graveyard_id, hand_id) = attacking_kavu(Zone::Graveyard);
    let hand_stable_id = wasm.game.object(hand_id).unwrap().stable_id;
    let checkpoint = mode_checkpoint(&wasm);
    let outcome = wasm
        .execute_with_replay(
            &checkpoint,
            &ReplayRoot::Advance,
            &[ReplayDecisionAnswer::Options(vec![0])],
        )
        .expect("the discard mode should stack without choosing a card yet");
    assert!(matches!(
        outcome,
        ReplayOutcome::Complete(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_)))
    ));
    assert_eq!(wasm.game.stack.len(), 1);
    assert_eq!(wasm.game.stack[0].chosen_modes.as_deref(), Some(&[0][..]));
    assert_eq!(wasm.game.object(hand_id).unwrap().zone, Zone::Hand);

    let before_resolution = wasm.game.clone();
    let mut dm = WasmReplayDecisionMaker::new(&[]);
    ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut dm)
        .expect("resolution should pause for the discard");
    let (prompt, _, _, _) = dm.finish();
    let ctx = match prompt {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => panic!("expected a discard selection during resolution, got {other:?}"),
    };
    assert!(
        ctx.candidates
            .iter()
            .any(|candidate| candidate.id == hand_id)
    );

    wasm.game = before_resolution;
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(vec![hand_id])]);
    ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut dm)
        .expect("discarding should complete the trigger and draw a card");
    assert!(dm.finish().0.is_none());
    let discarded_id = wasm.game.find_object_by_stable_id(hand_stable_id).unwrap();
    assert_eq!(
        wasm.game.object(discarded_id).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        wasm.game.object(graveyard_id).unwrap().zone,
        Zone::Graveyard
    );
    let alice = wasm.game.player(PlayerId::from_index(0)).unwrap();
    assert_eq!(alice.hand.len(), 2);
    assert!(
        alice
            .hand
            .iter()
            .any(|id| wasm.game.object(*id).unwrap().name == "Island")
    );
}

/// Exercise the same typed command validation and runner routing as dispatch,
/// without the JS-only snapshot serialization at the browser boundary.
fn respond_to_runner(wasm: &mut WasmGame, command: UiCommand) {
    assert!(wasm.runner_pending_decision, "expected a runner decision");
    let pending = wasm
        .pending_decision
        .take()
        .expect("pending runner decision");
    wasm.runner_pending_decision = false;
    wasm.apply_runner_decision(pending, command)
        .expect("runner command should advance to the next UI decision");
}

fn select_cleanup_discard(wasm: &mut WasmGame, discard: ObjectId) {
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::SelectObjects(_))
    ));
    respond_to_runner(
        wasm,
        UiCommand::SelectObjects {
            object_ids: vec![discard.0],
            object_stable_ids: Vec::new(),
            object_hidden_refs: Vec::new(),
        },
    );
}

fn choose_pending_trigger_mode(wasm: &mut WasmGame, description: &str) -> usize {
    let options = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(options)) => options,
        other => panic!("expected trigger mode selection, got {other:?}"),
    };
    assert!(options.options.iter().filter(|option| option.legal).count() > 1);
    let index = options
        .options
        .iter()
        .find(|option| option.description.contains(description))
        .expect("requested mode should be offered")
        .index;
    assert_ne!(
        index, options.options[0].index,
        "exercise a nonfirst choice"
    );
    assert!(
        wasm.game.stack.is_empty(),
        "mode choice must precede stacking"
    );
    let pending = wasm
        .pending_replay_action
        .as_ref()
        .expect("replayable trigger mode")
        .clone();
    let mut answers = pending.nested_answers;
    answers.push(ReplayDecisionAnswer::Options(vec![index]));
    let outcome = wasm
        .execute_with_replay(&pending.checkpoint, &pending.root, &answers)
        .expect("the selected mode should reach the stack");
    assert!(matches!(
        outcome,
        ReplayOutcome::Complete(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_)))
    ));
    assert_eq!(wasm.game.stack.len(), 1);
    assert_eq!(
        wasm.game.stack[0].chosen_modes.as_deref(),
        Some(&[index][..])
    );
    index
}

#[test]
fn runner_blocking_gargaroth_prompts_for_mode_before_stacking() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gargaroth = crate::compile_test_card_definitions("Elder Gargaroth")
        .unwrap()
        .remove(0);
    let blocker = wasm
        .game
        .create_object_from_definition(&gargaroth, bob, Zone::Battlefield);
    let attacker = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::grizzly_bears(),
        alice,
        Zone::Battlefield,
    );
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::Combat;
    wasm.game.turn.step = Some(Step::DeclareBlockers);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersCheck);
    runner
        .combat_mut()
        .attackers
        .push(ironsmith::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
    wasm.game.combat = Some(runner.combat().clone());
    wasm.runner = Some(runner);
    wasm.advance_until_decision()
        .expect("blocker selection should be exposed");
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::Blockers(_))
    ));
    respond_to_runner(
        &mut wasm,
        UiCommand::DeclareBlockers {
            declarations: vec![BlockerDeclarationInput {
                blocker: blocker.0,
                blocking: attacker.0,
            }],
        },
    );
    assert_eq!(wasm.game.player(bob).unwrap().life, 20);
    assert_eq!(wasm.game.battlefield.len(), 2);
    choose_pending_trigger_mode(&mut wasm, "gain 3 life");
    assert_eq!(wasm.game.stack[0].controller, bob);
    assert_eq!(
        wasm.game.player(bob).unwrap().life,
        20,
        "mode effects wait for resolution"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();
    assert_eq!(wasm.game.player(bob).unwrap().life, 23);
    assert_eq!(
        wasm.game.battlefield.len(),
        2,
        "the unchosen Beast mode must not run"
    );
}

#[test]
fn runner_cleanup_monument_prompts_for_mode_before_stacking() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let monument = crate::compile_test_card_definitions("Monument to Endurance")
        .unwrap()
        .remove(0);
    wasm.game
        .create_object_from_definition(&monument, alice, Zone::Battlefield);
    let discard = wasm
        .game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
    let draw = wasm
        .game
        .create_object_from_definition(&basic_island(), alice, Zone::Library);
    wasm.game.player_mut(alice).unwrap().max_hand_size = 0;
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = None;
    wasm.game.turn.phase = Phase::Ending;
    wasm.game.turn.step = Some(Step::Cleanup);
    wasm.runner = Some(TurnRunner::from_state_for_sync(TurnState::CleanupDiscard));
    wasm.advance_until_decision().unwrap();
    select_cleanup_discard(&mut wasm, discard);
    assert_eq!(wasm.game.player(bob).unwrap().life, 20);
    choose_pending_trigger_mode(&mut wasm, "loses 3 life");
    assert_eq!(wasm.game.player(bob).unwrap().life, 20);
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();
    assert_eq!(wasm.game.player(bob).unwrap().life, 17);
    assert_eq!(
        wasm.game.object(draw).unwrap().zone,
        Zone::Library,
        "the unchosen draw mode must not run"
    );
}

#[test]
fn runner_cleanup_preserves_discard_until_replacement_choice_is_answered() {
    let _guard = crate::test_id_counter_guard();
    let rip = crate::compile_test_card_definitions("Rest in Peace")
        .unwrap()
        .remove(0);
    let temper = crate::compile_test_card_definitions("Fiery Temper")
        .unwrap()
        .remove(0);
    for start in [
        TurnState::CleanupDiscard,
        TurnState::CleanupRecursiveDiscard,
    ] {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        wasm.game
            .create_object_from_definition(&rip, alice, Zone::Battlefield);
        let discard = wasm
            .game
            .create_object_from_definition(&temper, alice, Zone::Hand);
        let stable = wasm.game.object(discard).unwrap().stable_id;
        wasm.game.player_mut(alice).unwrap().max_hand_size = 0;
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = None;
        wasm.game.turn.phase = Phase::Ending;
        wasm.game.turn.step = Some(Step::Cleanup);
        wasm.runner = Some(TurnRunner::from_state_for_sync(start));
        wasm.advance_until_decision().unwrap();
        select_cleanup_discard(&mut wasm, discard);
        let options = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(options)) => options,
            other => panic!("expected competing discard replacements, got {other:?}"),
        };
        assert_eq!(options.options.len(), 2);
        let madness = options
            .options
            .iter()
            .find(|option| option.description.contains("Madness"))
            .expect("madness must remain selectable")
            .index;
        assert_ne!(madness, options.options[0].index);
        assert_eq!(
            wasm.game.object(discard).unwrap().zone,
            Zone::Hand,
            "discard must not commit before its replacement choice"
        );
        assert!(wasm.game.stack.is_empty());
        respond_to_runner(
            &mut wasm,
            UiCommand::SelectOptions {
                option_indices: vec![madness],
            },
        );
        assert!(matches!(
            wasm.pending_decision,
            Some(DecisionContext::Priority(_))
        ));
        let exiled = wasm.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(wasm.game.object(exiled).unwrap().zone, Zone::Exile);
        assert!(wasm.game.player(alice).unwrap().hand.is_empty());
        assert_eq!(
            wasm.game.stack.len(),
            1,
            "choosing Madness must create the cast opportunity"
        );
        assert_eq!(wasm.game.stack[0].object_id, exiled);
        assert!(wasm.game.stack[0].is_ability);
    }
}
