#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::*;

#[test]
pub(super) fn cascade_replay_surfaces_adventure_choice_after_accepting_free_cast() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let bloodbraid_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Bloodbraid Elf".to_string(), "hand".to_string(), true)
            .expect("Bloodbraid Elf should be added to hand"),
    );
    for _ in 0..3 {
        wasm.add_card_to_zone(0, "Mountain".to_string(), "battlefield".to_string(), true)
            .expect("Mountain should be added");
    }
    wasm.add_card_to_zone(0, "Forest".to_string(), "battlefield".to_string(), true)
        .expect("Forest should be added");
    wasm.add_card_to_zone(0, "Curious Pair".to_string(), "library".to_string(), true)
        .expect("Curious Pair should be added to library");

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));
    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == bloodbraid_id),
    );

    for _ in 0..32 {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::ManaPayment(ctx)) => {
                let command = serde_wasm_bindgen::to_value(&json!({
                    "type": "mana_payment",
                    "response": {
                        "action": "confirm",
                        "plan_id": ctx.plan.id.to_string(),
                        "request_hash": ctx.plan.request_hash.to_string(),
                    },
                }))
                .expect("mana payment confirmation should serialize");
                wasm.dispatch(command)
                    .expect("authoritative mana payment should succeed");
            }
            Some(DecisionContext::Priority(_)) => dispatch_pass_priority(&mut wasm),
            Some(DecisionContext::Boolean(ctx))
                if ctx.description.contains("Cast Curious Pair without paying") =>
            {
                break;
            }
            other => panic!("expected mana, priority, or cascade may prompt, got {other:?}"),
        }
    }

    dispatch_select_options(&mut wasm, &[1]);

    let choose_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected cascade Adventure choice after accepting may, got {other:?}"),
    };
    assert!(
        choose_ctx
            .options
            .iter()
            .any(|option| option.description == "Cast Treats to Share"),
        "expected Cascade to offer the Adventure half, got {:?}",
        choose_ctx.options
    );
}

#[test]
pub(super) fn saw_in_half_formidable_speaker_no_advances_resolution_chain() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    wasm.add_card_to_zone(
        0,
        "Omniscience".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("Omniscience should be added to the battlefield");

    let original_speaker_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Formidable Speaker".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("Formidable Speaker should enter and trigger"),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("discard a card"),
                "expected Formidable Speaker may prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected Formidable Speaker ETB boolean prompt, got {other:?}"),
    }

    dispatch_select_options(&mut wasm, &[0]);

    let saw_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Saw in Half".to_string(), "hand".to_string(), true)
            .expect("Saw in Half should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == saw_id),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Targets(ctx)) => {
            let target_ids: Vec<ObjectId> = ctx
                .requirements
                .iter()
                .flat_map(|req| req.legal_targets.iter())
                .filter_map(|target| match target {
                    Target::Object(object_id) => Some(*object_id),
                    _ => None,
                })
                .collect();
            assert!(
                target_ids.contains(&original_speaker_id),
                "Saw in Half should be able to target the original Formidable Speaker"
            );
        }
        other => panic!("expected Saw in Half target prompt, got {other:?}"),
    }

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [{ "kind": "object", "object": original_speaker_id.0 }],
        }))
        .expect("target selection should serialize"),
    )
    .expect("targeting Formidable Speaker should succeed");

    for _ in 0..8 {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(_)) => dispatch_pass_priority(&mut wasm),
            _ => break,
        }
    }

    let order_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Order(ctx)) => ctx,
        other => {
            panic!("expected trigger ordering prompt after Saw in Half resolves, got {other:?}")
        }
    };
    assert_eq!(
        order_ctx.items.len(),
        2,
        "Saw in Half should produce exactly two Formidable Speaker ETB triggers"
    );

    dispatch_select_options(&mut wasm, &[0, 1]);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(
                ctx.player, alice,
                "after ordering simultaneous triggers, the active player should receive the first new priority window"
            );
        }
        other => {
            panic!("expected a fresh priority window after ordering triggers, got {other:?}")
        }
    }
    assert_eq!(
        wasm.game.stack.len(),
        2,
        "ordering triggers should not auto-resolve any stack entries"
    );

    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(
                ctx.player,
                PlayerId::from_index(1),
                "one pass should hand priority to the opponent without resolving a trigger"
            );
        }
        other => panic!("expected opponent priority after one pass, got {other:?}"),
    }
    assert_eq!(
        wasm.game.stack.len(),
        2,
        "a single pass must not resolve the top trigger in multiplayer-style priority"
    );

    dispatch_pass_priority(&mut wasm);

    let first_boolean_source = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("discard a card"),
                "expected first resolving Formidable Speaker prompt, got {:?}",
                ctx.description
            );
            ctx.source
        }
        other => panic!("expected first resolving boolean prompt, got {other:?}"),
    };
    assert_eq!(
        wasm.game.stack.len(),
        1,
        "exactly one trigger should have resolved after both players pass"
    );

    dispatch_select_options(&mut wasm, &[0]);

    let next_ctx = wasm
        .pending_decision
        .as_ref()
        .unwrap_or_else(|| panic!("expected another decision after declining the first trigger"));

    match next_ctx {
        DecisionContext::Boolean(ctx) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("discard a card"),
                "expected the second Formidable Speaker prompt after declining the first, got {:?}",
                ctx.description
            );
            assert_ne!(
                ctx.source, first_boolean_source,
                "declining the first trigger should advance to the second trigger, not reissue the same source"
            );
        }
        other => panic!("expected the second Formidable Speaker boolean prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn live_resolution_follow_up_prompts_restore_resolving_stack_object() {
    let mut wasm = WasmGame::new();

    wasm.add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("first Forest should be added to hand");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("library filler should be added");

    wasm.add_card_to_zone(
        0,
        "Cultivator Colossus".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("Cultivator Colossus should enter with ETB processing");

    let resolving_checkpoint = wasm
        .pending_live_continuation
        .as_ref()
        .map(|continuation| continuation.checkpoint.clone())
        .expect("Cultivator ETB prompt should retain the committed resolution checkpoint");
    let next_ctx = wasm
        .pending_decision
        .clone()
        .expect("Cultivator ETB prompt should be pending");
    let expected_resolving_id = wasm
        .active_resolving_stack_object
        .as_ref()
        .map(|entry| entry.id)
        .expect("Cultivator ETB prompt should expose the resolving stack entry");

    wasm.clear_active_resolving_stack_object();
    assert!(
        wasm.active_resolving_stack_object.is_none(),
        "test setup should clear the resolving entry before simulating live dispatch"
    );

    wasm.finish_live_priority_dispatch(
        GameProgress::NeedsDecisionCtx(next_ctx),
        None,
        Some(resolving_checkpoint),
    )
    .expect("live follow-up prompt should snapshot cleanly");

    assert_eq!(
        wasm.active_resolving_stack_object
            .as_ref()
            .map(|entry| entry.id),
        Some(expected_resolving_id),
        "live follow-up prompts should restore the resolving stack entry from the committed resolution checkpoint"
    );
}

#[test]
pub(super) fn tainted_pact_declining_first_card_advances_to_second_prompt_in_live_ui_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Tainted Pact".to_string(), "hand".to_string(), true)
            .expect("Tainted Pact should be added to hand"),
    );
    wasm.add_card_to_zone(0, "Second Card".to_string(), "library".to_string(), true)
        .expect("second library card should be added");
    wasm.add_card_to_zone(0, "First Card".to_string(), "library".to_string(), true)
        .expect("first library card should be added");

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("first card"),
                "expected first Tainted Pact prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected first Tainted Pact boolean prompt, got {other:?}"),
    }

    dispatch_select_options(&mut wasm, &[0]);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("second card"),
                "declining the first card should advance to the second prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected second Tainted Pact boolean prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn tainted_pact_declining_first_revealed_unique_card_prompts_for_second_card() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Tainted Pact".to_string(), "hand".to_string(), true)
            .expect("Tainted Pact should be added to hand"),
    );
    wasm.game
        .create_hidden_card_placeholder(alice, Zone::Library, 0, "alice-slot-0".to_string());
    wasm.game
        .create_hidden_card_placeholder(alice, Zone::Library, 1, "alice-slot-1".to_string());

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    wasm.reveal_hidden_slot(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "slot": 1,
            "cardName": "Tainted Pact",
            "commitment": "alice-slot-1",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("first exiled card should reveal");

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description
                    .to_ascii_lowercase()
                    .contains("tainted pact"),
                "expected first revealed Tainted Pact prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected first Tainted Pact boolean prompt, got {other:?}"),
    }

    dispatch_select_options(&mut wasm, &[0]);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("hidden card")
                    || ctx.description.to_ascii_lowercase().contains("swamp"),
                "declining a unique first card should advance to the second prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected second Tainted Pact boolean prompt, got {other:?}"),
    }

    wasm.reveal_hidden_slot(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "slot": 0,
            "cardName": "Swamp",
            "commitment": "alice-slot-0",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("second exiled card should reveal");

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => {
            assert!(
                ctx.description.to_ascii_lowercase().contains("swamp"),
                "revealing the second unique card should preserve the choice prompt, got {:?}",
                ctx.description
            );
        }
        other => panic!("expected revealed second Tainted Pact prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn reveal_hidden_position_uses_position_commitment_over_private_slot_collision() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let wrong_private_slot = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        58,
        "alice-slot-58".to_string(),
    );
    let correct_position = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        36,
        "alice-slot-36".to_string(),
    );
    wasm.game.set_hidden_card_info(
        correct_position,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 36,
            commitment: "alice-slot-36".to_string(),
            public_slot: Some(58),
            public_commitment: Some("ziffle:test-deck:58".to_string()),
        },
    );

    wasm.reveal_hidden_position(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "position": 58,
            "originalSlot": 36,
            "cardName": "Swamp",
            "positionCommitment": "ziffle:test-deck:58",
            "commitment": "alice-slot-36",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("ziffle reveal should choose the object with the matching position commitment");

    assert_eq!(
        wasm.game
            .object(correct_position)
            .expect("correct position object should exist")
            .name,
        "Swamp",
        "the public ziffle commitment should select the object at that shuffled position"
    );
    assert_eq!(
        wasm.game
            .object(wrong_private_slot)
            .expect("private-slot collision object should exist")
            .name,
        "Hidden Card",
        "a matching private slot number must not win over a mismatched position commitment"
    );
}

#[test]
pub(super) fn reveal_hidden_position_preserves_existing_public_identity_for_private_opening() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let hand_id = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Hand,
        10,
        "ziffle:initial-deck:10".to_string(),
    );
    wasm.game.set_hidden_card_info(
        hand_id,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Hand,
            slot: 10,
            commitment: "ziffle:initial-deck:10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:shuffle-deck:51".to_string()),
        },
    );

    wasm.reveal_hidden_position(
        serde_wasm_bindgen::to_value(&json!({
            "owner": 0,
            "objectId": hand_id.0,
            "position": 10,
            "originalSlot": 40,
            "cardName": "Swamp",
            "positionCommitment": "ziffle:initial-deck:10",
            "commitment": "private-slot-40",
        }))
        .expect("reveal input should serialize"),
    )
    .expect("private position reveal should preserve the public ziffle identity");

    let info = wasm
        .game
        .hidden_card_info(hand_id)
        .expect("revealed hidden card should retain hidden metadata");
    assert_eq!(info.slot, 40);
    assert_eq!(info.commitment, "private-slot-40");
    assert_eq!(info.public_slot, Some(51));
    assert_eq!(
        info.public_commitment.as_deref(),
        Some("ziffle:shuffle-deck:51")
    );
    assert_eq!(
        wasm.game
            .object(hand_id)
            .expect("hand object should still exist")
            .name,
        "Swamp"
    );
}

#[test]
pub(super) fn reveal_hidden_positions_reveals_multiple_ziffle_positions_in_one_batch() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let first = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        10,
        "private-slot-10".to_string(),
    );
    let second = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        20,
        "private-slot-20".to_string(),
    );
    wasm.game.set_hidden_card_info(
        first,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 10,
            commitment: "private-slot-10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:test-deck:51".to_string()),
        },
    );
    wasm.game.set_hidden_card_info(
        second,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 20,
            commitment: "private-slot-20".to_string(),
            public_slot: Some(52),
            public_commitment: Some("ziffle:test-deck:52".to_string()),
        },
    );

    wasm.reveal_hidden_positions(
        serde_wasm_bindgen::to_value(&json!({
            "reveals": [
                {
                    "owner": 0,
                    "position": 51,
                    "originalSlot": 10,
                    "cardName": "Island",
                    "positionCommitment": "ziffle:test-deck:51",
                    "commitment": "private-slot-10",
                },
                {
                    "owner": 0,
                    "position": 52,
                    "originalSlot": 20,
                    "cardName": "Swamp",
                    "positionCommitment": "ziffle:test-deck:52",
                    "commitment": "private-slot-20",
                },
            ],
        }))
        .expect("batch reveal input should serialize"),
    )
    .expect("batch reveal should apply");

    assert_eq!(
        wasm.game.object(first).expect("first card exists").name,
        "Island"
    );
    assert_eq!(
        wasm.game.object(second).expect("second card exists").name,
        "Swamp"
    );
}

#[test]
pub(super) fn reveal_hidden_positions_rejects_batch_without_partial_reveals() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let first = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        10,
        "private-slot-10".to_string(),
    );
    let second = wasm.game.create_hidden_card_placeholder(
        alice,
        Zone::Library,
        20,
        "private-slot-20".to_string(),
    );
    wasm.game.set_hidden_card_info(
        first,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 10,
            commitment: "private-slot-10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:test-deck:51".to_string()),
        },
    );
    wasm.game.set_hidden_card_info(
        second,
        ironsmith::game_state::HiddenCardInfo {
            owner: alice,
            zone: Zone::Library,
            slot: 20,
            commitment: "private-slot-20".to_string(),
            public_slot: Some(52),
            public_commitment: Some("ziffle:test-deck:52".to_string()),
        },
    );

    let result = wasm.reveal_hidden_positions(
        serde_wasm_bindgen::to_value(&json!({
            "reveals": [
                {
                    "owner": 0,
                    "position": 51,
                    "originalSlot": 10,
                    "cardName": "Island",
                    "positionCommitment": "ziffle:test-deck:51",
                    "commitment": "private-slot-10",
                },
                {
                    "owner": 0,
                    "position": 52,
                    "originalSlot": 20,
                    "cardName": "Swamp",
                    "positionCommitment": "ziffle:test-deck:wrong",
                    "commitment": "private-slot-20",
                },
            ],
        }))
        .expect("batch reveal input should serialize"),
    );

    assert!(result.is_err(), "invalid batch reveal should fail");
    assert_eq!(
        wasm.game.object(first).expect("first card exists").name,
        "Hidden Card",
        "the valid first reveal must not be applied before the invalid second reveal is rejected"
    );
    assert_eq!(
        wasm.game.object(second).expect("second card exists").name,
        "Hidden Card",
        "the invalid second reveal should remain hidden"
    );
}

#[test]
pub(super) fn demonic_consultation_resolution_prompts_for_card_name_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Demonic Consultation".to_string(),
            "hand".to_string(),
            true,
        )
        .expect("Demonic Consultation should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::TextInput(ctx)) => {
            assert_eq!(ctx.description, "Choose a card name");
            assert_eq!(ctx.placeholder.as_deref(), Some("Enter a card name"));
        }
        other => panic!("expected Demonic Consultation card-name prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn mystical_tutor_resolution_prompts_for_hidden_library_choice_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Mystical Tutor".to_string(), "hand".to_string(), true)
            .expect("Mystical Tutor should be added to hand"),
    );
    wasm.game
        .player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let hidden_library_ids: Vec<ObjectId> = (0..3)
        .map(|slot| {
            wasm.game.create_hidden_card_placeholder(
                alice,
                Zone::Library,
                slot,
                format!("alice-hidden-library-{slot}"),
            )
        })
        .collect();

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .collect::<Vec<_>>(),
                hidden_library_ids,
                "Mystical Tutor should pause on the owner with hidden library candidates"
            );
        }
        other => panic!("expected Mystical Tutor hidden-library prompt, got {other:?}"),
    }

    assert!(
        wasm.active_audit_viewed_cards.iter().any(|view| {
            view.viewer == alice
                && view.subject == alice
                && view.zone == Zone::Library
                && !view.public
                && view.cards == hidden_library_ids
        }),
        "WASM dispatch should retain the private library view for audit material"
    );
}

#[test]
pub(super) fn krrik_casting_black_spell_surfaces_pay_two_life_option_in_wasm_flow() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    wasm.add_card_to_zone(
        0,
        "K'rrik, Son of Yawgmoth".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("K'rrik should be added to the battlefield");
    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Demonic Consultation".to_string(),
            "hand".to_string(),
            true,
        )
        .expect("Demonic Consultation should be added to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => {
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.description == "Pay 2 life"),
                "expected K'rrik to surface a pay-2-life payment option in the WASM decision"
            );
        }
        other => panic!("expected mana payment choice after starting the cast, got {other:?}"),
    }
}

#[test]
pub(super) fn public_reveal_survives_replay_advance_to_next_prompt() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;
    wasm.runner = Some(ironsmith::turn_runner::TurnRunner::new());
    wasm.runner_awaiting_priority = true;

    let revealed_card = CardBuilder::new(CardId::from_raw(90_200), "Bob's Revealed Top")
        .card_types(vec![CardType::Instant])
        .build();
    let revealed_id = wasm
        .game
        .create_object_from_card(&revealed_card, bob, Zone::Library);

    let mut replay_dm = WasmReplayDecisionMaker::new(&[]);
    let view_ctx = ViewCardsContext::new(alice, bob, None, Zone::Library, "Reveal consulted cards")
        .with_public(true);
    DecisionMaker::view_cards(&mut replay_dm, &wasm.game, alice, &[revealed_id], &view_ctx);
    let (_, viewed_cards, audit_viewed_cards) = replay_dm.finish();
    wasm.active_viewed_cards = viewed_cards;
    wasm.active_audit_viewed_cards = audit_viewed_cards;

    wasm.advance_until_decision()
        .expect("advance should produce a priority prompt");

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("publicly revealed cards should still be surfaced at the next prompt");
    assert_eq!(viewed_cards.visibility, "public");
    assert_eq!(viewed_cards.zone, "Library");
    assert_eq!(viewed_cards.card_ids, vec![revealed_id.0]);
    assert_eq!(viewed_cards.cards[0].name, "Bob's Revealed Top");
}

#[test]
pub(super) fn public_reveal_resolves_stale_replay_ids_to_live_card_names() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let revealed_card = CardBuilder::new(CardId::from_raw(90_201), "Bob's Moving Top")
        .card_types(vec![CardType::Instant])
        .build();
    let revealed_id = wasm
        .game
        .create_object_from_card(&revealed_card, bob, Zone::Library);

    let mut replay_dm = WasmReplayDecisionMaker::new(&[]);
    let view_ctx = ViewCardsContext::new(alice, bob, None, Zone::Library, "Reveal consulted cards")
        .with_public(true);
    DecisionMaker::view_cards(&mut replay_dm, &wasm.game, alice, &[revealed_id], &view_ctx);
    let (_, viewed_cards, audit_viewed_cards) = replay_dm.finish();
    wasm.active_viewed_cards = viewed_cards;
    wasm.active_audit_viewed_cards = audit_viewed_cards;

    let moved_id = wasm
        .game
        .move_object(
            revealed_id,
            Zone::Hand,
            ironsmith::events::cause::EventCause::from_game_rule(),
        )
        .expect("card should move to hand");

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("publicly revealed cards should still be surfaced");
    assert_eq!(viewed_cards.card_ids, vec![moved_id.0]);
    assert_eq!(viewed_cards.cards[0].id, moved_id.0);
    assert_eq!(viewed_cards.cards[0].name, "Bob's Moving Top");
}

#[test]
pub(super) fn viewed_card_snapshots_follow_stable_identity_when_object_id_is_stale() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let revealed_card = CardBuilder::new(CardId::from_raw(90_202), "Bob's Stable Secret")
        .card_types(vec![CardType::Instant])
        .build();
    let revealed_id = wasm
        .game
        .create_object_from_card(&revealed_card, bob, Zone::Hand);
    let stale_unrelated_id = ObjectId::from_raw(revealed_id.0.saturating_add(10_000));
    wasm.active_viewed_cards = Some(ActiveViewedCards {
        viewer: alice,
        subject: bob,
        zone: Zone::Hand,
        cards: vec![stale_unrelated_id],
        card_stable_ids: stable_ids_for_viewed_cards(&wasm.game, &[revealed_id]),
        public: false,
        source: None,
        description: "Inspect hidden card for decision".to_string(),
    });

    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        alice,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        None,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );

    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("view should resolve through the card's stable id");
    assert_eq!(viewed_cards.card_ids, vec![revealed_id.0]);
    assert_eq!(viewed_cards.cards[0].name, "Bob's Stable Secret");
    assert_eq!(
        snapshot.players[bob.index()].hand_cards[0].name,
        "Bob's Stable Secret"
    );
}

#[test]
pub(super) fn cultivator_colossus_snapshot_tracks_repeat_iteration_state() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    let forest_a = wasm
        .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("first Forest should be added to hand");
    let forest_b = wasm
        .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
        .expect("second Forest should be added to hand");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("first library filler should be added");
    wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
        .expect("second library filler should be added");

    wasm.add_card_to_zone(
        0,
        "Cultivator Colossus".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("Cultivator Colossus should enter with ETB processing");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let resolving_stack_object = snapshot
        .resolving_stack_object
        .as_ref()
        .expect("Cultivator ETB prompt should expose the resolving trigger in the snapshot");
    assert_eq!(resolving_stack_object.name, "Cultivator Colossus");
    assert_eq!(
        resolving_stack_object.ability_kind.as_deref(),
        Some("Triggered"),
        "the pinned resolving entry should surface Cultivator's ETB as a triggered ability"
    );
    assert!(
        snapshot.stack_objects.is_empty(),
        "the real stack should stay empty while the UI-only resolving entry is shown separately"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("yes choice should serialize"),
    )
    .expect("accepting the first Cultivator iteration should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        0,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist in snapshot");
    let mut hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
    hand_ids.sort_unstable();
    assert_eq!(
        hand_ids,
        vec![forest_a, forest_b],
        "first land-choice snapshot should still show both lands in hand"
    );
    let first_choice = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include first land-choice decision")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects snapshot, got {other:?}"),
    };
    let mut first_candidates: Vec<u64> = first_choice
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id)
        .collect();
    first_candidates.sort_unstable();
    assert_eq!(
        first_candidates,
        vec![forest_a, forest_b],
        "first land-choice snapshot should offer both lands"
    );
    assert!(
        snapshot.resolving_stack_object.is_some(),
        "the resolving Cultivator trigger should stay visible during the land-choice step"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_objects",
            "object_ids": [forest_a],
        }))
        .expect("first land selection should serialize"),
    )
    .expect("choosing the first land should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        1,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist in snapshot");
    let hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
    assert_eq!(
        hand_ids,
        vec![forest_b],
        "after the first land move, the snapshot hand should only show the remaining land"
    );
    let forest_count = me
        .battlefield
        .iter()
        .filter(|permanent| permanent.name == "Forest")
        .map(|permanent| permanent.count)
        .sum::<usize>();
    assert_eq!(
        forest_count, 1,
        "after the first land move, the snapshot battlefield should already show one Forest"
    );
    match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the repeated may decision")
    {
        super::DecisionView::SelectOptions { options, .. } => {
            let option_text: Vec<&str> = options
                .iter()
                .map(|option| option.description.as_str())
                .collect();
            assert_eq!(option_text, vec!["Yes", "No"]);
        }
        other => panic!("expected repeat yes/no snapshot, got {other:?}"),
    }
    assert!(
        snapshot.resolving_stack_object.is_some(),
        "the resolving Cultivator trigger should stay visible across repeat iterations"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("second yes choice should serialize"),
    )
    .expect("accepting the second Cultivator iteration should succeed");

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        wasm.active_viewed_cards.as_ref(),
        wasm.is_cancelable(),
        None,
        2,
    );
    let me = snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("perspective player should exist in snapshot");
    let hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
    assert_eq!(
        hand_ids,
        vec![forest_b],
        "before the second land is chosen, the snapshot hand should still show only the remaining land"
    );
    let second_choice = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include second land-choice decision")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected second select_objects snapshot, got {other:?}"),
    };
    let second_candidates: Vec<u64> = second_choice
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id)
        .collect();
    assert_eq!(
        second_candidates,
        vec![forest_b],
        "second land-choice snapshot should only offer the remaining land"
    );
}

#[test]
pub(super) fn pregame_mulligan_prompt_offers_keep_and_mulligan() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    let actions = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => &ctx.actions,
        other => panic!("expected pregame priority decision, got {other:?}"),
    };
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, LegalAction::KeepOpeningHand))
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, LegalAction::TakeMulligan))
    );
}

#[test]
pub(super) fn normal_multiplayer_first_mulligan_is_free() {
    let turn_order = vec![
        PlayerId::from_index(0),
        PlayerId::from_index(1),
        PlayerId::from_index(2),
    ];
    let pregame = PregameState::new(&turn_order, 7, MatchFormatInput::Normal);

    assert_eq!(
        pregame.free_mulligan_count(),
        1,
        "CR 103.5c gives every player in an ordinary multiplayer game a free first mulligan"
    );
}

#[test]
pub(super) fn mulligan_is_not_offered_after_the_opening_hand_reaches_zero() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let alice = PlayerId::from_index(0);
    let mut pregame = PregameState::new(
        &wasm.game.turn_store.turn_order,
        7,
        MatchFormatInput::Normal,
    );
    pregame.mulligans_taken.insert(alice, 7);
    wasm.pregame = Some(pregame);

    wasm.advance_until_decision()
        .expect("the forced final mulligan state should still produce a keep decision");

    let actions = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => &ctx.actions,
        other => panic!("expected final mulligan priority decision, got {other:?}"),
    };
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, LegalAction::KeepOpeningHand))
    );
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, LegalAction::TakeMulligan)),
        "CR 103.5 stops offering mulligans once the opening hand would already be zero cards"
    );
}

#[test]
pub(super) fn pregame_priority_labels_progress_from_keep_hand_to_pregame() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    assert_eq!(
        snapshot_priority_action_label(&mut wasm, "keep_opening_hand"),
        "Keep hand"
    );

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });

    assert_eq!(
        snapshot_priority_action_label(&mut wasm, "continue_pregame"),
        "Pregame"
    );

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });

    assert_eq!(
        snapshot_priority_action_label(&mut wasm, "begin_game"),
        "Pregame"
    );

    dispatch_matching_priority_action(&mut wasm, |action| matches!(action, LegalAction::BeginGame));

    assert!(
        wasm.pregame.is_none(),
        "game should leave pregame after the Pregame decision"
    );
}

#[test]
pub(super) fn commander_first_mulligan_is_free() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Commander);
    let alice = PlayerId::from_index(0);

    seed_filler_cards(&mut wasm, alice, Zone::Hand, 7);
    seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
    start_pregame(&mut wasm, 7, MatchFormatInput::Commander);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::TakeMulligan)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(ctx.player, alice, "pregame should move to opening actions");
            assert!(
                ctx.actions
                    .iter()
                    .any(|action| matches!(action, LegalAction::ContinuePregame))
            );
        }
        other => panic!("expected opening-actions priority prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn commander_second_mulligan_bottoms_one_card() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Commander);
    let alice = PlayerId::from_index(0);

    seed_filler_cards(&mut wasm, alice, Zone::Hand, 7);
    seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
    start_pregame(&mut wasm, 7, MatchFormatInput::Commander);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::TakeMulligan)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::TakeMulligan)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, Some(1));
        }
        other => panic!("expected one-card bottoming prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn serum_powder_redraws_without_counting_as_a_mulligan() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let alice = PlayerId::from_index(0);

    let serum_id = wasm
        .game
        .create_object_from_definition(&serum_powder(), alice, Zone::Hand);
    seed_filler_cards(&mut wasm, alice, Zone::Hand, 6);
    seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(
            action,
            LegalAction::UsePregameAction { card_id, .. } if *card_id == serum_id
        )
    });

    assert_eq!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .hand
            .len(),
        7,
        "Serum Powder should redraw the same hand size"
    );
    assert_eq!(
        wasm.game.exile.len(),
        7,
        "Serum Powder should exile the original opening hand"
    );
    assert_eq!(
        wasm.pregame
            .as_ref()
            .and_then(|pregame| pregame.mulligans_taken.get(&alice).copied())
            .unwrap_or(0),
        0,
        "Serum Powder should not increment the mulligan count"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "the same player should remain on the mulligan prompt"
    );
}

#[test]
pub(super) fn gemstone_caverns_appears_for_non_starting_player_in_opening_actions() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let bob = PlayerId::from_index(1);

    seed_filler_cards(&mut wasm, PlayerId::from_index(0), Zone::Hand, 7);
    let gemstone_id = wasm
        .game
        .create_object_from_definition(&gemstone_caverns(), bob, Zone::Hand);
    seed_filler_cards(&mut wasm, bob, Zone::Hand, 1);
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(ctx.player, bob);
            assert!(ctx.actions.iter().any(|action| {
                matches!(
                    action,
                    LegalAction::UsePregameAction { card_id, .. }
                        if *card_id == gemstone_id
                )
            }));
            assert!(
                ctx.actions
                    .iter()
                    .any(|action| matches!(action, LegalAction::BeginGame))
            );
        }
        other => panic!("expected Bob's opening-actions prompt, got {other:?}"),
    }
}

#[test]
pub(super) fn gemstone_caverns_moves_to_battlefield_and_prompts_for_exile() {
    let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
    let bob = PlayerId::from_index(1);

    seed_filler_cards(&mut wasm, PlayerId::from_index(0), Zone::Hand, 7);
    let _gemstone_id =
        wasm.game
            .create_object_from_definition(&gemstone_caverns(), bob, Zone::Hand);
    let exile_card = seed_filler_cards(&mut wasm, bob, Zone::Hand, 1)
        .into_iter()
        .next()
        .expect("expected filler card in hand");
    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::KeepOpeningHand)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });
    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::UsePregameAction { .. })
    });

    let gemstone_on_battlefield = wasm.game.battlefield.iter().copied().find(|id| {
        wasm.game
            .object(*id)
            .is_some_and(|object| object.name == "Gemstone Caverns")
    });
    let gemstone_on_battlefield =
        gemstone_on_battlefield.expect("Gemstone Caverns should move to the battlefield");
    assert_eq!(
        wasm.game
            .object(gemstone_on_battlefield)
            .and_then(|object| object.counters.get(&CounterType::Luck).copied()),
        Some(1),
        "Gemstone Caverns should enter with a luck counter"
    );
    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            assert_eq!(ctx.player, bob);
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, Some(1));
        }
        other => panic!("expected Gemstone exile prompt, got {other:?}"),
    }

    dispatch_select_objects(&mut wasm, &[exile_card.0]);

    assert!(
        wasm.game.exile.iter().any(|id| wasm
            .game
            .object(*id)
            .is_some_and(|object| object.name == "Ornithopter")),
        "the chosen card should be exiled"
    );
    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => {
            assert_eq!(ctx.player, bob);
            assert!(
                ctx.actions
                    .iter()
                    .any(|action| matches!(action, LegalAction::BeginGame))
            );
        }
        other => panic!("expected Bob to resume opening actions, got {other:?}"),
    }
}

#[test]
pub(super) fn custom_card_preview_supports_split_faces_and_fuse() {
    let wasm = WasmGame::new();
    let draft = CustomCardInput {
        layout: CustomCardLayoutInput::Split,
        has_fuse: true,
        faces: vec![
            custom_face(
                "Breaking Forge",
                &["Sorcery"],
                "Target player mills four cards.",
                None,
                None,
            ),
            custom_face(
                "Entering Forge",
                &["Sorcery"],
                "Return target creature card from a graveyard to the battlefield under your control.",
                None,
                None,
            ),
        ],
    };

    let preview = wasm
        .build_custom_card_preview(&draft)
        .expect("split custom preview should compile");

    assert_eq!(preview.faces.len(), 2);
    assert!(preview.has_fuse);
    assert_eq!(preview.faces[0].name, "Breaking Forge");
    assert_eq!(preview.faces[1].name, "Entering Forge");
}

#[test]
pub(super) fn create_custom_card_registers_runtime_linked_face_lookup() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let payload = serde_wasm_bindgen::to_value(&json!({
        "draft": {
            "layout": "transform_like",
            "hasFuse": false,
            "faces": [
                {
                    "name": "Forge Pup",
                    "manaCost": "{1}{R}",
                    "cardTypes": ["Creature"],
                    "subtypes": ["Wolf"],
                    "oracleText": "Haste",
                    "power": "2",
                    "toughness": "1"
                },
                {
                    "name": "Forge Howler",
                    "cardTypes": ["Creature"],
                    "subtypes": ["Wolf"],
                    "oracleText": "Trample",
                    "power": "4",
                    "toughness": "3"
                }
            ]
        },
        "playerIndex": 0,
        "zoneName": "hand",
        "skipTriggers": true
    }))
    .expect("custom card payload should encode");

    let object_id = wasm
        .create_custom_card(payload)
        .expect("linked custom card should be created");
    let object = wasm
        .game
        .object(ObjectId(object_id))
        .expect("created custom card should exist");

    assert_eq!(object.name, "Forge Pup");
    let linked = wasm
        .game
        .linked_face_definition_by_name_or_id(object.other_face_name.as_deref(), object.other_face)
        .expect("linked custom back face should resolve at runtime");
    assert_eq!(linked.name(), "Forge Howler");
}

#[test]
pub(super) fn external_linked_card_sources_accept_generated_camel_case_fields() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let sources = json!({
        "version": 1,
        "canonicalName": "Ondu Inversion",
        "aliases": [
            {
                "alias": "Ondu Inversion // Ondu Skyruins",
                "canonical": "Ondu Inversion"
            }
        ],
        "group": {
            "kind": "linked",
            "layout": "transform_like",
            "combinedName": "Ondu Inversion // Ondu Skyruins",
            "hasFuse": false,
            "faces": [
                {
                    "name": "Ondu Inversion",
                    "block": "Mana cost: {6}{W}{W}\nType: Sorcery\nDestroy all nonland permanents.",
                    "score": 1.0
                },
                {
                    "name": "Ondu Skyruins",
                    "block": "Type: Land\nThis land enters tapped.\n{T}: Add {W}.",
                    "score": 1.0
                }
            ]
        }
    });

    wasm.register_external_card_sources_json(sources.to_string())
        .expect("generated linked source JSON should register");
    let object_id = wasm
        .add_card_to_hand(0, "Ondu Inversion".to_string())
        .expect("front face should be addable after registration");
    let object = wasm
        .game
        .object(ObjectId(object_id))
        .expect("added object should exist");
    assert_eq!(object.name, "Ondu Inversion");
    assert_eq!(object.other_face_name.as_deref(), Some("Ondu Skyruins"));
}

#[test]
pub(super) fn snapshot_shows_foretold_card_only_to_the_player_allowed_to_look() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    let def = CardDefinitionBuilder::new(CardId::from_raw(50_001), "Foretell Snapshot Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .foretell(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::special_actions::perform(
        ironsmith::special_actions::SpecialAction::Foretell { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("foretell should succeed");

    let alice_snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let bob_snapshot = GameSnapshot::from_game(
        &game,
        bob,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );

    let alice_view = alice_snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice snapshot should exist");
    let bob_view_of_alice = bob_snapshot
        .players
        .iter()
        .find(|player| player.id == alice.0)
        .expect("alice zone snapshot should exist for bob");

    assert_eq!(alice_view.exile_cards.len(), 1);
    assert_eq!(alice_view.exile_cards[0].name, "Foretell Snapshot Probe");
    assert_eq!(bob_view_of_alice.exile_cards.len(), 1);
    assert_eq!(bob_view_of_alice.exile_cards[0].name, "Hidden card");
    assert!(
        bob_view_of_alice.exile_cards[0].card_types.is_empty(),
        "unauthorized players should not learn the face-down exiled card's characteristics"
    );
}

#[test]
pub(super) fn snapshot_uses_exile_look_permissions_instead_of_card_ownership() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let card_id = game.create_object_from_definition(&ornithopter(), bob, Zone::Exile);
    game.set_face_down(card_id);
    game.grant_face_down_exile_view(card_id, alice);

    let alice_snapshot = GameSnapshot::from_game(
        &game,
        alice,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );
    let bob_snapshot = GameSnapshot::from_game(
        &game,
        bob,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        false,
        None,
        0,
    );

    let alice_view_of_bob = alice_snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist for alice");
    let bob_view = bob_snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("bob snapshot should exist");

    assert_eq!(alice_view_of_bob.exile_cards.len(), 1);
    assert_eq!(alice_view_of_bob.exile_cards[0].name, "Ornithopter");
    assert_eq!(bob_view.exile_cards.len(), 1);
    assert_eq!(bob_view.exile_cards[0].name, "Hidden card");
}
