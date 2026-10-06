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

#[derive(Clone, Debug)]
struct FaultingDiscardCost {
    discard: ironsmith::effect::Effect,
    failing: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl ironsmith::effects::EffectExecutor for FaultingDiscardCost {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ironsmith::effects::EffectContext,
    ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        ironsmith::effects::EffectExecutor::execute(
            &ironsmith::effects::SequenceEffect::new(vec![
                self.discard.clone(),
                ironsmith::effect::Effect::new(TransientPaymentFault(self.failing.clone())),
            ]),
            game,
            ctx,
        )
    }
    fn as_cost_executable(&self) -> Option<&dyn ironsmith::effects::CostExecutableEffect> {
        Some(self)
    }
    fn cost_description(&self) -> Option<String> {
        Some("Discard a card, then complete payment".into())
    }
}
impl ironsmith::effects::CostExecutableEffect for FaultingDiscardCost {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        payer: PlayerId,
    ) -> Result<(), ironsmith::effects::CostValidationError> {
        ironsmith::effects::CostExecutableEffect::can_execute_as_cost(
            self.discard.0.as_cost_executable().unwrap(), game, source, payer,
        )
    }
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        payer: PlayerId,
        reason: ironsmith::costs::PaymentReason,
    ) -> Result<(), ironsmith::effects::CostValidationError> {
        ironsmith::effects::CostExecutableEffect::can_execute_as_cost_with_reason(
            self.discard.0.as_cost_executable().unwrap(), game, source, payer, reason,
        )
    }
    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ironsmith::effects::EffectContext,
        reason: ironsmith::costs::PaymentReason,
    ) -> Result<(), ironsmith::effects::CostValidationError> {
        self.discard
            .0
            .as_cost_executable()
            .unwrap()
            .can_execute_as_cost_with_context(game, ctx, reason)
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
                            // Sequence's standalone preflight binds unannounced
                            // X to zero. Keep the printed discard validator so
                            // the injected fault only changes live execution.
                            FaultingDiscardCost {
                                discard: effect.clone(),
                                failing: failing.clone(),
                            },
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
    wasm.commit_payment_command_disclosure(&context, &ReplayDecisionAnswer::Priority(action)).unwrap();
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
        disclosed_objects: Default::default(), required_retry: None,
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
        disclosed_objects: Default::default(),
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

#[test]
fn payment_disclosure_unpayable_cycling_rolls_back_without_retry_lock() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let alice = PlayerId(0);
    let definition = ironsmith_registry_test::compile_to_runtime_definition(
        "Cycling payment",
        "Cycling {2}",
        false,
    )
    .unwrap();
    let card = wasm
        .game
        .create_object_from_definition(&definition, alice, Zone::Hand);
    payment_disclosure_track_hand(&mut wasm, card, 0);
    let ability_index = definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap();
    let action = LegalAction::ActivateAbility {
        source: card,
        ability_index,
    };
    wasm.pending_decision = Some(DecisionContext::Priority(
        PriorityContext::new(&wasm.game, alice, vec![action]).unwrap(),
    ));
    let before_hand = wasm.game.player(alice).unwrap().hand.clone();
    let before_pool = wasm.game.player(alice).unwrap().mana_pool.clone();
    disclosure_command(
        &mut wasm,
        UiCommand::PriorityAction {
            action_index: Some(0),
            action_ref: None,
        },
    )
    .expect("an unpayable cycling announcement must synchronize a rollback, not fail dispatch");
    assert!(
        wasm.payment_disclosure.is_none(),
        "a rules rollback must not demand an impossible retry"
    );
    assert!(wasm.priority_state.pending_activation.is_none());
    assert_eq!(wasm.game.player(alice).unwrap().hand, before_hand);
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool, before_pool);
    assert!(wasm.game.stack.is_empty());
    assert!(wasm.game.is_publicly_revealed_hidden_card(card));
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::Priority(_))
    ));
    disclosure_priority_matching(&mut wasm, |action| {
        matches!(action, LegalAction::PassPriority)
    });
}

#[test]
fn payment_disclosure_rules_rollback_keeps_opened_identity_and_commitment_on_peer() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, source) = manual_payment_fixture();
    let alice = PlayerId(0);
    let definition = ironsmith_registry_test::cards::definitions::ornithopter();
    let card =
        wasm.game
            .create_hidden_card_placeholder(alice, Zone::Hand, 3, "encrypted-position".into());
    let checkpoint = wasm.capture_replay_checkpoint();
    wasm.game
        .reveal_hidden_card_with_definition(card, &definition)
        .unwrap();
    let mut info = wasm.game.hidden_card_info(card).unwrap().clone();
    info.origin_slot = Some(3);
    info.origin_commitment = Some("encrypted-position".into());
    info.slot = 9;
    info.commitment = "opened-deck-slot".into();
    wasm.game.set_hidden_card_info(card, info.clone());
    wasm.payment_disclosure = Some(PaymentDisclosureCommitment {
        source,
        payer: alice,
        disclosed_objects: [card].into_iter().collect(),
        required_retry: None,
    });
    let disclosed = wasm.game.clone();
    // Model the engine's own automatic cost rollback, which restores an older
    // placeholder before the session restores its action boundary.
    wasm.restore_replay_checkpoint(&checkpoint);
    wasm.rollback_live_action_chain_to_checkpoint(
        checkpoint,
        &ironsmith::game_loop::GameLoopError::ActionCancelled("cost became unpayable".into()),
        &disclosed,
    )
    .unwrap();
    assert_eq!(wasm.game.object(card).unwrap().name.as_str(), "Ornithopter");
    assert_eq!(wasm.game.object(card).unwrap().zone, Zone::Hand);
    assert_eq!(wasm.game.hidden_card_info(card).unwrap(), &info);
    assert!(wasm.game.is_publicly_revealed_hidden_card(card));
    assert!(wasm.payment_disclosure.is_none());
    assert!(
        !wasm.is_cancelable(),
        "there is no completed action to undo across this disclosure"
    );
}

// Authored for the reconciliation; deliberately unrun while validation is deferred.
#[test]
fn foreign_owned_disclosure_survives_move_and_native_checkpoint_rollback() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, _) = manual_payment_fixture();
    let payer = PlayerId(0);
    let owner = PlayerId(1);
    let definition = ironsmith_registry_test::cards::definitions::ornithopter();
    let source = wasm.game.create_hidden_card_placeholder(
        owner, Zone::Battlefield, 4, "foreign-source-position".into(),
    );
    wasm.game.set_face_down(source);
    wasm.game.set_current_controller(source, payer);
    let mut checkpoint = wasm.capture_replay_checkpoint();
    let stable = checkpoint.game.object(source).unwrap().stable_id;
    wasm.game.reveal_hidden_card_with_definition(source, &definition).unwrap();
    wasm.payment_disclosure = Some(PaymentDisclosureCommitment {
        source, payer, disclosed_objects: [source].into_iter().collect(), required_retry: None,
    });
    let moved = wasm.game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert_ne!(moved, source);
    assert_eq!(wasm.game.object(moved).unwrap().stable_id, stable);
    let disclosed = wasm.game.clone();
    wasm.retain_payment_disclosure_in_checkpoint(&mut checkpoint, &disclosed);
    let restored = checkpoint.game.object(source).unwrap();
    assert_eq!(restored.owner, owner);
    assert_eq!(restored.zone, Zone::Battlefield);
    assert_eq!(restored.stable_id, stable);
    assert!(restored.card.is_some(), "the foreign owner's learned definition survives rollback");
    assert!(checkpoint.game.is_face_down(source));
    assert!(checkpoint.game.is_publicly_revealed_hidden_card(source));
    let mut identity = restored.clone();
    identity.end_face_down_cast_overlay();
    assert_eq!(identity.name.as_str(), "Ornithopter");
}

fn foreign_disclosed_source_snapshot_fixture() -> (WasmGame, ObjectId) {
    let (mut wasm, _) = manual_payment_fixture();
    let owner = PlayerId(1);
    let definition = ironsmith_registry_test::cards::definitions::ornithopter();
    let source = wasm.game.create_object_from_definition(&definition, owner, Zone::Battlefield);
    wasm.game.set_face_down(source);
    wasm.game.set_current_controller(source, PlayerId(0));
    wasm.payment_disclosure = Some(PaymentDisclosureCommitment {
        source, payer: PlayerId(0), disclosed_objects: [source].into_iter().collect(), required_retry: None,
    });
    (wasm, source)
}

#[test]
fn shared_snapshot_projection_keeps_foreign_disclosed_source_face_down() {
    let _guard = crate::test_id_counter_guard();
    let (mut wasm, source) = foreign_disclosed_source_snapshot_fixture();
    let snapshot: serde_json::Value = serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
    assert!(snapshot["players"][1]["persistent_look_cards"].as_array().unwrap().iter()
        .any(|card| card["id"] == source.0 && card["name"] == "Ornithopter"));
    assert!(wasm.game.is_face_down(source));
    assert_eq!(wasm.game.calculated_power(source), Some(2));
}

// Exercises the actual JS route when the wasm test target is enabled.
#[cfg(target_arch = "wasm32")]
#[test]
fn browser_and_json_snapshots_project_the_same_disclosed_foreign_source() {
    let (mut wasm, source) = foreign_disclosed_source_snapshot_fixture();
    let native: serde_json::Value = serde_json::from_str(&wasm.snapshot_json_for_host().unwrap()).unwrap();
    let browser: serde_json::Value = serde_wasm_bindgen::from_value(wasm.snapshot().unwrap()).unwrap();
    assert_eq!(browser["players"][1]["persistent_look_cards"], native["players"][1]["persistent_look_cards"]);
    assert!(browser["players"][1]["persistent_look_cards"].as_array().unwrap().iter()
        .any(|card| card["id"] == source.0 && card["name"] == "Ornithopter"));
    assert!(wasm.game.is_face_down(source));
}
