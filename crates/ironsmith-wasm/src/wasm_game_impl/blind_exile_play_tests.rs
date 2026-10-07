// Source-authored integration witnesses. Intentionally unrun in this campaign.
fn blind_exile_fixture(kind: CardType, price: u32, alternative: bool) -> (WasmGame, ObjectId, ObjectId) {
    use ironsmith::grant_registry::{GrantSource, PlayFromConstraints};
    let (mut wasm, _) = manual_payment_fixture();
    let owner = PlayerId(0); let actor = PlayerId(1);
    wasm.perspective = actor;
    wasm.game.turn.active_player = actor;
    wasm.game.turn.priority_player = Some(actor);
    let source = wasm.game.create_object_from_definition(&CardDefinitionBuilder::new(CardId::new(), "Public permission")
        .card_types(vec![CardType::Enchantment]).build(), actor, Zone::Battlefield);
    let mut definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Unseen exile face")
        .card_types(vec![kind]).mana_cost(ManaCost::new().add_generic(price));
    if alternative { definition = definition.alternative_cast(ironsmith::alternative_cast::AlternativeCastingMethod::alternative_cost(
        "Hidden alternative", Some(ManaCost::new().add_generic(3)), Vec::new())); }
    let card = wasm.game.create_object_from_definition(&definition.build(), owner, Zone::Exile);
    wasm.game.set_face_down(card); wasm.game.grant_face_down_exile_view(card, owner);
    wasm.game.set_hidden_card_info(card, ironsmith::game_state::HiddenCardInfo {
                incarnation: Some(0),
        owner, zone: Zone::Exile, slot: 7, commitment: "opaque-exile-7".into(), origin_slot: None,
        origin_commitment: None, public_slot: None, public_commitment: None,
    });
    wasm.game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, actor,
        PlayFromConstraints::default(), GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    blind_exile_priority(&mut wasm);
    (wasm, source, card)
}
fn blind_exile_priority(wasm: &mut WasmGame) {
    let actor = wasm.game.turn.priority_player.unwrap();
    wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
    wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(&wasm.game, actor,
        compute_legal_actions(&wasm.game, actor).unwrap()).unwrap()));
}
fn blind_exile_command(wasm: &WasmGame, card: ObjectId) -> UiCommand {
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else { panic!("priority"); };
    let action = priority.actions.iter().find(|action| matches!(action,
        LegalAction::OpenExiledCardForPlay { card_id, .. } if *card_id == card)).unwrap();
    UiCommand::PriorityAction { action_index: None, action_ref: Some(priority_action_ref(action)) }
}
fn blind_exile_choose(wasm: &mut WasmGame, index: usize) {
    disclosure_command(wasm, UiCommand::SelectOptions { option_indices: vec![index] }).unwrap();
}

#[test]
fn blind_exile_public_menu_and_requirements_are_face_independent_before_opening() {
    let _ids = crate::test_id_counter_guard();
    let mut expected = None;
    for kind in [CardType::Land, CardType::Sorcery, CardType::Creature] {
        for price in [0, 100] { for alternative in [false, true] { for mana in [0, 200] {
            let (mut wasm, _, card) = blind_exile_fixture(kind, price, alternative);
            wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = mana;
            wasm.game.player_mut(PlayerId(1)).unwrap().lands_played_this_turn = 1;
            blind_exile_priority(&mut wasm);
            let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else { panic!("priority"); };
            let actions = priority.actions.iter().filter(|action| ironsmith::decision::legal_action_source(action) == Some(card)).collect::<Vec<_>>();
            assert_eq!(actions.len(), 2);
            let opening = actions.iter().find(|action| matches!(action, LegalAction::OpenExiledCardForPlay { .. })).unwrap();
            let mut view = serde_json::to_value(build_action_view(&wasm.game, PlayerId(1), None, 0, opening, None)).unwrap();
            assert_eq!(view["label"], "Play exiled card");
            assert_eq!(view["kind"], "open_exiled_card_for_play");
            assert!(view["object_id"].is_null()); assert!(view["mana_payment_available"].is_null());
            assert!(view["to_zone"].is_null()); assert_eq!(view["drag_requires_targets"], false);
            view["action_ref"]["card_id"] = serde_json::json!(1);
            view["action_ref"]["permission"]["source"] = serde_json::json!(2);
            if let Some(expected) = &expected { assert_eq!(&view, expected); } else { expected = Some(view); }
            let command = blind_exile_command(&wasm, card);
            let before = wasm.game.clone();
            let requirements = wasm.blind_exile_opening_requirements(&command).unwrap().unwrap();
            assert_eq!(requirements.len(), 1); let requirement = &requirements[0];
            assert_eq!(requirement.requirement_type, "public_open"); assert_eq!(requirement.timing.as_deref(), Some("pre"));
            assert_eq!(requirement.object_id, Some(card.0)); assert!(requirement.card.is_none());
            assert_eq!(requirement.owner, 0); assert_eq!(requirement.visibility.as_deref(), Some("public"));
            assert!(wasm.payment_disclosure.is_none()); assert!(wasm.game.is_face_down(card));
            assert!(!wasm.game.can_player_look_at_face_down_exiled_card(card, PlayerId(1)));
            assert_eq!(wasm.game.player(PlayerId(1)).unwrap().mana_pool, before.player(PlayerId(1)).unwrap().mana_pool);
        } } }
    }
}

#[test]
fn blind_exile_exact_ref_rejects_wrong_grant_stale_card_and_wrong_priority_holder() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, source, card) = blind_exile_fixture(CardType::Sorcery, 1, false);
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.clone() else { panic!("priority"); };
    let UiCommand::PriorityAction { action_ref: Some(reference), .. } = blind_exile_command(&wasm, card) else { unreachable!(); };
    let mut deferred = priority.clone(); deferred.analysis_complete = false; deferred.actions = ironsmith::decisions::context::PreparedPriorityActions::new(&wasm.game, Vec::new()).unwrap();
    assert!(resolve_priority_action(&wasm.game, &deferred, None, Some(&reference)).unwrap().is_some());
    for (bad_card, bad_source, bad_index) in [(source.0, source.0, 0), (card.0, card.0, 0), (card.0, source.0, 1)] {
        let forged = PriorityActionRef::OpenExiledCardForPlay { card_id: bad_card, incarnation: Some(0),
            permission: GrantSelectionRef { source: bad_source, index: bad_index } };
        assert!(resolve_priority_action(&wasm.game, &priority, Some(0), Some(&forged)).unwrap().is_none());
    }
    wasm.game.turn.priority_player = Some(PlayerId(0));
    assert!(resolve_priority_action(&wasm.game, &priority, None, Some(&reference)).unwrap().is_none());
    wasm.game.turn.priority_player = Some(PlayerId(1));
    wasm.game.effect_store.grant_registry.remove_grants_from_source(source);
    assert!(resolve_priority_action(&wasm.game, &priority, None, Some(&reference)).unwrap().is_none());
    assert!(wasm.game.is_face_down(card)); assert!(wasm.payment_disclosure.is_none());
}

#[test]
fn blind_exile_placeholder_has_the_same_opening_authority_but_requires_materialization() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, source, _) = blind_exile_fixture(CardType::Sorcery, 1, false);
    let hidden = wasm.game.create_hidden_card_placeholder(PlayerId(0), Zone::Exile, 8, "opaque-exile-8".into());
    wasm.game.set_face_down(hidden);
    wasm.game.effect_store.grant_registry.grant_play_from_to_card(hidden, Zone::Exile, PlayerId(1),
        Default::default(), ironsmith::grant_registry::GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    blind_exile_priority(&mut wasm);
    let command = blind_exile_command(&wasm, hidden);
    assert_eq!(wasm.blind_exile_opening_requirements(&command).unwrap().unwrap()[0].object_id, Some(hidden.0));
    // The native action rejects incomplete evidence, retaining the same command.
    assert!(disclosure_command(&mut wasm, command.clone()).is_err());
    assert!(wasm.game.is_face_down(hidden)); assert!(wasm.priority_state.opened_exile_play.is_none());
    assert!(!wasm.is_cancelable()); assert!(wasm.payment_disclosure.as_ref().unwrap().required_retry.is_some());
    let definition = CardDefinitionBuilder::new(CardId::new(), "Hydrated unavailable spell")
        .card_types(vec![CardType::Sorcery]).mana_cost(ManaCost::new().add_generic(100)).build();
    wasm.game.reveal_hidden_card_with_definition(hidden, &definition).unwrap();
    // Hydration does not grant private inspection or replace the opaque menu.
    assert!(!wasm.game.can_player_look_at_face_down_exiled_card(hidden, PlayerId(1)));
    disclosure_command(&mut wasm, command).unwrap();
    assert!(wasm.game.is_face_down(hidden)); assert!(wasm.payment_disclosure.is_none());
    assert!([PlayerId(0), PlayerId(1)].into_iter().all(|viewer| wasm.game.can_player_look_at_face_down_exiled_card(hidden, viewer)));
    assert!(!wasm.priority_state.has_pending_action()); assert!(wasm.game.stack.is_empty());
}

#[test]
fn blind_exile_land_and_spell_keep_original_subject_through_options_and_payment() {
    let _ids = crate::test_id_counter_guard();
    for kind in [CardType::Land, CardType::Sorcery] {
        let (mut wasm, _, card) = blind_exile_fixture(kind, 1, false);
        wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = 1;
        let stable = wasm.game.object(card).unwrap().stable_id;
        let command = blind_exile_command(&wasm, card);
        disclosure_command(&mut wasm, command).unwrap();
        assert!(!wasm.game.is_face_down(card)); assert!(matches!(wasm.pending_decision, Some(DecisionContext::SelectOptions(_))));
        assert_eq!(wasm.payment_transaction_subject(), Some((card, PlayerId(1))));
        assert!(!wasm.is_cancelable());
        let audit = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
        assert_eq!(audit["priorityRuntime"]["openedExilePlay"]["cardId"], card.0);
        assert_eq!(audit["priorityRuntime"]["openedExilePlay"]["choicePending"], true);
        blind_exile_choose(&mut wasm, 0);
        for _ in 0..20 {
            if !wasm.priority_state.has_pending_action() { break; }
            assert_eq!(wasm.payment_transaction_subject(), Some((card, PlayerId(1))));
            assert!(wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::ManaPayment(
                ironsmith::mana_payment::ManaPaymentResponse::Cancel)).is_err());
            match wasm.pending_decision.as_ref().unwrap() {
                DecisionContext::ManaPayment(_) => disclosure_confirm_mana(&mut wasm),
                DecisionContext::SelectOptions(options) => { let index = options.options.iter().find(|option| option.legal).unwrap().index; blind_exile_choose(&mut wasm, index); }
                context => panic!("unexpected continuation {context:?}"),
            }
        }
        assert!(!wasm.priority_state.has_pending_action()); assert!(wasm.payment_disclosure.is_none()); assert!(!wasm.is_cancelable());
        let played = wasm.game.find_object_by_stable_id(stable).unwrap(); assert_ne!(played, card);
        assert_eq!(wasm.game.object(played).unwrap().zone, if kind == CardType::Land { Zone::Battlefield } else { Zone::Stack });
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().lands_played_this_turn, u32::from(kind == CardType::Land));
    }
}

#[test]
fn blind_exile_unavailable_play_keeps_disclosure_without_spending_resources_or_permission() {
    let _ids = crate::test_id_counter_guard();
    for kind in [CardType::Land, CardType::Sorcery] {
        let (mut wasm, _, card) = blind_exile_fixture(kind, 100, false);
        wasm.game.player_mut(PlayerId(1)).unwrap().lands_played_this_turn = 1;
        let mana = wasm.game.player(PlayerId(1)).unwrap().mana_pool.clone();
        let command = blind_exile_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
        assert!(wasm.game.is_face_down(card)); assert_eq!(wasm.game.object(card).unwrap().zone, Zone::Exile);
        assert!([PlayerId(0), PlayerId(1)].into_iter().all(|viewer| wasm.game.can_player_look_at_face_down_exiled_card(card, viewer)));
        assert!(wasm.game.current_characteristics(card).unwrap().card_types.is_empty());
        assert!(wasm.payment_disclosure.is_none()); assert!(!wasm.is_cancelable());
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().lands_played_this_turn, 1);
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().mana_pool, mana);
        assert_eq!(wasm.game.turn.priority_player, Some(PlayerId(1)));
        assert!(wasm.game.stack.is_empty()); assert!(!wasm.priority_state.has_pending_action());
    }
}

#[test]
fn blind_exile_signed_recovery_and_native_savepoints_preserve_exact_public_source() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 1, false);
    let command = blind_exile_command(&wasm, card);
    let prefix = RuntimeSavepoint::capture(&wasm);
    // Equivalent to verified material being reapplied after an accepted-prefix
    // restore. Physical face-down state stays canonical; known identity is public.
    wasm.retain_payment_disclosure_command(command.clone()).unwrap();
    assert!(wasm.game.is_face_down(card)); assert_eq!(wasm.payment_disclosure_source_view().unwrap().zone, Zone::Exile);
    assert_eq!(wasm.payment_disclosure.as_ref().unwrap().payer, PlayerId(1));
    let view: serde_json::Value = serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
    assert!(view["players"][0]["persistent_look_cards"].as_array().unwrap().iter().any(|visible| visible["name"] == "Unseen exile face"));
    assert!(wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::Priority(LegalAction::PassPriority)).is_err());
    let held = RuntimeSavepoint::capture(&wasm); prefix.restore(&mut wasm);
    assert!(wasm.payment_disclosure.is_none()); assert!(wasm.game.is_face_down(card));
    held.restore(&mut wasm); assert!(!wasm.is_cancelable());
    wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = 1;
    disclosure_command(&mut wasm, command).unwrap();
    let opened = wasm.priority_state.opened_exile_play.clone().unwrap();
    wasm.grand_melee_host_lanes.insert(7, GrandMeleeHostLane { runner: None, runner_awaiting_priority: true,
        trigger_queue: wasm.trigger_queue.clone(), priority_state: wasm.priority_state.clone() });
    wasm.suspended_subgame_hosts.push((None, true, wasm.trigger_queue.clone(), wasm.priority_state.clone(), wasm.grand_melee_host_lanes.clone()));
    let checkpoint = wasm.capture_replay_checkpoint(); let mut branch = RuntimeSavepoint::capture(&wasm);
    wasm.priority_state.opened_exile_play = None; wasm.priority_state.pending_exile_play = None;
    wasm.grand_melee_host_lanes.clear(); wasm.suspended_subgame_hosts.clear();
    branch.exchange(&mut wasm);
    assert_eq!(wasm.priority_state.opened_exile_play.as_ref().unwrap().permission, opened.permission);
    assert_eq!(wasm.grand_melee_host_lanes[&7].priority_state.opened_exile_play.as_ref().unwrap().card_id, card);
    assert_eq!(wasm.suspended_subgame_hosts[0].3.opened_exile_play.as_ref().unwrap().player, PlayerId(1));
    wasm.restore_replay_checkpoint(&checkpoint);
    assert!(!wasm.game.is_face_down(card)); assert!(wasm.priority_state.pending_exile_play.is_some()); assert!(!wasm.is_cancelable());
}

#[test]
fn blind_exile_target_prompt_and_failed_land_replay_keep_the_opening_commitment() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 1, false);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Opened targeted spell")
        .card_types(vec![CardType::Sorcery]).mana_cost(ManaCost::new().add_generic(1))
        .with_spell_effect(vec![ironsmith::Effect::deal_damage(1, ironsmith::target::ChooseSpec::target_player())]).build();
    wasm.game.object_mut(card).unwrap().spell_effect = definition.spell_effect.clone().map(Into::into);
    wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = 1;
    blind_exile_priority(&mut wasm);
    let command = blind_exile_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    blind_exile_choose(&mut wasm, 0);
    assert!(matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))));
    assert_eq!(wasm.payment_transaction_subject(), Some((card, PlayerId(1))));
    assert!(!wasm.is_cancelable());
    disclosure_command(&mut wasm, UiCommand::SelectTargets { targets: vec![TargetInput::Player { player: 0 }] }).unwrap();
    for _ in 0..12 { if !wasm.priority_state.has_pending_action() { break; }
        match wasm.pending_decision.as_ref().unwrap() {
            DecisionContext::ManaPayment(_) => disclosure_confirm_mana(&mut wasm),
            DecisionContext::SelectOptions(options) => { let index = options.options.iter().find(|option| option.legal).unwrap().index; blind_exile_choose(&mut wasm, index); }
            context => panic!("unexpected spell continuation {context:?}"),
        }
    }
    assert!(!wasm.priority_state.has_pending_action()); assert_eq!(wasm.game.stack.len(), 1);
    assert_eq!(wasm.game.stack[0].targets, vec![ironsmith::game_state::Target::Player(PlayerId(0))]);
    assert_eq!(wasm.game.player(PlayerId(1)).unwrap().mana_pool.total(), 0);
    assert!(wasm.payment_disclosure.is_none()); assert!(!wasm.is_cancelable());

    let (mut wasm, source, card) = blind_exile_fixture(CardType::Land, 0, false);
    let command = blind_exile_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    let failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    wasm.game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
        source, PlayerId(1), ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
            ironsmith::target::ObjectFilter::specific(card), Some(Zone::Exile), Some(Zone::Battlefield)),
        ironsmith::replacement::ReplacementAction::Additionally(vec![ironsmith::Effect::gain_life(3),
            ironsmith::Effect::new(TransientPaymentFault(failing.clone()))])));
    let command = UiCommand::SelectOptions { option_indices: vec![0] };
    assert!(disclosure_command(&mut wasm, command.clone()).is_err());
    assert!(!wasm.game.is_face_down(card)); assert_eq!(wasm.game.object(card).unwrap().zone, Zone::Exile);
    assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life, 20);
    assert_eq!(wasm.game.player(PlayerId(1)).unwrap().lands_played_this_turn, 0);
    assert!(wasm.payment_disclosure.as_ref().unwrap().required_retry.is_some());
    assert!(wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::Options(vec![1])).is_err());
    let recovery = RuntimeSavepoint::capture(&wasm); recovery.clone().restore(&mut wasm);
    assert!(wasm.priority_state.pending_exile_play.is_some()); assert!(!wasm.is_cancelable());
    failing.store(false, std::sync::atomic::Ordering::SeqCst);
    disclosure_command(&mut wasm, command).unwrap();
    assert_eq!(wasm.game.player(PlayerId(1)).unwrap().lands_played_this_turn, 1);
    assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life, 23);
    assert!(wasm.payment_disclosure.is_none()); assert!(!wasm.is_cancelable());
}

#[test]
fn blind_exile_nested_land_entry_options_replay_the_chosen_play_instead_of_rechoosing_it() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, source, card) = blind_exile_fixture(CardType::Land, 0, false);
    let command = blind_exile_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    let Some(DecisionContext::SelectOptions(options)) = wasm.pending_decision.as_ref() else { panic!("opened-card choice"); };
    assert!(options.exile_play_choice); assert!(wasm.select_options_uses_live_priority_response(options));
    for amount in [1, 2] {
        wasm.game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            source, PlayerId(1), ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ironsmith::target::ObjectFilter::specific(card), Some(Zone::Exile), Some(Zone::Battlefield)),
            ironsmith::replacement::ReplacementAction::Additionally(vec![ironsmith::Effect::gain_life(amount)])));
    }
    blind_exile_choose(&mut wasm, 0);
    let Some(DecisionContext::SelectOptions(options)) = wasm.pending_decision.as_ref() else { panic!("entry replacement choice"); };
    assert!(!options.exile_play_choice); assert!(!wasm.select_options_uses_live_priority_response(options));
    assert!(matches!(wasm.pending_live_continuation.as_ref().map(|continuation| &continuation.root),
        Some(PendingPriorityContinuation::ApplyResponse(PriorityResponse::ExilePlayChoice(0)))));
    assert!(wasm.priority_state.pending_exile_play.is_some()); assert!(!wasm.is_cancelable());
    for _ in 0..12 {
        if !wasm.priority_state.has_pending_action() { break; }
        let Some(DecisionContext::SelectOptions(options)) = wasm.pending_decision.as_ref() else { panic!("replacement options"); };
        let choice = options.options.iter().find(|option| option.legal).unwrap().index;
        blind_exile_choose(&mut wasm, choice);
    }
    assert!(!wasm.priority_state.has_pending_action()); assert_eq!(wasm.game.player(PlayerId(1)).unwrap().lands_played_this_turn, 1);
    assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life, 23); assert!(!wasm.is_cancelable());
}

fn blind_exile_face_down_command(wasm: &WasmGame, card: ObjectId) -> UiCommand {
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else { panic!("priority"); };
    let action = priority.actions.iter().find(|action| matches!(action,
        LegalAction::CastExiledCardFaceDown { card_id, .. } if *card_id == card)).unwrap();
    UiCommand::PriorityAction { action_index: None, action_ref: Some(priority_action_ref(action)) }
}

#[test]
fn blind_exile_frozen_incarnation_rejects_reentry_and_survives_all_native_savepoint_carriers() {
    use ironsmith::grant_registry::{GrantSource, PlayFromConstraints};
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, source, card) = blind_exile_fixture(CardType::Sorcery, 100, false);
    let old = blind_exile_command(&wasm, card);
    let hidden_before = wasm.game.hidden_card_info(card).unwrap().clone();
    let snapshot = RuntimeSavepoint::capture(&wasm);
    let hand = wasm.game.move_object_by_game_rule(card, Zone::Hand).unwrap();
    let returned = wasm.game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
    wasm.game.set_face_down(returned);
    wasm.game.effect_store.grant_registry.grant_play_from_to_card(returned, Zone::Exile, PlayerId(1),
        PlayFromConstraints::default(), GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    blind_exile_priority(&mut wasm);
    let now = wasm.game.hidden_card_info(returned).unwrap();
    assert_eq!((now.owner, now.slot, &now.commitment, now.zone),
        (hidden_before.owner, hidden_before.slot, &hidden_before.commitment, hidden_before.zone));
    assert_eq!(now.incarnation, Some(2));
    let UiCommand::PriorityAction { action_ref: Some(mut stale_reference), .. } = old else { unreachable!() };
    if let PriorityActionRef::OpenExiledCardForPlay { card_id, .. } = &mut stale_reference { *card_id = returned.0; }
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else { panic!("priority"); };
    assert!(resolve_priority_action(&wasm.game, priority, None, Some(&stale_reference)).unwrap().is_none());
    snapshot.restore(&mut wasm);
    assert_eq!(wasm.game.hidden_incarnation_high_water(), 0);
    assert_eq!(wasm.game.hidden_card_info(card).unwrap().incarnation, Some(0));
    let command = blind_exile_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    // Unavailable play reverses physical state while retaining only knowledge.
    assert!(wasm.game.is_face_down(card)); assert_eq!(wasm.game.hidden_card_info(card).unwrap().incarnation, Some(0));
    assert_eq!(wasm.game.hidden_incarnation_high_water(), 0);
    assert!(wasm.game.can_player_look_at_face_down_exiled_card(card, PlayerId(1)));
}

#[test]
fn blind_exile_face_down_intent_and_declaration_never_request_or_commit_an_identity_opening() {
    let _ids = crate::test_id_counter_guard();
    for materialized in [false, true] {
        let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 100, true);
        if !materialized { wasm.game.object_mut(card).unwrap().redact_to_hidden_card(); }
        wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = 3;
        blind_exile_priority(&mut wasm);
        let command = blind_exile_face_down_command(&wasm, card);
        assert!(wasm.blind_exile_opening_requirements(&command).unwrap().unwrap().is_empty());
        let context = wasm.pending_decision.clone().unwrap();
        let answer = wasm.command_to_replay_answer(&context, command.clone()).unwrap();
        wasm.commit_payment_command_disclosure(&context, &answer).unwrap();
        assert!(wasm.payment_disclosure.is_none());
        disclosure_command(&mut wasm, command).unwrap();
        let Some(DecisionContext::SelectOptions(options)) = wasm.pending_decision.as_ref() else { panic!("public declaration"); };
        assert!(options.exile_face_down_choice); assert!(!options.exile_play_choice);
        assert_eq!(options.options.len(), 4);
        assert_eq!(options.options[0].description, "Declare morph");
        assert_eq!(wasm.priority_state.pending_exile_face_down.as_ref().unwrap().incarnation, Some(0));
        let choice = UiCommand::SelectOptions { option_indices: vec![0] };
        assert!(wasm.blind_exile_opening_requirements(&choice).unwrap().unwrap().is_empty());
        let native = RuntimeSavepoint::capture(&wasm);
        wasm.priority_state.pending_exile_face_down = None;
        native.restore(&mut wasm);
        disclosure_command(&mut wasm, choice).unwrap();
        for _ in 0..20 {
            if !wasm.priority_state.has_pending_action() { break; }
            let declared = wasm.priority_state.declared_exile_face_down.as_ref().unwrap();
            assert_eq!((declared.card_id, declared.incarnation, declared.player), (card, Some(0), PlayerId(1)));
            assert!(wasm.payment_disclosure.is_none());
            match wasm.pending_decision.as_ref().unwrap() {
                DecisionContext::ManaPayment(_) => disclosure_confirm_mana(&mut wasm),
                DecisionContext::SelectOptions(options) => { let index = options.options.iter().find(|option| option.legal).unwrap().index; blind_exile_choose(&mut wasm, index); }
                context => panic!("unexpected public face-down continuation {context:?}"),
            }
        }
        assert!(!wasm.priority_state.has_pending_action()); assert_eq!(wasm.game.stack.len(), 1);
        let spell = wasm.game.stack[0].object_id;
        assert_ne!(spell, card); assert!(wasm.game.is_face_down(spell));
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().mana_pool.total(), 0);
        assert!(wasm.game.hidden_identity_obligations().iter().any(|obligation|
            obligation.check == ironsmith::game_state::HiddenIdentityCheck::CastFaceDown(ironsmith::game_state::FaceDownCastKind::Morph)));
        assert!(wasm.payment_disclosure.is_none()); assert!(!wasm.priority_epoch_undo_locked_by_disclosure);
        assert!(!wasm.last_crypto_requirements.iter().any(|requirement|
            requirement.requirement_type == "public_open" && matches!(requirement.object_id, Some(id) if id == card.0 || id == spell.0)));
    }
}

#[test]
fn blind_exile_failed_kind_and_clean_prefix_accept_the_same_future_declaration() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 100, false);
    let command = blind_exile_face_down_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    let prefix = RuntimeSavepoint::capture(&wasm);
    let accepted_hash_input = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
    assert!(disclosure_command(&mut wasm, UiCommand::SelectOptions { option_indices: vec![2] }).is_err());
    assert_eq!(serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap(), accepted_hash_input);
    assert_eq!(wasm.priority_state.pending_exile_face_down.as_ref().unwrap().declared_kind, None);
    assert!(wasm.payment_disclosure.is_none()); assert!(wasm.game.is_face_down(card));
    let after_failure = RuntimeSavepoint::capture(&wasm);
    let mut outcomes = Vec::new();
    for point in [after_failure, prefix] {
        point.restore(&mut wasm);
        wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = 3;
        disclosure_command(&mut wasm, UiCommand::SelectOptions { option_indices: vec![0] }).unwrap();
        for _ in 0..20 {
            if !wasm.priority_state.has_pending_action() { break; }
            match wasm.pending_decision.as_ref().unwrap() {
                DecisionContext::ManaPayment(_) => disclosure_confirm_mana(&mut wasm),
                DecisionContext::SelectOptions(options) => { let index = options.options.iter().find(|option| option.legal).unwrap().index; blind_exile_choose(&mut wasm, index); }
                context => panic!("unexpected public face-down continuation {context:?}"),
            }
        }
        assert!(!wasm.priority_state.has_pending_action()); assert_eq!(wasm.game.stack.len(), 1);
        let spell = wasm.game.stack[0].object_id;
        assert!(wasm.game.hidden_identity_obligations().iter().any(|obligation|
            obligation.check == ironsmith::game_state::HiddenIdentityCheck::CastFaceDown(ironsmith::game_state::FaceDownCastKind::Morph)));
        outcomes.push((wasm.game.is_face_down(spell), wasm.game.object(spell).unwrap().face_down_cast_state.as_ref().unwrap().disguise_ward,
            wasm.game.player(PlayerId(1)).unwrap().mana_pool.total()));
    }
    assert_eq!(outcomes, vec![(true, false, 0); 2]);
}

#[test]
fn blind_exile_effect_kind_retains_its_exact_source_public_receipt_after_departure() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 100, false);
    let actor = PlayerId(1);
    let source = wasm.game.create_object_from_definition(&CardDefinitionBuilder::new(CardId::new(), "Public face-down permission")
        .card_types(vec![CardType::Enchantment]).build(), actor, Zone::Battlefield);
    let stable = wasm.game.object(source).unwrap().stable_id;
    wasm.game.grant_face_down_cast_permission(ironsmith::game_state::FaceDownCastPermission {
        source, player: actor, zone: Zone::Exile, filter: ironsmith::target::ObjectFilter::creature(),
        description: "Cast an exiled creature face down".into(), requires_source_on_battlefield: false,
        expires_after_turn: None, single_use: true,
    });
    let departed = wasm.game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
    assert_ne!(source, departed); assert_eq!(wasm.game.object(departed).unwrap().stable_id, stable);
    blind_exile_priority(&mut wasm);
    let command = blind_exile_face_down_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    let declaration = wasm.priority_state.pending_exile_face_down.as_ref().unwrap();
    assert_eq!(declaration.kind_source_public_ids.get(&source), Some(&stable));
    assert!(!declaration.kind_source_public_ids.contains_key(&departed));
    let snapshot = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
    let kind = &snapshot["priorityRuntime"]["exileFaceDown"]["kinds"][3];
    assert_eq!(kind["permissionSource"], source.0); assert_eq!(kind["permissionSourceStableId"], stable.0.0);
    wasm.grand_melee_host_lanes.insert(7, GrandMeleeHostLane { runner: None, runner_awaiting_priority: true,
        trigger_queue: wasm.trigger_queue.clone(), priority_state: wasm.priority_state.clone() });
    let native = RuntimeSavepoint::capture(&wasm);
    wasm.priority_state.pending_exile_face_down.as_mut().unwrap().kind_source_public_ids.clear();
    assert!(wasm.try_build_public_audit_checkpoint().is_err());
    native.restore(&mut wasm);
    assert_eq!(serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap(), snapshot);
    wasm.grand_melee_host_lanes.get_mut(&7).unwrap().priority_state.pending_exile_face_down.as_mut().unwrap().kind_source_public_ids.clear();
    assert!(wasm.try_build_public_audit_checkpoint().is_err(), "inactive lane evidence is checked too");
}


#[test]
fn blind_exile_accepted_kind_remains_public_and_fixed_after_payment_rollback() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 100, false);
    wasm.game.player_mut(PlayerId(1)).unwrap().mana_pool.colorless = 3;
    blind_exile_priority(&mut wasm);
    let command = blind_exile_face_down_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
    blind_exile_choose(&mut wasm, 0);
    assert!(matches!(wasm.pending_decision, Some(DecisionContext::ManaPayment(_))));
    assert_eq!(wasm.priority_state.declared_exile_face_down.as_ref().unwrap().declared_kind,
        Some(ironsmith::game_state::FaceDownCastKind::Morph));
    disclosure_command(&mut wasm, UiCommand::ManaPayment { response: ManaPaymentCommand::Cancel }).unwrap();
    let accepted = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
    assert_eq!(accepted["priorityRuntime"]["exileFaceDown"]["declaredKind"]["kind"], "morph");
    assert_eq!(accepted["priorityRuntime"]["exileFaceDown"]["choicePending"], true);
    let native = RuntimeSavepoint::capture(&wasm);
    assert!(disclosure_command(&mut wasm, UiCommand::SelectOptions { option_indices: vec![2] }).is_err());
    assert_eq!(serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap(), accepted);
    native.restore(&mut wasm);
    assert_eq!(wasm.priority_state.pending_exile_face_down.as_ref().unwrap().declared_kind,
        Some(ironsmith::game_state::FaceDownCastKind::Morph));
    blind_exile_choose(&mut wasm, 3);
    assert!(!wasm.priority_state.has_pending_action()); assert!(wasm.game.stack.is_empty());
    assert!(wasm.game.is_face_down(card)); assert!(!wasm.game.can_player_look_at_face_down_exiled_card(card, PlayerId(1)));
    assert!(wasm.payment_disclosure.is_none()); assert_eq!(wasm.game.player(PlayerId(1)).unwrap().mana_pool.total(), 3);
}


#[test]
fn blind_exile_native_preview_and_dispatch_reject_index_only_opening_or_declaration() {
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 100, false);
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else { panic!("priority"); };
    let indices = priority.actions.iter().enumerate().filter_map(|(index, action)|
        matches!(action, LegalAction::OpenExiledCardForPlay { .. } | LegalAction::CastExiledCardFaceDown { .. }).then_some(index)).collect::<Vec<_>>();
    assert_eq!(indices.len(), 2);
    for index in indices {
        let command = UiCommand::PriorityAction { action_index: Some(index), action_ref: None };
        assert!(wasm.blind_exile_opening_requirements(&command).is_err());
        assert!(disclosure_command(&mut wasm, command).is_err());
        assert!(wasm.game.is_face_down(card)); assert!(wasm.payment_disclosure.is_none());
        assert!(!wasm.game.can_player_look_at_face_down_exiled_card(card, PlayerId(1)));
    }
}


#[test]
fn blind_exile_manabrew_response_dispatches_the_captured_ref_and_rejects_stale_prompt_origins() {
    use manabrew_protocol::prompts::{PromptInput, PromptOutput, ChooseActionOutput};
    let _ids = crate::test_id_counter_guard();
    for opening in [true, false] {
        let (mut wasm, _, card) = blind_exile_fixture(CardType::Sorcery, 100, false);
        let context = wasm.pending_decision.clone().unwrap();
        let (input, binding) = wasm.build_manabrew_prompt(&context).unwrap();
        assert_eq!(manabrew_protocol::protocol::PROTOCOL_VERSION, 3);
        let wire_input = serde_json::to_value(&input).unwrap();
        assert!(!wire_input.to_string().contains("action_ref"));
        assert!(!wire_input.to_string().contains("incarnation"));
        let input: PromptInput = serde_json::from_value(wire_input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&input).unwrap(), wire_input);
        let PromptInput::ChooseAction(available) = &input else { panic!("action prompt"); };
        let label = if opening { "Play exiled card" } else { "Cast exiled card face down" };
        let action_id = available.actions.iter().find(|action| matches!(&action.kind,
            manabrew_protocol::prompts::common::AvailableActionKind::Cast { label: actual, .. } if actual == label)).unwrap().id.clone();
        let open = ManabrewOpenPrompt { prompt_id: 7, deciding_player: PlayerId(1), decision_hash: hash_debug_value(&context),
            source_card_id: None, source_card: None, input, binding };
        wasm.manabrew_open_prompt = Some(open.clone());
        let answer = PromptOutput::ChooseAction(ChooseActionOutput::Act { action_id });
        let wire_answer = serde_json::to_value(&answer).unwrap();
        let answer: PromptOutput = serde_json::from_value(wire_answer.clone()).unwrap();
        assert_eq!(serde_json::to_value(&answer).unwrap(), wire_answer);
        wasm.validate_manabrew_response(PlayerId(1), 7, &answer).unwrap();
        let ManabrewResponseAction::Dispatch(command) = wasm.manabrew_response_action(&open, answer.clone()).unwrap() else { panic!("dispatch"); };
        assert!(matches!(command, UiCommand::PriorityAction { action_index: None, action_ref: Some(_) }));
        let point = RuntimeSavepoint::capture(&wasm);
        let mut changed = wasm.game.hidden_card_info(card).unwrap().clone(); changed.commitment = "wrong-current-commitment".into();
        wasm.game.set_hidden_card_info(card, changed);
        assert!(wasm.manabrew_response_action(&open, answer.clone()).is_err());
        point.clone().restore(&mut wasm);
        let moved = wasm.game.move_object_by_game_rule(card, Zone::Hand).unwrap();
        wasm.game.move_object_by_game_rule(moved, Zone::Exile).unwrap();
        assert!(wasm.manabrew_response_action(&open, answer.clone()).is_err(), "a frozen prompt cannot adopt a later incarnation");
        point.restore(&mut wasm);
        disclosure_command(&mut wasm, command).unwrap();
        if opening {
            assert!(wasm.game.can_player_look_at_face_down_exiled_card(card, PlayerId(1)));
        } else {
            assert!(wasm.priority_state.pending_exile_face_down.is_some());
            assert!(!wasm.game.can_player_look_at_face_down_exiled_card(card, PlayerId(1)));
        }
    }
}


#[test]
fn blind_exile_accepted_cancel_emits_the_same_claim_digest_across_runtime_ids_and_source_departure() {
    use ironsmith::grant_registry::{GrantSource, PlayFromConstraints};
    let _ids = crate::test_id_counter_guard();
    let (mut wasm, grant_source, original_card) = blind_exile_fixture(CardType::Sorcery, 100, false);
    let actor = PlayerId(1);
    let original_source = wasm.game.create_object_from_definition(&CardDefinitionBuilder::new(CardId::new(), "Face-down rule")
        .card_types(vec![CardType::Enchantment]).build(), actor, Zone::Battlefield);
    let prefix = RuntimeSavepoint::capture(&wasm);
    let mut emitted = Vec::new(); let mut runtime_ids = Vec::new();
    for offset in [0, 5] {
        prefix.clone().restore(&mut wasm);
        for _ in 0..offset { wasm.game.new_object_id(); }
        let graveyard = wasm.game.move_object_by_game_rule(original_source, Zone::Graveyard).unwrap();
        let source = wasm.game.move_object_by_game_rule(graveyard, Zone::Battlefield).unwrap();
        let hand = wasm.game.move_object_by_game_rule(original_card, Zone::Hand).unwrap();
        let card = wasm.game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
        wasm.game.set_face_down(card);
        wasm.game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, actor, PlayFromConstraints::default(),
            GrantSource::Effect { source_id: grant_source, expires_end_of_turn: u32::MAX });
        wasm.game.grant_face_down_cast_permission(ironsmith::game_state::FaceDownCastPermission {
            source, player: actor, zone: Zone::Exile, filter: ironsmith::target::ObjectFilter::creature(),
            description: "Public creature face-down rule".into(), requires_source_on_battlefield: false,
            expires_after_turn: None, single_use: true,
        });
        wasm.game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        assert!(wasm.game.object(source).is_none());
        wasm.game.player_mut(actor).unwrap().mana_pool.colorless = 3;
        blind_exile_priority(&mut wasm);
        let command = blind_exile_face_down_command(&wasm, card); disclosure_command(&mut wasm, command).unwrap();
        blind_exile_choose(&mut wasm, 3);
        assert!(matches!(wasm.pending_decision, Some(DecisionContext::ManaPayment(_))));
        disclosure_command(&mut wasm, UiCommand::ManaPayment { response: ManaPaymentCommand::Cancel }).unwrap();
        assert_eq!(wasm.game.hidden_face_down_cast_claim(card), Some(ironsmith::game_state::FaceDownCastKind::Permission { source }));
        let checkpoint = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
        assert!(checkpoint["hiddenClaimLedgerDigest"].is_string(), "the real emitted checkpoint must include its inner claim digest");
        let rules = wasm.hidden_claim_ledger_rules_state().unwrap();
        let claims = serde_json::to_value(&rules.hidden_face_down_cast_claims).unwrap();
        assert!(claims[0].get("object").is_none()); assert!(claims[0].get("permissionSource").is_none());
        assert_eq!(claims[0]["blindExileOrigin"]["permissionSourceStableId"], original_source.0);
        emitted.push((checkpoint["hiddenClaimLedgerDigest"].clone(), claims));
        runtime_ids.push((card, source));
    }
    assert_ne!(runtime_ids[0], runtime_ids[1]);
    assert_eq!(emitted[0], emitted[1], "the inner ledger comes from native accepted payment/cancel, not a synthetic outer receipt");
}
