#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_02::*;
use super::*;

#[test]
pub(super) fn yawgmoth_activation_stays_cancelable_through_target_and_cost_prompts() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let yawgmoth_id = wasm.game.create_object_from_definition(
        &yawgmoth_thran_physician(),
        alice,
        Zone::Battlefield,
    );
    let target_id =
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    wasm.game
        .create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == yawgmoth_id
            )
        })
        .expect("expected Yawgmoth activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Yawgmoth should enter target selection");

    let targets_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected target prompt after Yawgmoth activation, got {other:?}"),
    };
    assert_eq!(
        targets_ctx.player, alice,
        "Yawgmoth target prompt should belong to the activating player"
    );
    assert!(
        wasm.pending_replay_action.is_some(),
        "Yawgmoth activation should keep replay state open while choosing targets"
    );
    assert!(
        wasm.is_cancelable(),
        "Yawgmoth activation should remain cancelable during target selection"
    );

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let cancelable = wasm.is_cancelable();
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        cancelable,
        wasm.visible_undo_land_stable_id(cancelable),
        0,
    );
    assert!(
        snapshot.cancelable,
        "snapshot should expose Yawgmoth target prompt as cancelable"
    );
    assert!(
        snapshot.resolving_stack_object.is_none(),
        "activation-time target prompts should not pin a resolving stack entry"
    );
    let decision = snapshot
        .decision
        .expect("snapshot should still include the target decision");
    let player = match decision {
        super::DecisionView::Targets { player, .. } => player,
        other => panic!("expected target decision snapshot, got {other:?}"),
    };
    assert_eq!(
        player, alice.0,
        "snapshot target decision should belong to the perspective player"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "object", "object": target_id.0 }
            ],
        }))
        .expect("target selection command should serialize"),
    )
    .expect("choosing Yawgmoth's target should continue activation");

    let next_cost_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected next-cost prompt after Yawgmoth target, got {other:?}"),
    };
    assert_eq!(
        next_cost_ctx.player, alice,
        "Yawgmoth next-cost prompt should belong to the activating player"
    );
    assert!(
        wasm.pending_replay_action.is_some(),
        "Yawgmoth activation should keep replay state open while choosing costs"
    );
    assert!(
        wasm.is_cancelable(),
        "Yawgmoth activation should remain cancelable after choosing targets"
    );

    let pending_cast_stack_id = wasm
        .priority_state
        .pending_cast
        .as_ref()
        .map(|p| p.stack_id);
    let cancelable = wasm.is_cancelable();
    let snapshot = GameSnapshot::from_game(
        &wasm.game,
        wasm.perspective,
        wasm.pending_decision.as_ref(),
        None,
        wasm.game_over.as_ref(),
        pending_cast_stack_id,
        wasm.active_resolving_stack_object.clone(),
        Vec::new(),
        None,
        cancelable,
        wasm.visible_undo_land_stable_id(cancelable),
        0,
    );
    assert!(
        snapshot.cancelable,
        "snapshot should expose Yawgmoth next-cost prompt as cancelable"
    );
    assert!(
        snapshot.resolving_stack_object.is_none(),
        "cost-payment prompts should not pin a resolving stack entry before the ability is committed"
    );
    let decision = snapshot
        .decision
        .expect("snapshot should still include the next-cost decision");
    match decision {
        super::DecisionView::SelectOptions { player, reason, .. } => {
            assert_eq!(player, alice.0);
            assert_eq!(reason.as_deref(), Some("Next cost"));
        }
        other => panic!("expected next-cost decision snapshot, got {other:?}"),
    }
}

#[test]
pub(super) fn yawgmoth_proliferate_next_cost_choices_advance_in_replay_chain() {
    fn setup_proliferate_prompt() -> WasmGame {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let yawgmoth_id = wasm.game.create_object_from_definition(
            &yawgmoth_thran_physician(),
            alice,
            Zone::Battlefield,
        );
        wasm.add_card_to_zone(
            alice.0,
            "Black Lotus".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Black Lotus to battlefield");
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Hand);
        wasm.game
            .create_object_from_definition(&ornithopter(), alice, Zone::Hand);

        let proliferate_ability_index = wasm
            .game
            .object(yawgmoth_id)
            .and_then(|object| {
                object.abilities.iter().position(|ability| {
                    matches!(
                        &ability.kind,
                        ironsmith::ability::AbilityKind::Activated(activated)
                            if activated.mana_cost.mana_cost().is_some()
                                && activated
                                    .mana_cost
                                    .costs()
                                    .iter()
                                    .any(|cost| cost.is_discard())
                    )
                })
            })
            .expect("Yawgmoth should have proliferate ability");

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::ActivateAbility { source, ability_index }
                        if *source == yawgmoth_id && *ability_index == proliferate_ability_index
                )
            })
            .expect("expected Yawgmoth proliferate activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Yawgmoth proliferate should open next-cost chooser");

        assert!(
            matches!(
                wasm.pending_decision,
                Some(DecisionContext::SelectOptions(_))
            ),
            "Yawgmoth proliferate should begin on a next-cost chooser"
        );

        wasm
    }

    let mut mana_wasm = setup_proliferate_prompt();
    mana_wasm
        .dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [0],
            }))
            .expect("next-cost mana choice should serialize"),
        )
        .expect("choosing Yawgmoth's mana cost should advance to mana payment");

    let mana_ctx = match mana_wasm.pending_decision.as_ref() {
        Some(DecisionContext::ManaPayment(ctx)) => ctx,
        other => panic!("expected authoritative mana payment after choosing mana, got {other:?}"),
    };
    assert!(
        mana_ctx.plan.mana_ability_steps.iter().any(|step| {
            mana_wasm
                .game
                .object(step.source)
                .is_some_and(|source| source.name == "Black Lotus")
        }),
        "the authoritative mana plan should select Black Lotus"
    );

    let mut discard_wasm = setup_proliferate_prompt();
    discard_wasm
        .dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [1],
            }))
            .expect("next-cost discard choice should serialize"),
        )
        .expect("choosing Yawgmoth's discard cost should advance to discard selection");

    let discard_ctx = match discard_wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => {
            panic!("expected discard selection prompt after choosing discard, got {other:?}")
        }
    };
    assert!(
        discard_ctx.description.to_lowercase().contains("discard"),
        "discard choice should advance to discard selection, got description: {}",
        discard_ctx.description
    );
    assert_eq!(discard_ctx.min, 1);
    assert_eq!(discard_ctx.max, Some(1));
}

#[test]
pub(super) fn stack_snapshot_includes_controller_and_targets() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let mountain_id = wasm
        .game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), alice, Zone::Hand);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let play_mountain_index = priority_ctx
        .actions
        .iter()
        .position(
            |action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == mountain_id),
        )
        .expect("expected play mountain action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": play_mountain_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("playing mountain should succeed");

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision after land play, got {other:?}"),
    };
    let cast_bolt_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == bolt_id
            )
        })
        .expect("expected cast lightning bolt action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_bolt_index,
        }))
        .expect("cast spell command should serialize"),
    )
    .expect("casting lightning bolt should enter its decision chain");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
        "lightning bolt cast should be waiting on targets"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "player", "player": bob.0 }
            ],
        }))
        .expect("target selection command should serialize"),
    )
    .expect("choosing the lightning bolt target should succeed");

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
        None,
        wasm.is_cancelable(),
        None,
        0,
    );
    let stack_entry = snapshot
        .stack_objects
        .first()
        .expect("snapshot should include the cast lightning bolt on the stack");

    assert_eq!(stack_entry.name, "Lightning Bolt");
    assert_eq!(stack_entry.controller, alice.0);
    assert_eq!(stack_entry.targets.len(), 1);
    match &stack_entry.targets[0] {
        TargetChoiceView::Player { player, name } => {
            assert_eq!(*player, bob.0);
            assert_eq!(name, "Bob");
        }
        other => panic!("expected player target on stack snapshot, got {other:?}"),
    }
}

#[test]
pub(super) fn wasm_stubborn_denial_can_target_and_counter_lightning_bolt() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let stubborn_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Stubborn Denial".to_string(), "hand".to_string(), true)
            .expect("Stubborn Denial should be loadable into Alice's hand"),
    );
    wasm.game
        .player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    let ferocious_creature = CardBuilder::new(CardId::new(), "Ferocious Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    wasm.game
        .create_object_from_card(&ferocious_creature, alice, Zone::Battlefield);

    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
    let bolt_stable_id = wasm
        .game
        .object(bolt_id)
        .expect("Lightning Bolt should exist on the stack")
        .stable_id;
    wasm.game
        .push_to_stack(StackEntry::new(bolt_id, bob).with_targets(vec![Target::Player(alice)]));

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == stubborn_id),
    );

    let targets_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Stubborn Denial target prompt, got {other:?}"),
    };
    assert!(
        targets_ctx
            .requirements
            .iter()
            .flat_map(|requirement| requirement.legal_targets.iter())
            .any(|target| *target == Target::Object(bolt_id)),
        "Stubborn Denial should expose Lightning Bolt as a legal noncreature spell target"
    );

    let snapshot_json = wasm
        .snapshot_json()
        .expect("target prompt snapshot should render");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot JSON should parse");
    let snapshot_targets = snapshot["decision"]["requirements"][0]["legal_targets"]
        .as_array()
        .expect("target prompt should serialize legal targets");
    assert!(
        snapshot_targets.iter().any(|target| {
            target["kind"].as_str() == Some("object")
                && target["object"].as_u64() == Some(bolt_id.0)
                && target["name"].as_str() == Some("Lightning Bolt")
        }),
        "WASM snapshot should expose Lightning Bolt as a clickable Stubborn Denial target: {snapshot_targets:?}"
    );

    dispatch_select_target_object(&mut wasm, bolt_id);

    for _ in 0..8 {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => {
                let option_index = ctx
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .map(|option| option.index)
                    .unwrap_or_else(|| panic!("expected a legal payment option, got {ctx:?}"));
                dispatch_select_options(&mut wasm, &[option_index]);
            }
            Some(DecisionContext::Priority(_)) => {
                dispatch_pass_priority(&mut wasm);
                if let Some(current_bolt_id) = wasm.game.find_object_by_stable_id(bolt_stable_id)
                    && wasm
                        .game
                        .object(current_bolt_id)
                        .is_some_and(|object| object.zone == Zone::Graveyard)
                {
                    break;
                }
            }
            Some(other) => panic!("unexpected Stubborn Denial follow-up decision: {other:?}"),
            None => break,
        }
    }

    let countered_bolt_id = wasm
        .game
        .find_object_by_stable_id(bolt_stable_id)
        .expect("countered Lightning Bolt should still be tracked");
    assert_eq!(
        wasm.game
            .object(countered_bolt_id)
            .expect("Lightning Bolt should still exist")
            .zone,
        Zone::Graveyard,
        "Stubborn Denial should counter Lightning Bolt through the WASM dispatch flow"
    );
    assert_eq!(
        wasm.game.player(alice).expect("Alice should exist").life,
        20,
        "countered Lightning Bolt should not resolve and damage Alice"
    );
}

#[test]
pub(super) fn wasm_dispatch_failed_counter_allows_protected_spell_to_resolve() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::SecondMain;
    wasm.game.turn.step = None;

    let goblin = CardBuilder::new(CardId::new(), "Raging Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let goblin_id = wasm
        .game
        .create_object_from_card(&goblin, alice, Zone::Battlefield);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), alice, Zone::Stack);
    wasm.game.add_temporary_spell_ability_grant(
        alice,
        bolt_id,
        ironsmith::target::ObjectFilter::instant_or_sorcery().cast_by(ironsmith::PlayerFilter::You),
        StaticAbility::cant_be_countered_ability(),
        1,
    );
    wasm.game
        .consume_temporary_spell_ability_grants_for_spell(bolt_id, alice);
    wasm.game.push_to_stack(
        StackEntry::new(bolt_id, alice)
            .with_targets(vec![Target::Object(goblin_id)])
            .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
                spec: ironsmith::target::ChooseSpec::AnyTarget,
                range: 0..1,
            }]),
    );

    let counterspell = CardDefinitionBuilder::new(CardId::new(), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell.")
        .expect("counter spell should parse");
    let counter_id = wasm
        .game
        .create_object_from_definition(&counterspell, alice, Zone::Stack);
    wasm.game.push_to_stack(
        StackEntry::new(counter_id, alice)
            .with_targets(vec![Target::Object(bolt_id)])
            .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
                spec: ironsmith::target::ChooseSpec::spell(),
                range: 0..1,
            }]),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

    assert!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .graveyard
            .iter()
            .any(|id| wasm
                .game
                .object(*id)
                .is_some_and(|object| object.name == "Raging Goblin")),
        "failed counter should leave the protected spell to deal lethal damage"
    );
}

#[test]
pub(super) fn duress_snapshot_keeps_revealed_hand_visible_during_discard_choice() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let duress_id = wasm
        .add_card_to_zone(0, "Duress".to_string(), "hand".to_string(), true)
        .expect("should add Duress to hand");
    wasm.add_card_to_zone(
        0,
        "Black Lotus".to_string(),
        "battlefield".to_string(),
        true,
    )
    .expect("should add Black Lotus to battlefield");

    let hydra_id = wasm
        .add_card_to_zone(1, "Ulvenwald Hydra".to_string(), "hand".to_string(), true)
        .expect("should add Ulvenwald Hydra to hand");
    let peek_id = wasm
        .add_card_to_zone(1, "Peek".to_string(), "hand".to_string(), true)
        .expect("should add Peek to hand");
    let keyrune_id = wasm
        .add_card_to_zone(1, "Dimir Keyrune".to_string(), "hand".to_string(), true)
        .expect("should add Dimir Keyrune to hand");
    let forest_id = wasm
        .add_card_to_zone(1, "Forest".to_string(), "hand".to_string(), true)
        .expect("should add Forest to hand");

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let cast_duress_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == ObjectId::from_raw(duress_id)
                )
            })
            .expect("expected cast Duress action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_duress_index,
        }))
        .expect("cast spell command should serialize"),
    )
    .expect("casting Duress should enter its decision chain");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
        "Duress should be waiting on targets after cast"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [
                { "kind": "player", "player": bob.0 }
            ],
        }))
        .expect("target selection command should serialize"),
    )
    .expect("choosing the Duress target should succeed");

    loop {
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(options)) => {
                let option_index = options
                    .options
                    .iter()
                    .find(|option| option.legal && option.description.contains("Black Lotus"))
                    .or_else(|| options.options.iter().find(|option| option.legal))
                    .map(|option| option.index)
                    .unwrap_or_else(|| {
                        panic!(
                            "expected a legal mana-payment option, got {:?}",
                            options
                                .options
                                .iter()
                                .map(|option| option.description.clone())
                                .collect::<Vec<_>>()
                        )
                    });
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": [option_index],
                    }))
                    .expect("option choice command should serialize"),
                )
                .expect("payment choice should succeed");
            }
            Some(DecisionContext::SelectObjects(_)) => break,
            Some(other) => panic!("unexpected Duress follow-up decision: {other:?}"),
            None => panic!("Duress resolved without presenting the discard decision"),
        }
    }

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

    let viewed_cards = snapshot
        .viewed_cards
        .as_ref()
        .expect("Duress discard prompt should keep revealed cards in snapshot");
    assert_eq!(viewed_cards.visibility, "public");
    assert_eq!(viewed_cards.subject, bob.0);
    assert_eq!(
        viewed_cards.card_ids,
        vec![hydra_id, peek_id, keyrune_id, forest_id],
        "snapshot should surface every revealed hand card, not only legal discard choices"
    );

    let decision = match snapshot
        .decision
        .as_ref()
        .expect("snapshot should include the pending discard choice")
    {
        super::DecisionView::SelectObjects { candidates, .. } => candidates,
        other => panic!("expected select_objects decision, got {other:?}"),
    };
    let candidate_ids: Vec<u64> = decision.iter().map(|candidate| candidate.id).collect();
    assert_eq!(
        candidate_ids,
        vec![peek_id, keyrune_id],
        "discard decision should only offer the legal noncreature nonland cards"
    );
}

#[test]
pub(super) fn gitaxian_probe_snapshot_keeps_looked_at_hand_visible_after_draw() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;
    wasm.runner_awaiting_priority = true;

    let probe = compile_to_runtime_definition(
        "Gitaxian Probe",
        "Mana Cost: {U/P}\nType: Sorcery\nLook at target player's hand.\nDraw a card.",
        false,
    )
    .expect("Gitaxian Probe should compile");
    let probe_id = wasm
        .game
        .create_object_from_definition(&probe, alice, Zone::Hand);
    wasm.game
        .create_object_from_definition(&basic_island(), alice, Zone::Library);
    let bolt_id = wasm
        .game
        .create_object_from_definition(&lightning_bolt(), bob, Zone::Hand);
    let mountain_id = wasm
        .game
        .create_object_from_definition(&basic_mountain(), bob, Zone::Hand);
    wasm.game
        .player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == probe_id),
    );
    dispatch_select_target_player(&mut wasm, bob);

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(options)) => {
            let option_index = options
                .options
                .iter()
                .find(|option| option.legal && option.description.contains("2 life"))
                .map(|option| option.index)
                .unwrap_or_else(|| {
                    panic!(
                        "expected phyrexian life payment option, got {:?}",
                        options
                            .options
                            .iter()
                            .map(|option| option.description.clone())
                            .collect::<Vec<_>>()
                    )
                });
            dispatch_select_options(&mut wasm, &[option_index]);
        }
        other => panic!("expected Gitaxian Probe payment option, got {other:?}"),
    }

    dispatch_pass_priority(&mut wasm);
    dispatch_pass_priority(&mut wasm);

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
        .expect("Gitaxian Probe should keep the looked-at hand in the next snapshot");
    assert_eq!(viewed_cards.visibility, "private");
    assert_eq!(viewed_cards.viewer, alice.0);
    assert_eq!(viewed_cards.subject, bob.0);
    assert_eq!(viewed_cards.zone, "Hand");
    assert_eq!(viewed_cards.card_ids, vec![bolt_id.0, mountain_id.0]);
    assert_eq!(
        viewed_cards
            .cards
            .iter()
            .map(|card| card.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Lightning Bolt", "Mountain"]
    );
}

#[test]
pub(super) fn stack_snapshot_keeps_reveal_cost_card_visible_while_spell_is_on_stack() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let revealed = CardBuilder::new(CardId::from_raw(701), "Merfolk Scout")
        .card_types(vec![CardType::Creature])
        .build();
    let revealed_id = game.create_object_from_card(&revealed, bob, Zone::Hand);

    let spell = CardBuilder::new(CardId::from_raw(702), "Silvergill Variant")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_card(&spell, bob, Zone::Stack);

    let snapshot = {
        let obj = game.object(revealed_id).expect("revealed hand card");
        ObjectSnapshot::from_object(obj, &game)
    };
    let mut tagged = std::collections::HashMap::new();
    tagged.insert(
        ironsmith::tag::TagKey::from(ironsmith::effects::PUBLIC_REVEALED_TAG),
        vec![snapshot],
    );
    game.push_to_stack(StackEntry::new(spell_id, bob).with_tagged_objects(tagged));

    let snapshot = GameSnapshot::from_game(
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

    let bob_snapshot = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("snapshot should include Bob");
    assert!(bob_snapshot.can_view_hand);
    assert!(
        bob_snapshot
            .hand_cards
            .iter()
            .any(|card| card.id == revealed_id.0),
        "revealed cost card should stay visible while the spell is on the stack"
    );

    let viewed = snapshot
        .viewed_cards
        .as_ref()
        .expect("stack reveal should populate viewed cards");
    assert_eq!(viewed.visibility, "public");
    assert_eq!(viewed.subject, bob.0);
    assert_eq!(viewed.card_ids, vec![revealed_id.0]);
}

#[test]
pub(super) fn stack_snapshot_keeps_hidden_zone_activation_source_visible() {
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source = CardBuilder::new(CardId::from_raw(703), "Street Wraith Variant")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&source, bob, Zone::Hand);
    let source_snapshot = {
        let obj = game.object(source_id).expect("hidden-zone source");
        ObjectSnapshot::from_object(obj, &game)
    };

    let entry = StackEntry::ability(
        source_id,
        bob,
        ironsmith::resolution::ResolutionProgram::default(),
    )
    .with_source_info(source_snapshot.stable_id, source_snapshot.name.clone())
    .with_source_snapshot(source_snapshot);
    game.push_to_stack(entry);

    let snapshot = GameSnapshot::from_game(
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

    let bob_snapshot = snapshot
        .players
        .iter()
        .find(|player| player.id == bob.0)
        .expect("snapshot should include Bob");
    assert!(bob_snapshot.can_view_hand);
    assert!(
        bob_snapshot
            .hand_cards
            .iter()
            .any(|card| card.id == source_id.0),
        "the source of an ability activated from hand should stay revealed on the stack"
    );
}

#[test]
pub(super) fn tayam_black_lotus_color_choice_keeps_paid_mana_state() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tayam_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield"),
    );
    let ornithopter_ids: Vec<ObjectId> = (0..3)
        .map(|_| {
            ObjectId::from_raw(
                wasm.add_card_to_zone(
                    alice.0,
                    "Ornithopter".to_string(),
                    "battlefield".to_string(),
                    false,
                )
                .expect("should add Ornithopter to battlefield"),
            )
        })
        .collect();
    let lotus_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Black Lotus".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Black Lotus to battlefield"),
    );

    for ornithopter_id in &ornithopter_ids {
        let ornithopter = wasm
            .game
            .object(*ornithopter_id)
            .expect("ornithopter should exist");
        assert_eq!(
            ornithopter
                .counters
                .get(&ironsmith::object::CounterType::Vigilance)
                .copied(),
            Some(1),
            "Tayam should grant each Ornithopter a vigilance counter"
        );
    }

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Tayam should begin its cost-payment chain");

    let next_cost_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected next-cost chooser after activating Tayam, got {other:?}"),
    };
    let mana_choice = next_cost_ctx
        .options
        .iter()
        .find(|option| option.legal && option.description.contains("Pay {3}"))
        .map(|option| option.index)
        .unwrap_or(0);

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [mana_choice],
        }))
        .expect("next-cost choice command should serialize"),
    )
    .expect("choosing Tayam's mana cost should advance to mana payment");

    let (plan_id, request_hash) = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::ManaPayment(ctx)) => {
            assert!(
                ctx.plan
                    .mana_ability_steps
                    .iter()
                    .any(|step| step.source == lotus_id),
                "the authoritative payment plan should select Black Lotus"
            );
            (ctx.plan.id.to_string(), ctx.plan.request_hash.to_string())
        }
        other => panic!("expected authoritative mana payment after choosing mana, got {other:?}"),
    };

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "mana_payment",
            "response": {
                "action": "confirm",
                "plan_id": plan_id,
                "request_hash": request_hash,
            },
        }))
        .expect("Black Lotus payment plan should serialize"),
    )
    .expect("confirming the Black Lotus payment plan should succeed");

    assert!(
        !wasm.game.battlefield.contains(&lotus_id),
        "Black Lotus should be sacrificed immediately once selected"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Colors(_))),
        "Black Lotus should surface a color-choice prompt"
    );

    let colors_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Colors(ctx)) => ctx,
        other => panic!("expected color-choice decision, got {other:?}"),
    };
    let green_option = colors_for_context(colors_ctx)
        .iter()
        .position(|color| *color == ironsmith::color::Color::Green)
        .expect("green should be a legal Black Lotus color choice");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [green_option],
        }))
        .expect("color choice command should serialize"),
    )
    .expect("choosing a Black Lotus color should replay the payment chain");

    assert!(
        !wasm.game.battlefield.contains(&lotus_id),
        "Black Lotus should remain sacrificed after the replayed color choice resolves"
    );
    let pool = &wasm
        .game
        .player(alice)
        .expect("alice should exist")
        .mana_pool;
    assert_eq!(
        pool.green, 3,
        "the full planned mana production should remain until the chosen mana cost is committed"
    );

    let pending_activation = wasm
        .priority_state
        .pending_activation
        .as_ref()
        .expect("Tayam activation should still be in progress");
    assert!(
        pending_activation.pending_mana_payment.is_some(),
        "the prepared whole-cost payment should remain attached to the activation"
    );
    assert!(
        matches!(
            wasm.pending_decision,
            Some(DecisionContext::SelectOptions(_))
        ),
        "after choosing the color, the UI should advance to the next payment prompt"
    );
}

#[test]
pub(super) fn tayam_counter_choice_keeps_removed_counters_state() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tayam_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield"),
    );
    let ornithopter_ids: Vec<ObjectId> = (0..3)
        .map(|_| {
            ObjectId::from_raw(
                wasm.add_card_to_zone(
                    alice.0,
                    "Ornithopter".to_string(),
                    "battlefield".to_string(),
                    false,
                )
                .expect("should add Ornithopter to battlefield"),
            )
        })
        .collect();

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Tayam should begin its cost-payment chain");

    let next_cost_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected next-cost chooser after activating Tayam, got {other:?}"),
    };
    let counter_choice = next_cost_ctx
        .options
        .iter()
        .find(|option| option.legal && option.description.contains("Remove three counters"))
        .map(|option| option.index)
        .unwrap_or(1);

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [counter_choice],
        }))
        .expect("counter-cost choice command should serialize"),
    )
    .expect("choosing Tayam's counter cost should open distribution");

    let distribute_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Distribute(ctx)) => ctx,
        other => panic!("expected counter distribution prompt, got {other:?}"),
    };
    let distribution_indices: Vec<usize> = ornithopter_ids
        .iter()
        .map(|ornithopter_id| {
            distribute_ctx
                .targets
                .iter()
                .position(|target| target.target == Target::Object(*ornithopter_id))
                .expect("each Ornithopter should be a legal distribution target")
        })
        .collect();

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": distribution_indices,
        }))
        .expect("distribution command should serialize"),
    )
    .expect("distributing Tayam's counters across the Ornithopters should succeed");

    for ornithopter_id in &ornithopter_ids {
        let counters_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Counters(ctx)) => ctx,
            other => panic!("expected counter-removal prompt, got {other:?}"),
        };
        assert_eq!(
            counters_ctx.target, *ornithopter_id,
            "counter-removal replay should advance through the distributed targets in order"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [0],
            }))
            .expect("counter selection command should serialize"),
        )
        .expect("removing the selected vigilance counter should succeed");

        let ornithopter = wasm
            .game
            .object(*ornithopter_id)
            .expect("ornithopter should still exist");
        assert_eq!(
            ornithopter
                .counters
                .get(&ironsmith::object::CounterType::Vigilance)
                .copied()
                .unwrap_or(0),
            0,
            "selected Ornithopter should keep its counter removed after replay"
        );
    }

    let pending_activation = wasm
        .priority_state
        .pending_activation
        .as_ref()
        .expect("Tayam activation should still be in progress");
    assert!(
        pending_activation.remaining_cost_steps.is_empty(),
        "after removing all three counters, the counter-payment step should be complete"
    );
    assert!(
        matches!(
            wasm.pending_decision,
            Some(DecisionContext::SelectOptions(_))
        ),
        "after paying the counter cost, the UI should advance to the remaining mana payment"
    );
}

#[test]
pub(super) fn tayam_activation_can_resolve_and_choose_graveyard_return_target() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let tayam_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Tayam, Luminous Enigma".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Tayam to battlefield"),
    );
    let wall_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Wall of Roots".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add Wall of Roots to battlefield"),
    );
    let ornithopter_id = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add Ornithopter to battlefield"),
    );
    let forest_a = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Forest".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add first Forest to battlefield"),
    );
    let forest_b = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Forest".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("should add second Forest to battlefield"),
    );
    let return_target = ObjectId::from_raw(
        wasm.add_card_to_zone(
            alice.0,
            "Forest".to_string(),
            "graveyard".to_string(),
            false,
        )
        .expect("should add return target to graveyard"),
    );

    assert!(
        wasm.game.player(bob).is_some(),
        "second player should exist for priority passing"
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Tayam should begin its cost-payment chain");

    loop {
        let pending = wasm
            .pending_decision
            .clone()
            .expect("Tayam activation should still have a pending decision");
        match pending {
            DecisionContext::SelectOptions(ctx) => {
                let choice = if ctx.description.contains("Choose next cost") {
                    ctx.options
                        .iter()
                        .find(|option| option.legal && option.description.contains("Pay {3}"))
                        .map(|option| option.index)
                        .expect("next-cost chooser should offer the mana payment")
                } else if ctx.description.contains("Pay mana") {
                    if let Some(option) = ctx
                        .options
                        .iter()
                        .find(|option| option.legal && option.description.contains("Wall of Roots"))
                    {
                        option.index
                    } else {
                        ctx.options
                            .iter()
                            .find(|option| option.legal && option.description.contains("Forest"))
                            .map(|option| option.index)
                            .expect("mana payment prompt should offer a legal mana source")
                    }
                } else if ctx.description.contains("Choose next cost") {
                    unreachable!("handled above")
                } else {
                    ctx.options
                        .iter()
                        .find(|option| {
                            option.legal && option.description.contains("Remove three counters")
                        })
                        .map(|option| option.index)
                        .or_else(|| {
                            ctx.options
                                .iter()
                                .find(|option| option.legal && option.description.contains("Pass"))
                                .map(|option| option.index)
                        })
                        .unwrap_or_else(|| {
                            ctx.options
                                .iter()
                                .find(|option| option.legal)
                                .map(|option| option.index)
                                .expect("select-options prompt should offer a legal choice")
                        })
                };

                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": [choice],
                    }))
                    .expect("select-options command should serialize"),
                )
                .expect("dispatching Tayam select-options step should succeed");
            }
            DecisionContext::Distribute(ctx) => {
                let wall_index = ctx
                    .targets
                    .iter()
                    .position(|target| target.target == Target::Object(wall_id))
                    .expect("Wall of Roots should be a legal distribution target");
                let ornithopter_index = ctx
                    .targets
                    .iter()
                    .position(|target| target.target == Target::Object(ornithopter_id))
                    .expect("Ornithopter should be a legal distribution target");
                let indices = vec![wall_index, wall_index, ornithopter_index];
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": indices,
                    }))
                    .expect("distribute command should serialize"),
                )
                .expect("counter distribution should succeed");
            }
            DecisionContext::Counters(ctx) => {
                let counter_index = ctx
                    .available_counters
                    .iter()
                    .position(|(_, available)| *available > 0)
                    .expect("counter prompt should offer at least one removable counter");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_options",
                        "option_indices": [counter_index],
                    }))
                    .expect("counter selection command should serialize"),
                )
                .expect("counter removal should succeed");
            }
            DecisionContext::Priority(ctx) => {
                let pass_index = ctx
                    .actions
                    .iter()
                    .position(|action| matches!(action, LegalAction::PassPriority))
                    .expect("priority prompt should include pass");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "priority_action",
                        "action_index": pass_index,
                    }))
                    .expect("priority pass command should serialize"),
                )
                .expect("priority pass during Tayam line should succeed");
            }
            DecisionContext::SelectObjects(ctx) => {
                let target_id = ctx
                    .candidates
                    .iter()
                    .find(|candidate| candidate.legal && candidate.id == return_target)
                    .map(|candidate| candidate.id.0)
                    .expect("graveyard return target should be legal");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_objects",
                        "object_ids": [target_id],
                    }))
                    .expect("graveyard target command should serialize"),
                )
                .expect("selecting Tayam's graveyard return target should succeed");
                break;
            }
            other => panic!("unexpected Tayam resolution decision: {other:?}"),
        }
    }

    assert!(
        !wasm.game.battlefield.contains(&forest_a) || !wasm.game.battlefield.contains(&forest_b),
        "at least one Forest should remain tapped after paying Tayam's mana cost"
    );
    assert!(
        wasm.game.battlefield.iter().any(|id| {
            wasm.game
                .object(*id)
                .is_some_and(|obj| obj.name == "Forest" && obj.owner == alice)
        }),
        "a Forest should still exist on the battlefield after Tayam resolves"
    );
}

#[test]
pub(super) fn polluted_delta_resolution_choice_keeps_paid_costs_and_resolved_land() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let delta_id =
        wasm.game
            .create_object_from_definition(&polluted_delta(), alice, Zone::Battlefield);
    let island_id = wasm
        .game
        .create_object_from_definition(&basic_island(), alice, Zone::Library);
    wasm.game
        .create_object_from_definition(&basic_mountain(), alice, Zone::Library);

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let priority_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx,
        other => panic!("expected priority decision, got {other:?}"),
    };
    let activate_index = priority_ctx
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == delta_id
            )
        })
        .expect("expected Polluted Delta activation action");

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": activate_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("activating Polluted Delta should succeed");

    assert!(
        wasm.game.player(bob).is_some(),
        "second player should exist for the pass-priority sequence"
    );
    assert!(
        !wasm.game.battlefield.contains(&delta_id),
        "Polluted Delta should be sacrificed during activation"
    );
    assert!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .graveyard
            .contains(&delta_id),
        "Polluted Delta should be in the graveyard after activation"
    );
    assert_eq!(
        wasm.game.player(alice).expect("alice should exist").life,
        19,
        "Polluted Delta activation should pay 1 life immediately"
    );

    loop {
        let pending = wasm
            .pending_decision
            .clone()
            .expect("fetchland line should keep producing prompts until the search resolves");
        match pending {
            DecisionContext::Priority(ctx) => {
                let pass_index = ctx
                    .actions
                    .iter()
                    .position(|action| matches!(action, LegalAction::PassPriority))
                    .expect("priority prompt should include pass");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "priority_action",
                        "action_index": pass_index,
                    }))
                    .expect("priority pass command should serialize"),
                )
                .expect("passing priority during fetchland line should succeed");
            }
            DecisionContext::SelectObjects(ctx) => {
                let choice = ctx
                    .candidates
                    .iter()
                    .find(|candidate| candidate.legal && candidate.id == island_id)
                    .map(|candidate| candidate.id.0)
                    .expect("basic Island should be a legal fetchland search result");
                wasm.dispatch(
                    serde_wasm_bindgen::to_value(&json!({
                        "type": "select_objects",
                        "object_ids": [choice],
                    }))
                    .expect("fetchland selection command should serialize"),
                )
                .expect("choosing the searched land should succeed");
                break;
            }
            other => panic!("unexpected Polluted Delta follow-up decision: {other:?}"),
        }
    }

    assert_eq!(
        wasm.game.player(alice).expect("alice should exist").life,
        19,
        "resolving the fetchland search should not rewind the paid life cost"
    );
    assert!(
        !wasm.game.battlefield.contains(&delta_id),
        "resolving the fetchland search should not put Polluted Delta back onto the battlefield"
    );
    assert!(
        wasm.game
            .player(alice)
            .expect("alice should exist")
            .graveyard
            .contains(&delta_id),
        "Polluted Delta should remain in the graveyard after the search completes"
    );
    assert!(
        wasm.game.battlefield.contains(&island_id),
        "the chosen Island should enter the battlefield"
    );
    assert!(
        !wasm
            .game
            .player(alice)
            .expect("alice should exist")
            .library
            .contains(&island_id),
        "the chosen Island should leave the library after resolution"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "after the search resolves, the game should return to priority"
    );
}

#[test]
pub(super) fn committed_resolution_prompt_is_not_cancelable() {
    let mut wasm = WasmGame::new();
    let alice = PlayerId::from_index(0);
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.priority_epoch_has_undoable_action = true;
    wasm.pending_decision = Some(DecisionContext::SelectObjects(SelectObjectsContext::new(
        alice,
        None,
        "Resolve effect",
        vec![SelectableObject::new(ObjectId::from_raw(1), "Choice")],
        1,
        Some(1),
    )));
    assert!(
        wasm.pending_action_checkpoint.is_none(),
        "committed follow-up prompts should not retain the action-chain undo checkpoint"
    );
    assert!(
        !wasm.is_cancelable(),
        "once the spell has resolved into its imprint prompt, undo should be disabled"
    );
    assert!(
        wasm.cancel_decision().is_err(),
        "non-cancelable prompts must reject direct cancelDecision calls"
    );
}

#[test]
pub(super) fn emrakul_cast_trigger_needs_targets_in_four_player_game() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dana = PlayerId::from_index(3);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    assert_eq!(
        wasm.trigger_queue.entries.len(),
        1,
        "Emrakul should queue its cast trigger from the stack"
    );

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");

    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    assert_eq!(
        targets_ctx.player, alice,
        "the caster should choose Emrakul's target opponent"
    );
    assert_eq!(
        targets_ctx.requirements.len(),
        1,
        "Emrakul should ask for exactly one target requirement"
    );

    let legal_targets = &targets_ctx.requirements[0].legal_targets;
    let legal_players: Vec<PlayerId> = legal_targets
        .iter()
        .filter_map(|target| match target {
            ironsmith::game_state::Target::Player(player) => Some(*player),
            ironsmith::game_state::Target::Object(_) => None,
        })
        .collect();
    assert_eq!(
        legal_players,
        vec![bob, charlie, dana],
        "all opponents should be legal Emrakul targets"
    );

    assert_eq!(
        wasm.game.stack.len(),
        1,
        "replay should leave the live game advanced to the pending target decision"
    );
}

#[test]
pub(super) fn auto_advance_target_prompt_dispatch_reexecutes_replay_root() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");
    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    wasm.pending_decision = Some(DecisionContext::Targets(targets_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_targets",
            "targets": [{ "kind": "player", "player": bob.0 }],
        }))
        .expect("target selection should serialize"),
    )
    .expect("dispatching replay-backed targets should succeed");

    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "after choosing Emrakul's target, auto-advance should continue to priority"
    );
    assert_eq!(
        wasm.game.stack.len(),
        2,
        "choosing the trigger target should put Emrakul's cast trigger onto the stack"
    );
}

#[test]
pub(super) fn auto_advance_legend_rule_prompt_dispatch_applies_live_state_without_root_replay() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let alice = PlayerId::from_index(0);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let legend = CardBuilder::new(CardId::from_raw(90_300), "Scale Probe Relic")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact])
        .build();
    let keep_id = wasm
        .game
        .create_object_from_card(&legend, alice, Zone::Battlefield);
    let put_away_id = wasm
        .game
        .create_object_from_card(&legend, alice, Zone::Battlefield);

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach the legend-rule prompt");
    let legend_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::SelectObjects(ctx)) => ctx,
        other => panic!("expected legend-rule select_objects prompt, got {other:?}"),
    };
    assert!(
        legend_ctx.description.contains("legend rule"),
        "prompt should be the legend-rule decision"
    );

    wasm.pending_decision = Some(DecisionContext::SelectObjects(legend_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    dispatch_select_objects(&mut wasm, &[keep_id.0]);

    assert!(
        wasm.pending_replay_action.is_none(),
        "legend-rule choice should not leave the replay chain open"
    );
    assert!(
        wasm.game.battlefield.contains(&keep_id),
        "chosen legend should remain on the battlefield"
    );
    assert!(
        !wasm.game.battlefield.contains(&put_away_id),
        "unchosen duplicate should leave the battlefield"
    );
    assert!(
        wasm.game.object(put_away_id).is_none(),
        "the original duplicate object should leave the battlefield as a zone-change object"
    );
    assert!(
        matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
        "after resolving the legend rule, auto-advance should resume to priority"
    );
}

#[test]
pub(super) fn emrakul_target_prompt_snapshot_shows_pending_triggered_ability() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");
    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    wasm.pending_decision = Some(DecisionContext::Targets(targets_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    let snapshot_json = wasm
        .snapshot_json()
        .expect("snapshot json should render pending Emrakul trigger");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");

    let stack_objects = snapshot["stack_objects"]
        .as_array()
        .expect("snapshot should include stack objects");
    assert_eq!(
        stack_objects.len(),
        2,
        "snapshot should show spell plus cast trigger"
    );
    assert_eq!(stack_objects[0]["name"], "Emrakul, the Promised End");
    assert_eq!(stack_objects[0]["ability_kind"], "Triggered");
    assert!(
        stack_objects[0]["ability_text"]
            .as_str()
            .is_some_and(|text| text.to_ascii_lowercase().contains("target opponent")),
        "pending trigger snapshot should describe Emrakul's cast trigger"
    );
    assert_eq!(stack_objects[1]["name"], "Emrakul, the Promised End");
    assert!(
        stack_objects[1]["ability_kind"].is_null(),
        "the second stack object should remain the Emrakul spell"
    );
}

#[test]
pub(super) fn emrakul_target_prompt_snapshot_encodes_for_js_with_safe_stack_ids() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
        1,
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let emrakul_id =
        wasm.game
            .create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = wasm
        .game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.clone()))
        .expect("Emrakul spell object should exist");
    wasm.game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    for trigger in check_triggers(&wasm.game, &event) {
        wasm.trigger_queue.add(trigger);
    }

    let checkpoint = wasm.capture_replay_checkpoint();
    let outcome = wasm
        .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
        .expect("auto-advance should reach Emrakul's trigger decision");
    let targets_ctx = match outcome {
        ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
        other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
    };

    wasm.pending_decision = Some(DecisionContext::Targets(targets_ctx));
    wasm.pending_replay_action = Some(PendingReplayAction {
        checkpoint,
        root: ReplayRoot::Advance,
        nested_answers: Vec::new(),
    });

    let snapshot_value = wasm
        .snapshot()
        .expect("snapshot should encode for JS with safe stack ids");
    let snapshot: serde_json::Value =
        serde_wasm_bindgen::from_value(snapshot_value).expect("snapshot value should parse");
    let stack_objects = snapshot["stack_objects"]
        .as_array()
        .expect("snapshot should include stack objects");

    assert_eq!(
        stack_objects.len(),
        2,
        "snapshot should keep both stack entries"
    );
    for entry in stack_objects {
        let id = entry["id"]
            .as_u64()
            .expect("stack entry id should be a JS-safe integer");
        assert!(
            id <= 9_007_199_254_740_991,
            "stack entry id should stay within JS safe integer range, got {id}"
        );
    }

    let triggered_id = stack_objects[0]["id"]
        .as_u64()
        .expect("triggered ability id should exist");
    let spell_id = stack_objects[1]["id"]
        .as_u64()
        .expect("spell id should exist");
    assert_ne!(
        triggered_id, spell_id,
        "triggered ability and spell should keep distinct UI ids"
    );
}

#[test]
pub(super) fn target_prompt_snapshot_shows_all_queued_targeted_triggers_while_spell_resolves() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;

    let blood_artist_id =
        wasm.game
            .create_object_from_definition(&blood_artist(), alice, Zone::Battlefield);
    let victim_id =
        wasm.game
            .create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    let victim_snapshot = wasm
        .game
        .object(victim_id)
        .map(|object| ironsmith::snapshot::ObjectSnapshot::from_object(object, &wasm.game))
        .expect("victim snapshot should exist");
    let dies_event = TriggerEvent::new_with_provenance(
        ironsmith::events::ZoneChangeEvent::with_cause(
            victim_id,
            Zone::Battlefield,
            Zone::Graveyard,
            ironsmith::events::cause::EventCause::from_sba(),
            Some(victim_snapshot),
        ),
        ProvNodeId::default(),
    );

    let trigger = check_triggers(&wasm.game, &dies_event)
        .into_iter()
        .find(|entry| entry.source == blood_artist_id)
        .expect("Blood Artist should trigger when another creature dies");
    wasm.trigger_queue.add(trigger.clone());
    wasm.trigger_queue.add(trigger);

    let culling_id =
        wasm.game
            .create_object_from_definition(&culling_the_weak(), alice, Zone::Stack);
    let culling_snapshot = build_stack_object_snapshot(
        &wasm.game,
        wasm.perspective,
        None,
        &StackEntry::new(culling_id, alice),
    );
    wasm.active_resolving_stack_object = Some(culling_snapshot);

    wasm.pending_decision = Some(DecisionContext::Targets(TargetsContext::new(
        alice,
        blood_artist_id,
        "Blood Artist's triggered ability".to_string(),
        vec![TargetRequirementContext {
            description: "target for Blood Artist".to_string(),
            legal_targets: vec![Target::Player(alice), Target::Player(bob)],
            legal_target_sets: Vec::new(),
            min_targets: 1,
            max_targets: Some(1),
            distinct_player_group: None,
        }],
    )));

    let snapshot_json = wasm
        .snapshot_json()
        .expect("snapshot should render queued Blood Artist triggers");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");

    let stack_objects = snapshot["stack_objects"]
        .as_array()
        .expect("snapshot should include queued stack objects");
    assert_eq!(
        stack_objects.len(),
        2,
        "snapshot should show both queued Blood Artist triggers"
    );
    assert!(
        stack_objects
            .iter()
            .all(|entry| entry["name"] == "Blood Artist" && entry["ability_kind"] == "Triggered"),
        "queued stack objects should both be Blood Artist triggers: {stack_objects:?}"
    );
    assert_ne!(
        stack_objects[0]["id"], stack_objects[1]["id"],
        "queued trigger previews should keep distinct UI ids"
    );

    let resolving = snapshot["resolving_stack_object"]
        .as_object()
        .expect("resolving spell should remain visible separately");
    assert_eq!(resolving["name"], "Culling the Weak");
}

#[test]
pub(super) fn roaming_throne_blood_artist_culling_flow_reaches_two_trigger_ordering_options() {
    let mut wasm = WasmGame::new();

    let alice = PlayerId::from_index(0);

    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.step = None;
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    wasm.add_card_to_zone(
        0,
        "Roaming Throne".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("should start Roaming Throne battlefield entry");

    let vampire_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx
            .options
            .iter()
            .find(|option| option.description == "Vampire")
            .map(|option| option.index)
            .expect("Vampire should be a legal creature type"),
        other => panic!("expected Roaming Throne type selection, got {other:?}"),
    };
    dispatch_select_options(&mut wasm, &[vampire_index]);

    wasm.add_card_to_zone(
        0,
        "Blood Artist".to_string(),
        "battlefield".to_string(),
        false,
    )
    .expect("should add Blood Artist to the battlefield");

    let culling_id = wasm
        .add_card_to_zone(0, "Culling the Weak".to_string(), "hand".to_string(), false)
        .expect("should add Culling the Weak to hand");

    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));
    dispatch_matching_priority_action(
        &mut wasm,
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == ObjectId::from_raw(culling_id)),
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => {
            let blood_artist_id = wasm
                .game
                .battlefield
                .iter()
                .find_map(|id| {
                    wasm.game
                        .object(*id)
                        .filter(|obj| obj.name == "Blood Artist")
                        .map(|_| *id)
                })
                .expect("Blood Artist should be on the battlefield");
            dispatch_select_objects(&mut wasm, &[blood_artist_id.0]);
        }
        other => panic!("expected sacrifice target prompt for Culling the Weak, got {other:?}"),
    }

    let order_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Order(ctx)) => ctx,
        other => {
            panic!("expected trigger ordering prompt after sacrificing Blood Artist, got {other:?}")
        }
    };
    assert_eq!(
        order_ctx.items.len(),
        2,
        "Roaming Throne should create two Blood Artist ordering items"
    );
    assert!(
        order_ctx
            .items
            .iter()
            .all(|(_, label)| label.starts_with("Blood Artist\n")),
        "ordering labels should both be Blood Artist triggers: {:?}",
        order_ctx.items
    );

    let snapshot_json = wasm
        .snapshot_json()
        .expect("snapshot json should encode trigger ordering state");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_json).expect("snapshot json should parse");
    let decision = snapshot["decision"]
        .as_object()
        .expect("snapshot should include ordering decision");
    assert_eq!(decision["kind"], "select_options");
    assert_eq!(decision["reason"], "Order triggers");
    assert_eq!(
        decision["options"]
            .as_array()
            .expect("ordering decision should expose options")
            .len(),
        2,
        "UI decision payload should keep both Blood Artist trigger ordering options"
    );
    assert!(
        decision["options"]
            .as_array()
            .expect("ordering decision should expose options")
            .iter()
            .all(|option| option["description"]
                .as_str()
                .is_some_and(|description| description.starts_with("Blood Artist\n"))),
        "synthetic trigger-order options should expose their public labels: {decision:?}"
    );
}

#[test]
pub(super) fn priority_decision_routing_uses_replay_for_generic_modal_choices() {
    let boolean = DecisionContext::Boolean(BooleanContext::new(
        PlayerId::from_index(0),
        None,
        "play an additional land this turn",
    ));
    let number = DecisionContext::Number(NumberContext::new(
        PlayerId::from_index(0),
        None,
        0,
        3,
        "choose a number",
    ));
    let targets = DecisionContext::Targets(TargetsContext::new(
        PlayerId::from_index(0),
        ObjectId::from_raw(1),
        "resolve trigger",
        vec![TargetRequirementContext {
            description: "target player".to_string(),
            legal_targets: vec![Target::Player(PlayerId::from_index(1))],
            legal_target_sets: Vec::new(),
            min_targets: 1,
            max_targets: Some(1),
            distinct_player_group: None,
        }],
    ));
    let select_objects = DecisionContext::SelectObjects(SelectObjectsContext::new(
        PlayerId::from_index(0),
        None,
        "choose a land",
        vec![SelectableObject::new(ObjectId::from_raw(1), "Forest")],
        1,
        Some(1),
    ));
    let select_options =
        DecisionContext::SelectOptions(ironsmith::decisions::context::SelectOptionsContext::new(
            PlayerId::from_index(0),
            None,
            "choose a mode",
            vec![SelectableOption::new(0, "Only option")],
            1,
            1,
        ));
    let wasm = WasmGame::new();

    assert!(
        wasm.decision_requires_root_reexecution(&boolean),
        "boolean prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&number),
        "generic number prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&targets),
        "generic target prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&select_objects),
        "resolution-time object prompts should replay from the original root response"
    );
    assert!(
        wasm.decision_requires_root_reexecution(&select_options),
        "generic select-options prompts should replay from the original root response"
    );
    assert!(
        !wasm.decision_uses_live_priority_response(&select_options),
        "generic select-options prompts should route through replay continuations, not the live priority responder"
    );
    assert!(
        !wasm.decision_uses_live_priority_response(&number),
        "generic number prompts should not route through the live priority responder"
    );
    assert!(
        !wasm.decision_uses_live_priority_response(&targets),
        "generic target prompts should not route through the live priority responder"
    );
}

#[test]
pub(super) fn priority_decision_routing_keeps_cost_option_prompts_on_live_responder() {
    let mut wasm = WasmGame::new();
    wasm.priority_state.pending_cast = Some(PendingCast::new(
        ObjectId::from_raw(1),
        Zone::Hand,
        PlayerId::from_index(0),
        ProvNodeId::default(),
        CastStage::ChoosingOptionalCosts,
        None,
        Vec::new(),
        CastingMethod::Normal,
        OptionalCostsPaid::new(1),
        None,
        ObjectId::from_raw(1),
    ));

    let select_options =
        DecisionContext::SelectOptions(ironsmith::decisions::context::SelectOptionsContext::new(
            PlayerId::from_index(0),
            Some(ObjectId::from_raw(1)),
            "Choose optional costs",
            vec![SelectableOption::new(0, "Kicker")],
            0,
            1,
        ));

    assert!(
        wasm.decision_uses_live_priority_response(&select_options),
        "cost-selection select-options prompts should stay on the live priority responder"
    );
}

#[test]
pub(super) fn backdraft_wasm_flow_offers_resolved_sorcery_history_choice() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

    let alice = PlayerId::from_index(0);

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
    .expect("should add Omniscience to battlefield");
    for _ in 0..3 {
        wasm.add_card_to_zone(
            0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Ornithopter to battlefield");
    }

    let blasphemous_act_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Blasphemous Act".to_string(), "hand".to_string(), true)
            .expect("should add Blasphemous Act to hand"),
    );
    let backdraft_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Backdraft".to_string(), "hand".to_string(), true)
            .expect("should add Backdraft to hand"),
    );

    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
        alice,
        compute_legal_actions(&wasm.game, alice),
    )));

    let cast_blasphemous_act_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == blasphemous_act_id
                )
            })
            .expect("expected cast Blasphemous Act action"),
        other => {
            panic!("expected priority decision before casting Blasphemous Act, got {other:?}")
        }
    };

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_blasphemous_act_index,
        }))
        .expect("cast Blasphemous Act command should serialize"),
    )
    .expect("casting Blasphemous Act should succeed");

    for _ in 0..4 {
        let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else {
            break;
        };
        let pass_index = ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PassPriority))
            .expect("priority prompt should include pass");
        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": pass_index,
            }))
            .expect("priority pass command should serialize"),
        )
        .expect("passing priority during Blasphemous Act should succeed");
        if wasm.game.stack.is_empty() {
            break;
        }
    }

    assert!(
        wasm.game.stack.is_empty(),
        "Blasphemous Act should be resolved before casting Backdraft"
    );
    let history_after_blasphemous = wasm
        .game
        .turn_store
        .turn_history
        .spell_cast_snapshot_history();
    let blasphemous_snapshots = history_after_blasphemous
        .iter()
        .filter(|snapshot| snapshot.name == "Blasphemous Act")
        .collect::<Vec<_>>();
    assert_eq!(
        blasphemous_snapshots.len(),
        1,
        "expected Blasphemous Act cast history to persist after resolution, got {:?}",
        history_after_blasphemous
            .iter()
            .map(|snapshot| (
                snapshot.name.clone(),
                snapshot.zone,
                snapshot.card_types.clone(),
                snapshot.cast_order_this_turn
            ))
            .collect::<Vec<_>>()
    );
    let blasphemous_cast_id = blasphemous_snapshots[0].object_id;
    assert_eq!(
        wasm.game
            .turn_store
            .turn_history
            .damage_dealt_by_spell_this_turn(wasm.game.provenance_graph(), blasphemous_cast_id),
        39,
        "Blasphemous Act should record 39 total damage from the three Ornithopters"
    );

    let cast_backdraft_index = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx
                .actions
                .iter()
                .position(|action| {
                    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == backdraft_id)
                })
                .expect("expected cast Backdraft action"),
            other => panic!("expected priority decision before casting Backdraft, got {other:?}"),
        };

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "priority_action",
            "action_index": cast_backdraft_index,
        }))
        .expect("cast Backdraft command should serialize"),
    )
    .expect("casting Backdraft should succeed");

    for _ in 0..4 {
        let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else {
            break;
        };
        let pass_index = ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PassPriority))
            .expect("priority prompt should include pass");
        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": pass_index,
            }))
            .expect("priority pass command should serialize"),
        )
        .expect("passing priority during Backdraft should succeed");
        if !matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))) {
            break;
        }
    }

    let first_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected Backdraft to stop for a player choice, got {other:?}"),
    };
    let first_legal = first_choice
        .options
        .iter()
        .filter(|option| option.legal)
        .collect::<Vec<_>>();
    assert_eq!(
        first_legal.len(),
        1,
        "expected only Alice to qualify for Backdraft's player choice, got {:?}",
        first_choice
            .options
            .iter()
            .map(|option| (option.index, option.description.clone(), option.legal))
            .collect::<Vec<_>>()
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [first_legal[0].index],
        }))
        .expect("single-player choice command should serialize"),
    )
    .expect("choosing the only qualifying Backdraft player should succeed");

    let spell_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx,
        other => panic!("expected Backdraft to prompt for the historical spell, got {other:?}"),
    };
    let legal_spell_descriptions = spell_choice
        .options
        .iter()
        .filter(|option| option.legal)
        .map(|option| option.description.clone())
        .collect::<Vec<_>>();
    assert!(
        legal_spell_descriptions
            .iter()
            .any(|description| description.contains("Blasphemous Act")),
        "expected Blasphemous Act to remain a legal Backdraft history choice, got {:?}",
        legal_spell_descriptions
    );
    assert!(
        legal_spell_descriptions
            .iter()
            .any(|description| description.contains("Backdraft")),
        "expected Backdraft to also be present in the history choice, got {:?}",
        legal_spell_descriptions
    );
}

#[test]
pub(super) fn cultivator_colossus_etb_does_not_repeat_may_prompt_before_next_land_choice() {
    let mut wasm = WasmGame::new();

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

    let first_may = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => ctx,
        other => panic!("expected Cultivator Colossus may prompt, got {other:?}"),
    };
    assert!(
        first_may
            .description
            .to_ascii_lowercase()
            .contains("put a land card from your hand onto the battlefield tapped"),
        "expected Cultivator Colossus may text, got {:?}",
        first_may.description
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("yes choice should serialize"),
    )
    .expect("accepting the first Cultivator iteration should succeed");

    let first_land_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => panic!("expected first land selection prompt, got {other:?}"),
    };
    let mut first_candidates: Vec<u64> = first_land_choice
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id.0)
        .collect();
    first_candidates.sort_unstable();
    assert_eq!(
        first_candidates,
        vec![forest_a, forest_b],
        "first land selection should offer both lands in hand"
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_objects",
            "object_ids": [forest_a],
        }))
        .expect("first land selection should serialize"),
    )
    .expect("choosing the first land should succeed");

    assert_eq!(
        wasm.game
            .player(PlayerId::from_index(0))
            .expect("player should exist")
            .hand
            .len(),
        1,
        "after choosing a land, the live game state should keep that land out of hand"
    );
    let lands_on_battlefield = wasm
        .game
        .battlefield
        .iter()
        .filter(|&&id| {
            wasm.game
                .object(id)
                .is_some_and(|object| object.is_land() && object.owner == PlayerId::from_index(0))
        })
        .count();
    assert_eq!(
        lands_on_battlefield, 1,
        "the chosen land should already be on the battlefield before the next repeat decision"
    );

    let second_may = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => ctx,
        other => panic!("expected second Cultivator may prompt, got {other:?}"),
    };
    assert!(
        second_may
            .description
            .to_ascii_lowercase()
            .contains("put a land card from your hand onto the battlefield tapped"),
        "expected repeated Cultivator may text, got {:?}",
        second_may.description
    );

    wasm.dispatch(
        serde_wasm_bindgen::to_value(&json!({
            "type": "select_options",
            "option_indices": [1],
        }))
        .expect("second yes choice should serialize"),
    )
    .expect("accepting the second Cultivator iteration should succeed");

    let second_land_choice = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => panic!("expected second land selection prompt, got {other:?}"),
    };
    let second_candidates: Vec<u64> = second_land_choice
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id.0)
        .collect();
    assert_eq!(
        second_candidates,
        vec![forest_b],
        "after one land is chosen, the next prompt should go straight to the remaining land"
    );
}

#[test]
pub(super) fn doubling_chant_same_name_search_prompts_are_ui_friendly_in_wasm_flow() {
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
    let battlefield_ornithopter = ObjectId::from_raw(
        wasm.add_card_to_zone(
            0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("battlefield Ornithopter should be added"),
    );
    let library_ornithopter_a = wasm
        .add_card_to_zone(0, "Ornithopter".to_string(), "library".to_string(), true)
        .expect("first library Ornithopter should be added");
    let library_ornithopter_b = wasm
        .add_card_to_zone(0, "Ornithopter".to_string(), "library".to_string(), true)
        .expect("second library Ornithopter should be added");
    let spell_id = ObjectId::from_raw(
        wasm.add_card_to_zone(0, "Doubling Chant".to_string(), "hand".to_string(), true)
            .expect("Doubling Chant should be added to hand"),
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

    let free_cast_index = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectOptions(ctx)) => ctx
            .options
            .iter()
            .position(|option| option.description.contains("Without paying mana cost"))
            .expect("Doubling Chant should surface an Omniscience cast option"),
        other => panic!("expected Doubling Chant cast-method choice, got {other:?}"),
    };
    dispatch_select_options(&mut wasm, &[free_cast_index]);
    dispatch_pass_priority(&mut wasm);

    let may_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Boolean(ctx)) => ctx,
        other => panic!("expected Doubling Chant may prompt on resolution, got {other:?}"),
    };
    let may_text = may_ctx.description.to_ascii_lowercase();
    assert!(
        may_text
            .contains("search your library for a creature card with the same name as ornithopter"),
        "expected a user-facing Doubling Chant may prompt, got {:?}",
        may_ctx.description
    );
    assert!(
        !may_text.contains("tags it as 'searched'"),
        "Doubling Chant may prompt should not expose internal search tags: {:?}",
        may_ctx.description
    );

    dispatch_select_options(&mut wasm, &[1]);

    let select_ctx = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::SelectObjects(ctx)) => ctx,
        other => {
            panic!("expected Doubling Chant library choice after accepting the may, got {other:?}")
        }
    };
    let select_text = select_ctx.description.to_ascii_lowercase();
    assert!(
        select_text
            .contains("search your library for a creature card with the same name as ornithopter"),
        "expected a user-facing Doubling Chant search prompt, got {:?}",
        select_ctx.description
    );
    assert_eq!(
        select_ctx.candidates.len(),
        2,
        "the search prompt should expose the two matching library Ornithopters"
    );
    assert!(
        select_ctx
            .candidates
            .iter()
            .all(|candidate| candidate.name == "Ornithopter"),
        "Doubling Chant search candidates should be the matching library cards"
    );
    let candidate_ids: Vec<u64> = select_ctx
        .candidates
        .iter()
        .map(|candidate| candidate.id.0)
        .collect();
    assert!(
        !candidate_ids.contains(&battlefield_ornithopter.0),
        "the battlefield Ornithopter should not appear in the library search candidates"
    );
    assert!(
        candidate_ids.contains(&library_ornithopter_a)
            && candidate_ids.contains(&library_ornithopter_b),
        "the search candidates should point at the library Ornithopter objects"
    );
}
