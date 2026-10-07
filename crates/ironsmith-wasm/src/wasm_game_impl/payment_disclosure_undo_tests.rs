// UNVALIDATED: source-only regressions, included in live_action_rollback_tests.
fn payment_disclosure_prepare_priority(wasm: &mut WasmGame) {
    let alice = PlayerId(0);
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(
        PriorityContext::new(
            &wasm.game,
            alice,
            compute_legal_actions(&wasm.game, alice).unwrap(),
        )
        .unwrap(),
    ));
}

// Use real, owner-known identities with peer hidden-zone bookkeeping. Merely
// putting an untracked card into Hand does not exercise the public-opening
// selection policy used in a peer game.
fn payment_disclosure_track_hand(wasm: &mut WasmGame, card: ObjectId, slot: u16) {
    wasm.game.set_hidden_card_info(
        card,
        ironsmith::game_state::HiddenCardInfo {
                incarnation: Some(0),
            owner: PlayerId(0),
            zone: Zone::Hand,
            slot,
            commitment: format!("payment-disclosure-slot-{slot}"),
            origin_slot: None,
            origin_commitment: None,
            public_slot: None,
            public_commitment: None,
        },
    );
}

#[test]
fn payment_disclosure_exact_discard_cost_cards_disable_completed_action_undo() {
    let _guard = crate::test_id_counter_guard();
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/payment_disclosure_partials.json.fixture"
    ))
    .unwrap();
    for card in cards
        .iter()
        .filter(|card| card["previously_proposed_complete"] == true)
    {
        for candidate_count in [1, 2] {
            let name = card["name"].as_str().unwrap();
            let definition = ironsmith_registry_test::compile_to_runtime_definition(
                name,
                card["text"].as_str().unwrap(),
                false,
            )
            .unwrap();
            let (mut wasm, _) = manual_payment_fixture();
            let alice = PlayerId(0);
            let source =
                wasm.game
                    .create_object_from_definition(&definition, alice, Zone::Battlefield);
            wasm.game.remove_summoning_sickness(source);
            let payment_card =
                ironsmith::card::CardBuilder::new(CardId::new(), "Public discard payment")
                    .card_types(vec![CardType::Artifact])
                    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
                    .build();
            let payment = wasm
                .game
                .create_object_from_card(&payment_card, alice, Zone::Hand);
            payment_disclosure_track_hand(&mut wasm, payment, 0);
            if candidate_count == 2 {
                let alternative =
                    wasm.game
                        .create_object_from_card(&payment_card, alice, Zone::Hand);
                payment_disclosure_track_hand(&mut wasm, alternative, 1);
            }
            let payment_stable = wasm.game.object(payment).unwrap().stable_id;
            wasm.game
                .player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 3);
            let target = if name == "Kozilek, the Great Distortion" {
                let target_card =
                    ironsmith::card::CardBuilder::new(CardId::new(), "Three-value spell")
                        .card_types(vec![CardType::Instant])
                        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
                        .build();
                let id = wasm
                    .game
                    .create_object_from_card(&target_card, PlayerId(1), Zone::Stack);
                wasm.game.push_to_stack(StackEntry::new(id, PlayerId(1)));
                TargetInput::Object { object: id.0 }
            } else {
                TargetInput::Player { player: 1 }
            };
            let before_stack = wasm.game.stack.len();
            let before_library = wasm.game.player(alice).unwrap().library.clone();
            payment_disclosure_prepare_priority(&mut wasm);
            let index = definition
                .abilities
                .iter()
                .position(|ability| {
                    matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_))
                })
                .unwrap();
            disclosure_priority_matching(&mut wasm, |action| {
                matches!(action,
            LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index)
            });
            let mut selected = false;
            for _ in 0..40 {
                let command = match wasm.pending_decision.as_ref().unwrap() {
                    DecisionContext::Priority(_) => break,
                    DecisionContext::Number(_) => UiCommand::NumberChoice { value: 3 },
                    DecisionContext::Targets(_) => UiCommand::SelectTargets {
                        targets: vec![target.clone()],
                    },
                    DecisionContext::ManaPayment(_) => {
                        disclosure_confirm_mana(&mut wasm);
                        continue;
                    }
                    DecisionContext::SelectOptions(options) => UiCommand::SelectOptions {
                        option_indices: vec![
                            options
                                .options
                                .iter()
                                .find(|option| option.legal)
                                .unwrap()
                                .index,
                        ],
                    },
                    DecisionContext::SelectObjects(objects) => {
                        assert!(
                            objects
                                .candidates
                                .iter()
                                .any(|candidate| candidate.id == payment)
                        );
                        assert_eq!(
                            objects.reveal_policy,
                            SelectionRevealPolicy::Public,
                            "the regression must retain public proof validation"
                        );
                        if !selected {
                            assert!(
                                wasm.is_cancelable(),
                                "a pending selection before disclosure remains cancelable: {name}"
                            );
                        }
                        selected = true;
                        UiCommand::SelectObjects {
                            object_ids: vec![payment.0],
                            object_stable_ids: Vec::new(),
                            object_hidden_refs: Vec::new(),
                        }
                    }
                    other => panic!("unexpected payment decision for {name}: {other:?}"),
                };
                disclosure_command(&mut wasm, command).unwrap();
            }
            assert!(
                selected,
                "{name} must retain its public discard prompt with {candidate_count} candidate(s)"
            );
            assert!(matches!(
                wasm.pending_decision,
                Some(DecisionContext::Priority(_))
            ), "{name} with {candidate_count} payment cards: {:?}", wasm.pending_decision);
            assert_eq!(wasm.game.stack.len(), before_stack + 1);
            assert_eq!(
                wasm.game.player(alice).unwrap().library,
                before_library,
                "the ability has not resolved; the existing library latch cannot explain this result"
            );
            let paid = wasm.game.find_object_by_stable_id(payment_stable).unwrap();
            assert_eq!(wasm.game.object(paid).unwrap().zone, Zone::Graveyard);
            assert!(
                !wasm.is_cancelable(),
                "published cost identity cannot be undone: {name}"
            );
            #[cfg(target_arch = "wasm32")]
            assert!(
                wasm.cancel_decision().is_err(),
                "direct cancelDecision must enforce the same guard"
            );
            assert_eq!(wasm.game.object(paid).unwrap().zone, Zone::Graveyard);
        }
    }
}

#[test]
fn payment_disclosure_public_hand_reveal_locks_undo_without_a_zone_change() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let alice = PlayerId(0);
    let definition = ironsmith_registry_test::compile_to_runtime_definition(
        "Hand reveal Undo control",
        "Type: Artifact\n{T}, Reveal a card from your hand: Draw a card.",
        false,
    )
    .unwrap();
    let source = wasm
        .game
        .create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand = wasm.game.create_object_from_card(
        &ironsmith::card::CardBuilder::new(CardId::new(), "Revealed card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Hand,
    );
    let alternative = wasm.game.create_object_from_card(
        &ironsmith::card::CardBuilder::new(CardId::new(), "Unrevealed alternative")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Hand,
    );
    payment_disclosure_track_hand(&mut wasm, hand, 0);
    payment_disclosure_track_hand(&mut wasm, alternative, 1);
    payment_disclosure_prepare_priority(&mut wasm);
    disclosure_priority_matching(
        &mut wasm,
        |action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source),
    );
    assert!(wasm.is_cancelable());
    disclosure_command(
        &mut wasm,
        UiCommand::SelectObjects {
            object_ids: vec![hand.0],
            object_stable_ids: Vec::new(),
            object_hidden_refs: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(wasm.game.object(hand).unwrap().zone, Zone::Hand);
    assert_eq!(wasm.game.stack.len(), 1);
    assert!(!wasm.is_cancelable());
}

#[test]
fn payment_disclosure_private_views_and_mana_only_actions_do_not_set_the_guard() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, mountain) = manual_payment_fixture();
    let alice = PlayerId(0);
    let hand = wasm.game.create_object_from_card(
        &ironsmith::card::CardBuilder::new(CardId::new(), "Private hand card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Hand,
    );
    payment_disclosure_prepare_priority(&mut wasm);
    let before = wasm.capture_replay_checkpoint();
    let view =
        ViewCardsContext::new(alice, alice, None, Zone::Hand, "Owner's hand").with_public(false);
    merge_audit_viewed_cards(
        &wasm.game,
        &mut wasm.active_audit_viewed_cards,
        alice,
        &[hand],
        &view,
    );
    assert!(!wasm.has_irreversible_hand_disclosure_since(&before));
    disclosure_priority_matching(&mut wasm, |action| {
        matches!(action,
        LegalAction::ActivateManaAbility { source, .. } if *source == mountain)
    });
    assert!(wasm.game.is_tapped(mountain));
    assert!(
        wasm.is_cancelable(),
        "ordinary mana-only Undo remains available"
    );
    wasm.cancel_decision().unwrap();
    assert!(!wasm.game.is_tapped(mountain));
    assert_eq!(wasm.game.object(hand).unwrap().zone, Zone::Hand);
}

#[test]
fn payment_disclosure_audit_views_are_included_and_speculative_restore_does_not_latch() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let alice = PlayerId(0);
    let hand = wasm.game.create_object_from_card(
        &ironsmith::card::CardBuilder::new(CardId::new(), "Public hand view")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Hand,
    );
    payment_disclosure_prepare_priority(&mut wasm);
    let before = wasm.capture_replay_checkpoint();
    let old_views = wasm.active_audit_viewed_cards.clone();
    let view = ViewCardsContext::new(PlayerId(1), alice, None, Zone::Hand, "Reveal payment")
        .with_public(true);
    merge_audit_viewed_cards(
        &wasm.game,
        &mut wasm.active_audit_viewed_cards,
        PlayerId(1),
        &[hand],
        &view,
    );
    assert!(
        wasm.has_irreversible_hand_disclosure_since(&before),
        "a reveal need not move any card or drain a trigger event yet"
    );
    let disclosed = wasm.capture_replay_checkpoint();
    assert!(
        !wasm.has_irreversible_hand_disclosure_since(&disclosed),
        "an existing disclosure is part of a later safe boundary"
    );
    // Match preview_crypto_requirements: restore the game checkpoint AND the
    // saved audit buffer, not just GameState. No persistent speculative latch.
    wasm.restore_replay_checkpoint(&before);
    wasm.active_audit_viewed_cards = old_views;
    assert!(!wasm.has_irreversible_hand_disclosure_since(&before));
}

#[test]
fn payment_disclosure_knollspine_nested_discard_replacement_cannot_undo_opened_payment() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let alice = PlayerId(0);
    let source_definition = ironsmith_registry_test::compile_to_runtime_definition(
        "Knollspine Invocation",
        "Mana cost: {1}{R}{R}\nType: Enchantment\n{X}, Discard a card with mana value X: This enchantment deals X damage to any target.",
        false,
    ).unwrap();
    let source =
        wasm.game
            .create_object_from_definition(&source_definition, alice, Zone::Battlefield);
    let rip = crate::compile_test_card_definitions("Rest in Peace")
        .unwrap()
        .remove(0);
    let temper = crate::compile_test_card_definitions("Fiery Temper")
        .unwrap()
        .remove(0);
    wasm.game
        .create_object_from_definition(&rip, alice, Zone::Battlefield);
    let payment = wasm
        .game
        .create_object_from_definition(&temper, alice, Zone::Hand);
    let alternative = wasm
        .game
        .create_object_from_definition(&temper, alice, Zone::Hand);
    payment_disclosure_track_hand(&mut wasm, payment, 0);
    payment_disclosure_track_hand(&mut wasm, alternative, 1);
    let stable = wasm.game.object(payment).unwrap().stable_id;
    wasm.game
        .player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    let before_library = wasm.game.player(alice).unwrap().library.clone();
    payment_disclosure_prepare_priority(&mut wasm);
    dispatch_priority_action_matching(
        &mut wasm,
        |action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source),
    );
    let mut selected = false;
    let mut replacement_seen = false;
    for _ in 0..30 {
        let command = match wasm.pending_decision.as_ref().unwrap() {
            DecisionContext::Priority(_) => break,
            DecisionContext::Number(_) => UiCommand::NumberChoice { value: 3 },
            DecisionContext::Targets(_) => UiCommand::SelectTargets {
                targets: vec![TargetInput::Player { player: 1 }],
            },
            DecisionContext::ManaPayment(_) => {
                assert!(
                    !selected,
                    "printed mana is prepared before the card is disclosed"
                );
                confirm_pending_mana_payment(&mut wasm);
                continue;
            }
            DecisionContext::SelectObjects(objects) => {
                assert_eq!(objects.reveal_policy, SelectionRevealPolicy::Public);
                assert!(objects.candidates.iter().any(|card| card.id == payment));
                if !selected {
                    assert!(wasm.is_cancelable());
                }
                selected = true;
                UiCommand::SelectObjects {
                    object_ids: vec![payment.0],
                    object_stable_ids: Vec::new(),
                    object_hidden_refs: Vec::new(),
                }
            }
            DecisionContext::SelectOptions(options) => {
                let graveyard_replacement = options.options.iter().find(|option| {
                    option.description.contains("Rest in Peace")
                        || option.description.contains("Graveyard replacement")
                });
                let index = if selected {
                    assert!(
                        options
                            .options
                            .iter()
                            .any(|option| option.description.contains("Madness")),
                        "a real discard replacement interrupts payment"
                    );
                    let option =
                        graveyard_replacement.expect("Rest in Peace competes with Madness");
                    let index = option.index;
                    replacement_seen = true;
                    assert_eq!(
                        wasm.game.object(payment).unwrap().zone,
                        Zone::Hand,
                        "the core staged cost has not committed its zone change"
                    );
                    assert!(wasm.game.stack.is_empty(), "the ability has not finalized");
                    assert!(
                        !wasm.is_cancelable(),
                        "the public selection remains known during replacement choice"
                    );
                    #[cfg(target_arch = "wasm32")]
                    assert!(wasm.cancel_decision().is_err());
                    index
                } else {
                    options
                        .options
                        .iter()
                        .find(|option| option.legal)
                        .unwrap()
                        .index
                };
                UiCommand::SelectOptions {
                    option_indices: vec![index],
                }
            }
            other => panic!("unexpected Knollspine replacement decision: {other:?}"),
        };
        dispatch_manual_payment_command(&mut wasm, command);
    }
    assert!(replacement_seen);
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::Priority(_))
    ));
    assert_eq!(wasm.game.stack.len(), 1);
    let paid = wasm.game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(wasm.game.object(paid).unwrap().zone, Zone::Exile);
    assert_eq!(wasm.game.player(alice).unwrap().library, before_library);
    assert!(!wasm.is_cancelable());
}
