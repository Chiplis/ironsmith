// Source-only tests of resolving payment questions through the existing dispatcher.
fn nonmana_unless_definition(name: &str) -> CardDefinition {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/card-failure-campaign/nonmana-unless-counter-payments.json")).unwrap();
    let row = fixture["cards"].as_array().unwrap().iter().find(|row| row["name"] == name).unwrap();
    ironsmith_registry_test::compile_to_runtime_definition(name, row["text"].as_str().unwrap(), false).unwrap()
}
fn nonmana_unless_announce(wasm: &mut WasmGame, definition: &CardDefinition, player: PlayerId, target: Option<Target>) -> ObjectId {
    struct Targets(Option<Target>);
    impl ironsmith::decision::DecisionMaker for Targets {
        fn answers_player_choices(&self) -> bool { true }
        fn decide_targets(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
            if let Some(target) = self.0 { assert!(ctx.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target))); vec![target] }
            else { ironsmith::decision::SelectFirstDecisionMaker.decide_targets(game, ctx) }
        }
    }
    wasm.game.turn.priority_player = Some(player);
    let card = wasm.game.create_object_from_definition(definition, player, Zone::Hand);
    let stable = wasm.game.object(card).unwrap().stable_id;
    let action = compute_legal_actions(&wasm.game, player).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Normal, .. } if *spell_id == card)).unwrap();
    let mut dm = Targets(target);
    let mut progress = apply_priority_response_with_dm(&mut wasm.game, &mut wasm.trigger_queue,
        &mut wasm.priority_state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..30 {
        if let GameProgress::NeedsDecisionCtx(ref ctx) = progress
            && !matches!(ctx, DecisionContext::Priority(_)) {
            progress = ironsmith::game_loop::apply_decision_context_with_dm(&mut wasm.game,
                &mut wasm.trigger_queue, &mut wasm.priority_state, ctx, &mut dm).unwrap();
        } else { assert!(wasm.priority_state.pending_cast.is_none()); break; }
    }
    wasm.game.find_object_by_stable_id(stable).unwrap()
}

fn nonmana_unless_payment_fixture(name: &str, failing: std::sync::Arc<std::sync::atomic::AtomicBool>) -> (WasmGame, ObjectId, ObjectId, [ObjectId; 2], ObjectId) {
    let (mut wasm, _) = manual_payment_fixture();
    for player in [PlayerId(0), PlayerId(1)] { for mana in [ManaSymbol::Colorless, ManaSymbol::Blue, ManaSymbol::Black] {
        wasm.game.player_mut(player).unwrap().mana_pool.add(mana, 10);
    } }
    let smasher = if name == "Reality Smasher" {
        let spell = nonmana_unless_announce(&mut wasm, &nonmana_unless_definition(name), PlayerId(0), None);
        let stable = wasm.game.object(spell).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        Some(wasm.game.find_object_by_stable_id(stable).unwrap())
    } else { None };
    let mut hand = Vec::new();
    for (slot, card_name) in ["Payment first", "Payment second"].into_iter().enumerate() {
        let card = ironsmith::card::CardBuilder::new(CardId::new(), card_name).card_types(vec![CardType::Artifact]).build();
        let id = wasm.game.create_object_from_card(&card, PlayerId(1), Zone::Hand);
        wasm.game.set_hidden_card_info(id, ironsmith::game_state::HiddenCardInfo {
                incarnation: Some(0),
            owner: PlayerId(1), zone: Zone::Hand, slot: slot as u16, commitment: format!("unless-discard-{slot}"),
            origin_slot: None, origin_commitment: None, public_slot: None, public_commitment: None,
        });
        hand.push(id);
    }
    let foreign = wasm.game.create_object_from_card(&ironsmith::card::CardBuilder::new(CardId::new(), "Caster's private card")
        .card_types(vec![CardType::Artifact]).build(), PlayerId(0), Zone::Hand);
    let replaced_card = if name == "Perplex" { hand[0] } else { hand[1] };
    wasm.game.effect_store.replacement_effects.add_one_shot_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(foreign, PlayerId(0),
            ironsmith::events::WouldDiscardMatcher::any_player().with_card_filter(ironsmith::filter::ObjectFilter::specific(replaced_card)),
            ironsmith::replacement::ReplacementAction::Additionally(vec![ironsmith::effect::Effect::new(TransientPaymentFault(failing))])));
    let (source, victim) = if let Some(smasher) = smasher {
        let instant = ironsmith_registry_test::compile_to_runtime_definition("Targeting witness",
            "Mana cost: {U}\nType: Instant\nTap target creature.", false).unwrap();
        let victim = nonmana_unless_announce(&mut wasm, &instant, PlayerId(1), Some(Target::Object(smasher)));
        (smasher, victim)
    } else {
        wasm.game.turn.active_player = PlayerId(1);
        let witness = ironsmith_registry_test::compile_to_runtime_definition("Witness spell",
            "Mana cost: {1}\nType: Sorcery\nYou gain 1 life.", false).unwrap();
        let victim = nonmana_unless_announce(&mut wasm, &witness, PlayerId(1), None);
        let source = nonmana_unless_announce(&mut wasm, &nonmana_unless_definition(name), PlayerId(0), Some(Target::Object(victim)));
        (source, victim)
    };
    drain_pending_trigger_events(&mut wasm.game, &mut wasm.trigger_queue);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut wasm.game, &mut wasm.trigger_queue,
        &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    wasm.game.turn.priority_player = Some(PlayerId(0));
    wasm.priority_state = PriorityLoopState::new(wasm.game.players_in_game());
    payment_disclosure_prepare_priority(&mut wasm);
    for _ in 0..3 {
        if matches!(wasm.pending_decision, Some(DecisionContext::Boolean(_))) { break; }
        disclosure_priority_matching(&mut wasm, |action| matches!(action, LegalAction::PassPriority));
    }
    let Some(DecisionContext::Boolean(offer)) = wasm.pending_decision.as_ref() else { panic!("payment offer") };
    assert_eq!(offer.player, PlayerId(1));
    disclosure_command(&mut wasm, UiCommand::SelectOptions { option_indices: vec![1] }).unwrap();
    let Some(DecisionContext::SelectObjects(objects)) = wasm.pending_decision.as_ref() else { panic!("public payment choice") };
    assert_eq!(objects.player, PlayerId(1));
    assert_eq!(objects.cost_payment, Some(ironsmith::decisions::context::CostPaymentIdentity { source, payer: PlayerId(1) }));
    assert_eq!(objects.reveal_policy, SelectionRevealPolicy::Public);
    assert_eq!(wasm.payment_transaction_subject(), Some((source, PlayerId(1))));
    (wasm, source, victim, [hand[0], hand[1]], foreign)
}

#[test]
fn resolving_nonmana_unless_disclosure_uses_payer_not_counter_controller_and_retries_exact_payment() {
    let _guard = crate::test_id_counter_guard();
    for name in ["Perplex", "Reality Smasher"] {
        let failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (mut wasm, source, victim, hand, foreign) = nonmana_unless_payment_fixture(name, failing.clone());
        let selected = if name == "Perplex" { hand.to_vec() } else { vec![hand[1]] };
        let command = |ids: Vec<ObjectId>| UiCommand::SelectObjects { object_ids: ids.iter().map(|id| id.0).collect(),
            object_stable_ids: Vec::new(), object_hidden_refs: Vec::new() };
        let before_stack = format!("{:?}", wasm.game.stack);
        let before_hand = wasm.game.player(PlayerId(1)).unwrap().hand.clone();
        let wrong_actor = if name == "Perplex" { vec![hand[0], foreign] } else { vec![foreign] };
        assert!(disclosure_command(&mut wasm, command(wrong_actor)).is_err());
        assert!(wasm.payment_disclosure.is_none(), "illegal actor cannot pin an irreversible payment");
        assert!(disclosure_command(&mut wasm, command(selected.clone())).is_err());
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().hand, before_hand);
        assert_eq!(format!("{:?}", wasm.game.stack), before_stack);
        assert!(!wasm.is_cancelable());
        let held = wasm.payment_disclosure.as_ref().unwrap();
        assert_eq!(held.source, source); assert_eq!(held.payer, PlayerId(1));
        assert_eq!(held.disclosed_objects.iter().copied().collect::<Vec<_>>(), selected);
        assert!(wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::ManaPayment(
            ironsmith::mana_payment::ManaPaymentResponse::Cancel)).is_err());
        if name == "Reality Smasher" { assert!(disclosure_command(&mut wasm, command(vec![hand[0]])).is_err()); }
        failing.store(false, std::sync::atomic::Ordering::SeqCst);
        disclosure_command(&mut wasm, command(selected)).unwrap();
        assert!(wasm.game.stack.iter().any(|entry| !entry.is_ability && entry.object_id == victim));
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().hand.len(), if name == "Perplex" { 0 } else { 1 });
        assert_eq!(wasm.game.object(foreign).unwrap().zone, Zone::Hand);
        assert_eq!(wasm.game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::SpellCountered), 0);
        assert!(!wasm.is_cancelable());
    }
}
