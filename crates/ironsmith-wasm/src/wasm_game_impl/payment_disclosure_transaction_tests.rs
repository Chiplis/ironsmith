// UNVALIDATED. These tests use the public dispatcher's typed core, including
// its per-command runtime savepoint, rather than bypassing that boundary.
#[derive(Clone, Debug)]
struct TransientPaymentFault(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl ironsmith::effects::EffectExecutor for TransientPaymentFault {
    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ironsmith::effects::EffectContext,
    ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        if self.0.load(std::sync::atomic::Ordering::SeqCst) {
            Err(ironsmith::effects::ExecutionError::InternalError(
                "injected transient payment failure".into(),
            ))
        } else {
            Ok(ironsmith::effect::EffectOutcome::count(1))
        }
    }
    fn as_cost_executable(&self) -> Option<&dyn ironsmith::effects::CostExecutableEffect> {
        Some(self)
    }
    fn cost_description(&self) -> Option<String> {
        Some("Complete payment".into())
    }
}
impl ironsmith::effects::CostExecutableEffect for TransientPaymentFault {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _payer: PlayerId,
    ) -> Result<(), ironsmith::effects::CostValidationError> {
        Ok(())
    }
}

fn disclosure_command(wasm: &mut WasmGame, command: UiCommand) -> Result<JsValue, JsValue> {
    wasm.dispatch_typed_command(command, 0.0)
}
fn disclosure_confirm_mana(wasm: &mut WasmGame) {
    let Some(DecisionContext::ManaPayment(payment)) = wasm.pending_decision.as_ref() else {
        panic!("mana prompt");
    };
    let command = UiCommand::ManaPayment {
        response: ManaPaymentCommand::Confirm {
            plan_id: payment.plan.id.to_string(),
            request_hash: payment.plan.request_hash.to_string(),
        },
    };
    disclosure_command(wasm, command).unwrap();
}
fn disclosure_priority_matching(wasm: &mut WasmGame, predicate: impl Fn(&LegalAction) -> bool) {
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else {
        panic!("priority");
    };
    let index = priority
        .actions
        .iter()
        .position(predicate)
        .expect("legal action");
    disclosure_command(
        wasm,
        UiCommand::PriorityAction {
            action_index: Some(index),
            action_ref: None,
        },
    )
    .unwrap();
}
fn disclosure_activate(wasm: &mut WasmGame, source: ObjectId) {
    payment_disclosure_prepare_priority(wasm);
    let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else {
        panic!("priority");
    };
    let index = priority.actions.iter().position(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
    disclosure_command(
        wasm,
        UiCommand::PriorityAction {
            action_index: Some(index),
            action_ref: None,
        },
    )
    .unwrap();
}
fn disclosure_selection(card: ObjectId) -> UiCommand {
    UiCommand::SelectObjects {
        object_ids: vec![card.0],
        object_stable_ids: Vec::new(),
        object_hidden_refs: Vec::new(),
    }
}

#[test]
fn payment_disclosure_transaction_retries_same_choice_without_double_payment_or_losing_announcements()
 {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let alice = PlayerId(0);
    let mut definition = ironsmith_registry_test::compile_to_runtime_definition("Knollspine Invocation",
        "Mana cost: {1}{R}{R}\nType: Enchantment\n{X}, Discard a card with mana value X: This enchantment deals X damage to any target.", false).unwrap();
    let failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    // Fault injection is confined to the test: retain the actual printed
    // discard producer, then fail its continuation after the public selection.
    let activation = definition
        .abilities
        .iter_mut()
        .find_map(|ability| match &mut ability.kind {
            ironsmith::ability::AbilityKind::Activated(activation) => Some(activation),
            _ => None,
        })
        .unwrap();
    activation.mana_cost = ironsmith::cost::TotalCost::from_costs(
        activation
            .mana_cost
            .costs()
            .iter()
            .map(|cost| {
                if let Some(effect) = cost.effect_ref() {
                    if effect
                        .downcast_ref::<ironsmith::effects::DiscardEffect>()
                        .is_some()
                    {
                        return ironsmith::costs::Cost::try_effect(ironsmith::effect::Effect::new(
                            ironsmith::effects::SequenceEffect::new(vec![
                                effect.clone(),
                                ironsmith::effect::Effect::new(TransientPaymentFault(
                                    failing.clone(),
                                )),
                            ]),
                        ))
                        .unwrap();
                    }
                }
                cost.clone()
            })
            .collect(),
    );
    let source = wasm
        .game
        .create_object_from_definition(&definition, alice, Zone::Battlefield);
    let make_card = |name| {
        ironsmith::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::new().add_generic(3))
            .build()
    };
    let chosen =
        wasm.game
            .create_object_from_card(&make_card("Disclosed payment"), alice, Zone::Hand);
    let other = wasm.game.create_object_from_card(
        &make_card("Still private alternative"),
        alice,
        Zone::Hand,
    );
    payment_disclosure_track_hand(&mut wasm, chosen, 0);
    payment_disclosure_track_hand(&mut wasm, other, 1);
    let stable = wasm.game.object(chosen).unwrap().stable_id;
    wasm.game
        .player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    disclosure_activate(&mut wasm, source);
    for _ in 0..20 {
        let command = match wasm.pending_decision.as_ref().unwrap() {
            DecisionContext::SelectObjects(_) => break,
            DecisionContext::Number(_) => UiCommand::NumberChoice { value: 3 },
            DecisionContext::Targets(_) => UiCommand::SelectTargets {
                targets: vec![TargetInput::Player { player: 1 }],
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
            other => panic!("unexpected announcement: {other:?}"),
        };
        disclosure_command(&mut wasm, command).unwrap();
    }
    assert!(
        wasm.is_cancelable(),
        "no hand choice has been disclosed yet"
    );
    let before_pool = wasm.game.player(alice).unwrap().mana_pool.clone();
    let before_hand = wasm.game.player(alice).unwrap().hand.clone();
    let before_pending = format!("{:?}", wasm.priority_state.pending_activation);
    let pending = RuntimeSavepoint::capture(&wasm);
    wasm.cancel_decision().unwrap();
    assert_eq!(wasm.game.player(alice).unwrap().hand, before_hand);
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool, before_pool);
    assert!(wasm.game.stack.is_empty());
    assert!(wasm.payment_disclosure.is_none());
    pending.restore(&mut wasm);
    assert!(disclosure_command(&mut wasm, disclosure_selection(chosen)).is_err());
    assert_eq!(wasm.game.player(alice).unwrap().hand, before_hand);
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool, before_pool);
    assert_eq!(
        format!("{:?}", wasm.priority_state.pending_activation),
        before_pending,
        "payer/source/X/targets/prepared mana and prior cost state survive the failed command"
    );
    assert!(wasm.game.stack.is_empty());
    assert!(!wasm.is_cancelable());
    assert!(
        wasm.payment_disclosure_precheck(&ReplayDecisionAnswer::ManaPayment(
            ironsmith::mana_payment::ManaPaymentResponse::Cancel
        ))
        .is_err()
    );
    assert!(
        disclosure_command(&mut wasm, disclosure_selection(other)).is_err(),
        "failed public choice cannot be replaced"
    );
    wasm.perspective = PlayerId(1);
    let visible: serde_json::Value =
        serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
    let hand = visible["players"][0]["hand_cards"].as_array().unwrap();
    assert!(hand.iter().any(|card| card["name"] == "Disclosed payment"));
    assert!(
        !hand
            .iter()
            .any(|card| card["name"] == "Still private alternative")
    );
    wasm.perspective = alice;
    failing.store(false, std::sync::atomic::Ordering::SeqCst);
    disclosure_command(&mut wasm, disclosure_selection(chosen)).unwrap();
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::Priority(_))
    ));
    assert_eq!(wasm.game.stack.len(), 1);
    assert_eq!(
        wasm.game
            .object(wasm.game.find_object_by_stable_id(stable).unwrap())
            .unwrap()
            .zone,
        Zone::Graveyard
    );
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool.total(), 0);
    assert_eq!(wasm.game.player(alice).unwrap().hand, vec![other]);
    let discards = wasm
        .game
        .turn_store
        .turn_history
        .event_records
        .iter()
        .filter(|record| {
            record
                .event
                .downcast::<ironsmith::events::other::CardDiscardedEvent>()
                .is_some()
        })
        .count();
    assert_eq!(discards, 1);
    assert!(wasm.payment_disclosure.is_none());
    assert!(wasm.priority_epoch_undo_locked_by_disclosure);
    assert!(!wasm.is_cancelable());
}

#[test]
fn payment_disclosure_transaction_savepoints_remove_only_speculative_commitments() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let source = wasm.game.create_object_from_definition(&ironsmith_registry_test::compile_to_runtime_definition(
        "Hand payment control", "Type: Creature\nPower/Toughness: 1/1\n{1}, Exile this card from your hand: You gain 1 life.", false).unwrap(), PlayerId(0), Zone::Hand);
    wasm.game
        .player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    payment_disclosure_prepare_priority(&mut wasm);
    let context = wasm.pending_decision.clone().unwrap();
    let action = match &context { DecisionContext::Priority(priority) => priority.actions.iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap().clone(), _ => unreachable!() };
    let saved = RuntimeSavepoint::capture(&wasm);
    let public_before = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
    wasm.commit_payment_command_disclosure(&context, &ReplayDecisionAnswer::Priority(action));
    assert!(wasm.payment_disclosure.is_some());
    assert_eq!(
        serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap(),
        public_before,
        "recovery knowledge must not change the signed accepted-prefix state hash"
    );
    saved.restore(&mut wasm);
    assert!(
        wasm.payment_disclosure.is_none(),
        "speculative preview does not publish or latch a commitment"
    );
}

#[test]
fn payment_disclosure_transaction_native_savepoint_keeps_live_commitment() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    wasm.payment_disclosure = Some(PaymentDisclosureCommitment {
        source: ObjectId::from_raw(1), payer: PlayerId(0),
        hand_objects: Default::default(), required_retry: None,
    });
    let saved = RuntimeSavepoint::capture(&wasm);
    wasm.payment_disclosure = None;
    saved.restore(&mut wasm);
    assert!(wasm.payment_disclosure.is_some());
}

#[test]
fn payment_disclosure_transaction_finished_replay_cannot_undo_but_later_safe_action_can() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let before = wasm.capture_replay_checkpoint();
    wasm.payment_disclosure = Some(PaymentDisclosureCommitment {
        source: ObjectId::from_raw(1),
        payer: PlayerId(0),
        hand_objects: Default::default(),
        required_retry: None,
    });
    wasm.finish_payment_disclosure();
    let old = PendingReplayAction {
        checkpoint: before,
        root: ReplayRoot::Response(PriorityResponse::PriorityAction(
            LegalAction::ActivateAbility {
                source: ObjectId::from_raw(1),
                ability_index: 0,
            },
        )),
        nested_answers: Vec::new(),
    };
    assert!(
        !wasm.is_replay_chain_cancelable(&old),
        "a completed disclosed action stays irrevocable even if no movement event was produced"
    );
    wasm.pending_replay_action = None;
    wasm.pending_action_checkpoint = Some(wasm.capture_replay_checkpoint());
    assert!(
        wasm.is_cancelable(),
        "a later action may cancel back to a checkpoint after the disclosure"
    );
}

#[test]
fn payment_disclosure_transaction_rejects_stale_plan_and_illegal_source_then_accepts_current_plan()
{
    let _guard = crate::test_id_counter_guard();
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/snc_exiled_land_mana_grants.json.fixture"
    ))
    .unwrap();
    let definition = ironsmith_registry_test::compile_to_runtime_definition(
        cards[0]["name"].as_str().unwrap(),
        cards[0]["text"].as_str().unwrap(),
        false,
    )
    .unwrap();
    let (mut wasm, land) = manual_payment_fixture();
    let source = wasm
        .game
        .create_object_from_definition(&definition, PlayerId(0), Zone::Hand);
    payment_disclosure_track_hand(&mut wasm, source, 0);
    wasm.game
        .player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let opponent_land = wasm.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::basic_mountain(),
        PlayerId(1),
        Zone::Battlefield,
    );
    disclosure_activate(&mut wasm, source);
    for _ in 0..8 {
        match wasm.pending_decision.as_ref().unwrap() {
            DecisionContext::ManaPayment(_) => break,
            DecisionContext::Targets(_) => {
                disclosure_command(
                    &mut wasm,
                    UiCommand::SelectTargets {
                        targets: vec![TargetInput::Object { object: land.0 }],
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected payment setup: {other:?}"),
        }
    }
    assert!(
        wasm.payment_disclosure.is_some(),
        "hand activation has already disclosed its source"
    );
    let Some(DecisionContext::ManaPayment(payment)) = wasm.pending_decision.as_ref() else {
        panic!("mana payment");
    };
    let stale = UiCommand::ManaPayment {
        response: ManaPaymentCommand::Confirm {
            plan_id: payment.plan.id.wrapping_add(1).to_string(),
            request_hash: payment.plan.request_hash.to_string(),
        },
    };
    let before = format!("{:?}", wasm.priority_state.pending_activation);
    assert!(disclosure_command(&mut wasm, stale).is_err());
    assert!(
        wasm.payment_disclosure
            .as_ref()
            .unwrap()
            .required_retry
            .is_none()
    );
    assert!(
        disclosure_command(
            &mut wasm,
            UiCommand::ManaPayment {
                response: ManaPaymentCommand::Activate {
                    source_id: opponent_land.0.to_string(),
                    ability_index: 0,
                }
            }
        )
        .is_err()
    );
    assert!(
        wasm.payment_disclosure
            .as_ref()
            .unwrap()
            .required_retry
            .is_none()
    );
    assert_eq!(
        format!("{:?}", wasm.priority_state.pending_activation),
        before
    );
    disclosure_confirm_mana(&mut wasm);
    assert_eq!(wasm.game.stack.len(), 1);
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(), 0);
    assert!(!wasm.game.is_tapped(opponent_land));
    assert!(wasm.payment_disclosure.is_none());
    assert!(!wasm.is_cancelable());
}
