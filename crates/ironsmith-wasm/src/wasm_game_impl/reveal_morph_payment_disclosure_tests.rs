// Authored only: native face-down casting followed by the existing public
// dispatcher and disclosure journal. No alternate reveal protocol is used.
fn reveal_morph_definition(name: &str) -> CardDefinition {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/card-failure-campaign/reveal-in-hand-morph-costs.json")).unwrap();
    let card = fixture["cards"].as_array().unwrap().iter().find(|card| card["name"] == name).unwrap();
    ironsmith_registry_test::compile_to_runtime_definition(name, card["text"].as_str().unwrap(), false).unwrap()
}

fn reveal_morph_cast(wasm: &mut WasmGame, definition: &CardDefinition) -> ObjectId {
    reveal_morph_cast_for(wasm, definition, PlayerId(0), false)
}

fn reveal_morph_cast_for(wasm: &mut WasmGame, definition: &CardDefinition, alice: PlayerId, tracked: bool) -> ObjectId {
    wasm.game.turn.active_player = alice;
    wasm.game.turn.priority_player = Some(alice);
    wasm.game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
    let hand = wasm.game.create_object_from_definition(definition, alice, Zone::Hand);
    if tracked {
        wasm.game.set_hidden_card_info(hand, ironsmith::game_state::HiddenCardInfo {
                incarnation: Some(0),
            owner: alice, zone: Zone::Hand, slot: 20,
            commitment: "tracked-morph-source".into(), origin_slot: None, origin_commitment: None,
            public_slot: None, public_commitment: None,
        });
    }
    let stable = wasm.game.object(hand).unwrap().stable_id;
    let action = compute_legal_actions(&wasm.game, alice).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::FaceDown, .. }
            if *spell_id == hand)).unwrap();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(&mut wasm.game, &mut wasm.trigger_queue,
        &mut wasm.priority_state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..20 {
        if wasm.priority_state.pending_cast.is_none() && !wasm.game.stack.is_empty() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut wasm.game,
            &mut wasm.trigger_queue, &mut wasm.priority_state, &ctx, &mut dm).unwrap();
    }
    let spell = wasm.game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(wasm.game.object(spell).unwrap().zone, Zone::Stack);
    assert!(wasm.game.is_face_down(spell));
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool.total(), 0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut dm).unwrap();
    let source = wasm.game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(wasm.game.object(source).unwrap().zone, Zone::Battlefield);
    assert!(wasm.game.is_face_down(source));
    source
}

fn reveal_morph_hand(wasm: &mut WasmGame, colors: ironsmith::color::ColorSet) -> [ObjectId; 2] {
    let mut ids = Vec::new();
    for (slot, name) in ["Chosen reveal payment", "Unchosen reveal alternative"].into_iter().enumerate() {
        let card = ironsmith::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant]).color_indicator(colors).build();
        let id = wasm.game.create_object_from_card(&card, PlayerId(0), Zone::Hand);
        payment_disclosure_track_hand(wasm, id, slot as u16);
        ids.push(id);
    }
    [ids[0], ids[1]]
}

fn reveal_morph_begin(wasm: &mut WasmGame, source: ObjectId) {
    payment_disclosure_prepare_priority(wasm);
    disclosure_priority_matching(wasm, |action| matches!(action,
        LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == source));
    let Some(DecisionContext::SelectObjects(objects)) = wasm.pending_decision.as_ref() else {
        panic!("morph must wait for its reveal payment: {:?}", wasm.pending_decision);
    };
    assert_eq!(objects.player, PlayerId(0));
    assert_eq!(objects.min, 1); assert_eq!(objects.max, Some(1));
    assert_eq!(objects.reveal_policy, SelectionRevealPolicy::Public);
    assert_eq!(wasm.payment_transaction_subject(), Some((source, PlayerId(0))));
    assert!(wasm.game.is_face_down(source));
}

#[test]
fn reveal_morph_full_card_public_payment_cancels_before_disclosure_and_locks_after_it() {
    let _guard = crate::test_id_counter_guard();
    for (name, color) in [
        ("Dragon's Eye Savants", ironsmith::color::ColorSet::BLUE),
        ("Horde Ambusher", ironsmith::color::ColorSet::RED),
        ("Ruthless Ripper", ironsmith::color::ColorSet::BLACK),
        ("Temur Charger", ironsmith::color::ColorSet::GREEN),
        ("Watcher of the Roost", ironsmith::color::ColorSet::WHITE),
    ] {
        let (mut wasm, _) = manual_payment_fixture();
        let source = reveal_morph_cast(&mut wasm, &reveal_morph_definition(name));
        let [chosen, other] = reveal_morph_hand(&mut wasm, color);
        let stable = wasm.game.object(chosen).unwrap().stable_id;
        reveal_morph_begin(&mut wasm, source);
        assert!(wasm.is_cancelable());
        wasm.cancel_decision().unwrap();
        assert!(wasm.game.is_face_down(source));
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, vec![chosen, other]);
        assert!(wasm.payment_disclosure.is_none());
        reveal_morph_begin(&mut wasm, source);
        for invalid in [vec![], vec![chosen.0, chosen.0], vec![chosen.0, other.0], vec![source.0]] {
            assert!(disclosure_command(&mut wasm, UiCommand::SelectObjects {
                object_ids: invalid, object_stable_ids: Vec::new(), object_hidden_refs: Vec::new(),
            }).is_err());
            assert!(wasm.payment_disclosure.is_none()); assert!(wasm.is_cancelable());
            assert!(wasm.game.is_face_down(source));
        }
        disclosure_command(&mut wasm, disclosure_selection(chosen)).unwrap();
        // Target selection belongs to the turned-face-up trigger. The paid
        // special action has already disclosed exactly one hand identity.
        assert!(!wasm.game.is_face_down(source));
        assert!(!wasm.is_cancelable());
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, vec![chosen, other]);
        assert_eq!(wasm.game.object(chosen).unwrap().stable_id, stable);
        assert!(wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), chosen)));
        assert!(!wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), other)));
    }
}

#[test]
fn reveal_morph_failed_public_attempt_pins_same_answer_and_replays_one_native_payment() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let mut definition = reveal_morph_definition("Watcher of the Roost");
    let failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    for ability in &mut definition.abilities {
        let ironsmith::ability::AbilityKind::Static(static_ability) = &mut ability.kind else { continue; };
        if let Some(cost) = static_ability.turn_face_up_cost() {
            let mut components = cost.costs().to_vec();
            components.push(ironsmith::costs::Cost::try_effect(ironsmith::effect::Effect::new(
                TransientPaymentFault(failing.clone()))).unwrap());
            *static_ability = ironsmith::static_abilities::StaticAbility::morph(
                ironsmith::cost::TotalCost::from_costs(components));
        }
    }
    let source = reveal_morph_cast(&mut wasm, &definition);
    let [chosen, other] = reveal_morph_hand(&mut wasm, ironsmith::color::ColorSet::WHITE);
    reveal_morph_begin(&mut wasm, source);
    let before_pool = wasm.game.player(PlayerId(0)).unwrap().mana_pool.clone();
    // The transport has committed/opened the chosen identity before replay.
    // A subsequent native payment admission/continuation failure cannot
    // restore cancellation or permit substitution of a different hand card.
    failing.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(disclosure_command(&mut wasm, disclosure_selection(chosen)).is_err());
    assert!(wasm.game.is_face_down(source));
    assert!(!wasm.is_cancelable());
    let committed = wasm.payment_disclosure.as_ref().unwrap();
    assert_eq!(committed.source, source); assert_eq!(committed.payer, PlayerId(0));
    assert_eq!(committed.disclosed_objects.iter().copied().collect::<Vec<_>>(), vec![chosen]);
    assert!(wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::ManaPayment(
        ironsmith::mana_payment::ManaPaymentResponse::Cancel)).is_err());
    assert!(disclosure_command(&mut wasm, disclosure_selection(other)).is_err());
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, vec![chosen, other]);
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool, before_pool);
    let view = wasm.payment_disclosure_view().unwrap();
    assert_eq!(view.cards, vec![chosen]); assert!(view.public);
    failing.store(false, std::sync::atomic::Ordering::SeqCst);
    disclosure_command(&mut wasm, disclosure_selection(chosen)).unwrap();
    assert!(!wasm.game.is_face_down(source));
    assert!(!wasm.is_cancelable());
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, vec![chosen, other]);
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool, before_pool);
    assert_eq!(wasm.game.stack.len(), 1, "exactly one turned-face-up trigger");
    assert!(wasm.payment_disclosure.is_none());
    assert!(wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), chosen)));
    assert!(!wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), other)));
}

#[test]
fn reveal_morph_source_opening_and_hand_payment_share_the_existing_attempt_owner() {
    let _guard = crate::test_id_counter_guard();
    for (name, color) in [
        ("Dragon's Eye Savants", ironsmith::color::ColorSet::BLUE),
        ("Horde Ambusher", ironsmith::color::ColorSet::RED),
        ("Ruthless Ripper", ironsmith::color::ColorSet::BLACK),
        ("Temur Charger", ironsmith::color::ColorSet::GREEN),
        ("Watcher of the Roost", ironsmith::color::ColorSet::WHITE),
    ] {
        let (mut wasm, _) = manual_payment_fixture();
        // Bob owned/cast this exact tracked source; Alice currently controls
        // it and is the payer. Existing command opening metadata uses the
        // source identity while the payment choice still belongs to Alice.
        let source = reveal_morph_cast_for(&mut wasm, &reveal_morph_definition(name), PlayerId(1), true);
        wasm.game.set_current_controller(source, PlayerId(0));
        wasm.game.turn.priority_player = Some(PlayerId(0));
        let [chosen, other] = reveal_morph_hand(&mut wasm, color);
        payment_disclosure_prepare_priority(&mut wasm);
        let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.clone() else { panic!("priority") };
        let action = priority.actions.iter().find(|action| matches!(action,
            LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == source)).unwrap().clone();
        let answer = ReplayDecisionAnswer::Priority(action);
        let context = DecisionContext::Priority(priority);
        // This is the read-only classifier called before existing signed
        // transport opening construction. It must include the face-down
        // source, before the hand-card payment question even exists.
        let saved = wasm.payment_disclosure.clone();
        wasm.commit_payment_command_disclosure(&context, &answer).unwrap();
        let classified = wasm.payment_disclosure.as_ref().unwrap();
        assert_eq!(classified.source, source); assert_eq!(classified.payer, PlayerId(0));
        assert_eq!(classified.disclosed_objects.iter().copied().collect::<Vec<_>>(), vec![source]);
        wasm.payment_disclosure = saved;
        reveal_morph_begin(&mut wasm, source);
        assert_eq!(wasm.game.object(source).unwrap().owner, PlayerId(1));
        assert!(!wasm.is_cancelable(), "the source opening is already public before selecting the hand payment");
        assert!(wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::ManaPayment(
            ironsmith::mana_payment::ManaPaymentResponse::Cancel)).is_err());
        assert_eq!(wasm.payment_disclosure.as_ref().unwrap().disclosed_objects.iter().copied().collect::<Vec<_>>(), vec![source]);
        let source_view = wasm.payment_disclosure_source_view().unwrap();
        assert_eq!(source_view.subject, PlayerId(1)); assert_eq!(source_view.cards, vec![source]);
        assert!(source_view.public);
        wasm.perspective = PlayerId(1);
        let visible: serde_json::Value = serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
        assert!(visible["players"][1]["persistent_look_cards"].as_array().unwrap().iter()
            .any(|card| card["name"] == name));
        assert!(wasm.game.is_face_down(source));
        assert_eq!(wasm.game.calculated_power(source), Some(2));
        wasm.perspective = PlayerId(0);
        disclosure_command(&mut wasm, disclosure_selection(chosen)).unwrap();
        assert!(!wasm.game.is_face_down(source));
        assert!(!wasm.is_cancelable());
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, vec![chosen, other]);
        assert!(wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), chosen)));
        assert!(!wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), other)));
        if let Some(held) = wasm.payment_disclosure.as_ref() {
            assert!(held.disclosed_objects.contains(&source));
            assert!(held.disclosed_objects.contains(&chosen));
            assert_eq!(held.payer, PlayerId(0));
        } else {
            assert!(wasm.priority_epoch_undo_locked_by_disclosure);
        }
    }
}

#[test]
fn reveal_morph_failed_source_opening_retries_the_same_native_action_before_hand_payment() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let mut definition = reveal_morph_definition("Watcher of the Roost");
    let failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    for ability in &mut definition.abilities {
        let ironsmith::ability::AbilityKind::Static(static_ability) = &mut ability.kind else { continue; };
        if let Some(cost) = static_ability.turn_face_up_cost() {
            let mut components = cost.costs().to_vec();
            components.push(ironsmith::costs::Cost::try_effect(ironsmith::effect::Effect::new(
                TransientPaymentFault(failing.clone()))).unwrap());
            *static_ability = ironsmith::static_abilities::StaticAbility::morph(
                ironsmith::cost::TotalCost::from_costs(components));
        }
    }
    let source = reveal_morph_cast_for(&mut wasm, &definition, PlayerId(0), true);
    let other_source = reveal_morph_cast_for(&mut wasm, &reveal_morph_definition("Watcher of the Roost"), PlayerId(0), false);
    let [chosen, other] = reveal_morph_hand(&mut wasm, ironsmith::color::ColorSet::WHITE);
    payment_disclosure_prepare_priority(&mut wasm);
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else { panic!("priority") };
    let command_for = |id| UiCommand::PriorityAction { action_index: Some(priority.actions.iter().position(|action|
        matches!(action, LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == id)).unwrap()), action_ref: None };
    let command = command_for(source);
    let substitute = command_for(other_source);
    failing.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(disclosure_command(&mut wasm, command.clone()).is_err());
    assert!(wasm.game.is_face_down(source));
    assert!(!wasm.is_cancelable());
    let committed = wasm.payment_disclosure.as_ref().unwrap();
    assert_eq!(committed.disclosed_objects.iter().copied().collect::<Vec<_>>(), vec![source]);
    assert!(committed.required_retry.is_some());
    assert!(disclosure_command(&mut wasm, substitute).is_err());
    assert!(!wasm.public_hand_disclosure_identities().contains(&(PlayerId(0), chosen)));
    failing.store(false, std::sync::atomic::Ordering::SeqCst);
    disclosure_command(&mut wasm, command).unwrap();
    assert!(matches!(wasm.pending_decision, Some(DecisionContext::SelectObjects(_))));
    assert!(!wasm.is_cancelable());
    disclosure_command(&mut wasm, disclosure_selection(chosen)).unwrap();
    assert!(!wasm.game.is_face_down(source));
    assert!(wasm.game.is_face_down(other_source));
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, vec![chosen, other]);
    assert_eq!(wasm.game.stack.len(), 1);
    assert!(!wasm.is_cancelable());
}
