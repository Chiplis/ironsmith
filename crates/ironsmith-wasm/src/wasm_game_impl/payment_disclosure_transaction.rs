/// One announced casting/activation/special-action transaction whose public information
/// cannot be taken back. The authoritative pending state and continuation own
/// the source/payer/X/targets/cost choices; this commitment prevents their
/// checkpoint from being replaced by the pre-announcement game on failure.
#[derive(Debug, Clone)]
pub(super) struct PaymentDisclosureCommitment {
    source: ObjectId,
    payer: PlayerId,
    /// Hand payment cards and a face-down source opened by its native action.
    disclosed_objects: std::collections::BTreeSet<ObjectId>,
    /// A failed, already-disclosing command must be retried as the same native
    /// decision answer, rather than using a restored prompt to substitute it.
    required_retry: Option<String>,
}

fn payment_disclosure_error(message: &str) -> JsValue {
    #[cfg(target_arch = "wasm32")]
    {
        JsValue::from_str(message)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        JsValue::NULL
    }
}

impl WasmGame {
    /// Reverse the payment's game actions while keeping the identities that
    /// its signed commands have already made public. Map by stable identity:
    /// a paid discard may have allocated a new zone object before failing.
    fn retain_payment_disclosure_in_checkpoint(
        &self,
        checkpoint: &mut ReplayCheckpoint,
        disclosed_game: &GameState,
    ) {
        // Opening identities may have loaded new registry definitions. Their
        // CardIds survive with the disclosure and must not be allocated again.
        checkpoint.id_counters.card = checkpoint.id_counters.card.max(snapshot_id_counters().card);
        let mut disclosed = self.public_hand_disclosure_identities();
        if let Some(committed) = self.payment_disclosure.as_ref() {
            disclosed.extend(
                committed
                    .disclosed_objects
                    .iter()
                    // Ownership comes from the native pre-command checkpoint.
                    // A controlled face-down source need not belong to its payer,
                    // and later movement must not redefine who owned the disclosure.
                    .filter_map(|id| checkpoint.game.object(*id).map(|object| (object.owner, *id))),
            );
        }
        for (owner, id) in disclosed {
            let Some(original) = checkpoint.game.object(id) else {
                continue;
            };
            if original.owner != owner {
                continue;
            }
            let stable = original.stable_id;
            let known = [
                Some(disclosed_game),
                self.pending_decision_game.as_deref(),
                Some(&self.game),
            ]
            .into_iter()
            .flatten()
            .find_map(|game| {
                let current = game.find_object_by_stable_id(stable)?;
                let object = game.object(current)?;
                (object.owner == owner && object.card.is_some()).then(|| {
                    (
                        object.to_card_definition(),
                        game.hidden_card_info(current).cloned(),
                    )
                })
            });
            if let Some((definition, info)) = known {
                if let Some(mut info) = info {
                    info.zone = original.zone;
                    checkpoint.game.set_hidden_card_info(id, info);
                }
                checkpoint
                    .game
                    .reveal_hidden_card_with_definition(id, &definition);
            }
            checkpoint.game.mark_hidden_cards_publicly_revealed(&[id]);
        }
    }

    fn finish_payment_disclosure(&mut self) {
        if self.payment_disclosure.take().is_some() {
            self.priority_epoch_undo_locked_by_disclosure = true;
            self.payment_disclosure_generation =
                self.payment_disclosure_generation.saturating_add(1);
            self.cached_snapshot = None;
        }
    }

    fn payment_transaction_subject(&self) -> Option<(ObjectId, PlayerId)> {
        // Casting moves to a new ObjectId. The original opaque exile
        // incarnation and actor own this whole disclosure transaction.
        if let Some(opened) = self.priority_state.opened_exile_play.as_ref() {
            return Some((opened.card_id, opened.player));
        }
        if let Some(declared) = self.priority_state.declared_exile_face_down.as_ref() {
            return Some((declared.card_id, declared.player));
        }
        if let Some(DecisionContext::SelectObjects(objects)) = self.pending_decision.as_ref()
            && let Some(payment) = objects.cost_payment
        {
            return Some((payment.source, payment.payer));
        }
        if let Some(activation) = self.priority_state.pending_activation.as_ref() {
            return Some((activation.source, activation.activator));
        }
        if let Some(cast) = self.priority_state.pending_cast.as_ref() {
            return Some((cast.spell_id, cast.caster));
        }
        let root = self.pending_live_action_root.as_ref().or_else(|| {
            self.pending_replay_action
                .as_ref()
                .and_then(|replay| match &replay.root {
                    ReplayRoot::Response(response) => Some(response),
                    _ => None,
                })
        });
        match root {
            Some(PriorityResponse::PriorityAction(
                LegalAction::ActivateAbility { source, .. }
                | LegalAction::ActivateManaAbility { source, .. }
                | LegalAction::TurnFaceUp { creature_id: source, .. }
                | LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::TurnFaceUp {
                    permanent_id: source, ..
                }),
            )) => {
                let payer = self
                    .pending_action_checkpoint
                    .as_ref()
                    .and_then(|checkpoint| checkpoint.game.turn.priority_player)
                    .or(self.game.turn.priority_player)?;
                Some((*source, payer))
            }
            _ => None,
        }
    }

    fn payment_disclosure_precheck(&self, answer: &ReplayDecisionAnswer) -> Result<(), String> {
        if let (
            Some(DecisionContext::SelectObjects(objects)),
            ReplayDecisionAnswer::Objects(selected),
        ) = (self.pending_decision.as_ref(), answer)
            && (self.payment_transaction_subject().is_some() || self.payment_disclosure.is_some())
            && !objects.selection_satisfies_relation_filter(
                self.pending_decision_game.as_deref().unwrap_or(&self.game),
                selected,
            )
        {
            return Err(
                "selected hand group does not satisfy its payment filter or relation".into(),
            );
        }
        // Reject stale/client-authored inputs before publishing or latching a
        // retry. These did not execute a cost and must not pin an impossible
        // command over an otherwise valid disclosed transaction.
        if let (
            Some(DecisionContext::ManaPayment(payment)),
            ReplayDecisionAnswer::ManaPayment(response),
        ) = (self.pending_decision.as_ref(), answer)
        {
            use ironsmith::mana_payment::ManaPaymentResponse;
            match response {
                ManaPaymentResponse::Confirm {
                    plan_id,
                    request_hash,
                } if !payment.plan.payable
                    || *plan_id != payment.plan.id
                    || *request_hash != payment.plan.request_hash =>
                {
                    return Err("stale or client-authored disclosed payment plan".into());
                }
                ManaPaymentResponse::Activate {
                    source,
                    ability_index,
                } if !ironsmith::mana_payment::manual_mana_abilities(
                    &self.game,
                    &payment.request,
                )
                .contains(&(*source, *ability_index)) =>
                {
                    return Err("illegal mana activation during disclosed payment".into());
                }
                _ => {}
            }
        }
        if let Some(committed) = self.payment_disclosure.as_ref() {
            if self
                .payment_transaction_subject()
                .is_some_and(|(source, payer)| {
                    source != committed.source || payer != committed.payer
                })
            {
                return Err("disclosed payment source or payer changed before completion".into());
            }
            if matches!(
                answer,
                ReplayDecisionAnswer::ManaPayment(
                    ironsmith::mana_payment::ManaPaymentResponse::Cancel
                )
            ) {
                return Err("payment disclosed hand information; finish the committed payment instead of cancelling".into());
            }
            if committed
                .required_retry
                .as_ref()
                .is_some_and(|required| required != &format!("{answer:?}"))
            {
                return Err("resume the same disclosed payment command; its choices cannot be replaced after a failure".into());
            }
        }
        Ok(())
    }

    fn commit_payment_command_disclosure(
        &mut self,
        context: &DecisionContext,
        answer: &ReplayDecisionAnswer,
    ) -> Result<(), String> {
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let mut subject = self.payment_transaction_subject();
        let mut disclosed = Vec::new();
        match (context, answer) {
            (DecisionContext::Priority(_), ReplayDecisionAnswer::Priority(
                LegalAction::OpenExiledCardForPlay { card_id, incarnation, permission },
            )) => {
                ironsmith::alternative_cast::blind_play::validate_incarnation(game, *card_id, *incarnation)
                    .map_err(|error| error.to_string())?;
                let actor = ironsmith::alternative_cast::blind_play::priority_actor(game, *card_id, permission)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "blind exile opening no longer belongs to a priority actor".to_string())?;
                subject = Some((*card_id, actor));
                disclosed.push(*card_id);
            }
            (
                DecisionContext::Priority(priority),
                ReplayDecisionAnswer::Priority(
                    LegalAction::TurnFaceUp { creature_id: source, .. }
                    | LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::TurnFaceUp {
                        permanent_id: source, ..
                    }),
                ),
            ) if game.is_face_down(*source) && game.hidden_card_info(*source).is_some() => {
                // The existing transport opens the command's face-down
                // source before replay. Its payer may differ from its owner.
                subject = Some((*source, priority.player));
                disclosed.push(*source);
            }
            (
                DecisionContext::Priority(priority),
                ReplayDecisionAnswer::Priority(
                    LegalAction::ActivateAbility { source, .. }
                    | LegalAction::ActivateManaAbility { source, .. },
                ),
            ) if game
                .object(*source)
                .is_some_and(|object| object.zone == Zone::Hand) =>
            {
                subject = Some((*source, priority.player));
                disclosed.push(*source);
            }
            (DecisionContext::SelectObjects(objects), ReplayDecisionAnswer::Objects(selected))
                if subject.is_some() && objects.reveal_policy == SelectionRevealPolicy::Public =>
            {
                disclosed.extend(selected.iter().copied().filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.zone == Zone::Hand)
                }));
            }
            _ => {}
        }
        let Some((source, payer)) = subject else {
            return Ok(());
        };
        if disclosed.is_empty() {
            return Ok(());
        }
        let committed =
            self.payment_disclosure
                .get_or_insert_with(|| PaymentDisclosureCommitment {
                    source,
                    payer,
                    disclosed_objects: Default::default(),
                    required_retry: None,
                });
        committed.disclosed_objects.extend(disclosed);
        self.cached_snapshot = None;
        Ok(())
    }

    /// All accepted commands use the same lossless, per-command error boundary.
    /// A failed continuation cannot erase previously accepted payment choices,
    /// pending replacement context, or proof-required information commitments.
    fn dispatch_typed_command(
        &mut self,
        command: UiCommand,
        command_decode_ms: f64,
    ) -> Result<JsValue, JsValue> {
        if self.pending_priority_decision_is_stale() {
            self.recompute_stale_priority_decision()?;
            return Err(payment_disclosure_error(
                "pending priority decision no longer matches the game priority holder",
            ));
        }
        let context = self
            .pending_decision
            .clone()
            .ok_or_else(|| payment_disclosure_error("no pending decision to dispatch"))?;
        let may_disclose = matches!(&command, UiCommand::PriorityAction { action_ref: Some(PriorityActionRef::OpenExiledCardForPlay { .. }), .. }) || match &context {
            DecisionContext::Priority(priority) => {
                priority.actions.iter().any(|action| match action {
                    LegalAction::OpenExiledCardForPlay { .. } => true,
                    LegalAction::ActivateAbility { source, .. }
                    | LegalAction::ActivateManaAbility { source, .. } => self
                        .game
                        .object(*source)
                        .is_some_and(|object| object.zone == Zone::Hand),
                    LegalAction::TurnFaceUp { creature_id: source, .. }
                    | LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::TurnFaceUp {
                        permanent_id: source, ..
                    }) => self.game.is_face_down(*source) && self.game.hidden_card_info(*source).is_some(),
                    _ => false,
                }) || matches!(&command,
                    UiCommand::PriorityAction { action_ref: Some(
                        PriorityActionRef::ActivateAbility { source, .. }
                        | PriorityActionRef::ActivateManaAbility { source, .. }
                    ), .. } if self.game.object(ObjectId::from_raw(*source))
                        .is_some_and(|object| object.zone == Zone::Hand))
            }
            DecisionContext::SelectObjects(objects) => {
                self.payment_transaction_subject().is_some()
                    && objects.reveal_policy == SelectionRevealPolicy::Public
                    && objects.candidates.iter().any(|candidate| {
                        self.pending_decision_game
                            .as_deref()
                            .unwrap_or(&self.game)
                            .object(candidate.id)
                            .is_some_and(|object| object.zone == Zone::Hand)
                    })
            }
            _ => false,
        };
        let face_down_declaration = self.priority_state.pending_exile_face_down.is_some()
            || self.priority_state.declared_exile_face_down.is_some();
        if self.payment_disclosure.is_none() && !may_disclose && !face_down_declaration {
            return self.dispatch_routed_command(command, command_decode_ms);
        }
        let before = RuntimeSavepoint::capture(self);
        let answer = self.command_to_replay_answer(&context, command.clone())?;
        self.payment_disclosure_precheck(&answer)
            .map_err(|error| payment_disclosure_error(&error))?;
        self.commit_payment_command_disclosure(&context, &answer)
            .map_err(|error| payment_disclosure_error(&error))?;
        let committed = self.payment_disclosure.clone();
        let committed_generation = self.payment_disclosure_generation;
        let result = self.dispatch_routed_command(command, command_decode_ms);
        if result.is_err() {
            // Restore the command's accepted prefix exactly. An unsuccessful
            // nondisclosing kind choice cannot create a local-only kind lock;
            // an already accepted kind is retained by this savepoint itself.
            before.restore(self);
            // The transport pins the same unaccepted signed command before
            // sending its openings. The native continuation retains that same
            // answer here, including for a direct/local dispatcher.
            if let Some(mut committed) = committed {
                committed.required_retry = Some(format!("{answer:?}"));
                self.payment_disclosure = Some(committed);
                self.payment_disclosure_generation = committed_generation;
                self.cached_snapshot = None;
            }
        } else if let Some(committed) = self.payment_disclosure.as_mut() {
            committed.required_retry = None;
        }
        result
    }
}

#[wasm_bindgen]
impl WasmGame {
    /// Read-only metadata for the signed transport's disclosure boundary. It
    /// never opens a card or changes the authoritative payment/continuation.
    #[wasm_bindgen(js_name = getPaymentDisclosureForCommand)]
    pub fn get_payment_disclosure_for_command(
        &mut self,
        command: JsValue,
    ) -> Result<JsValue, JsValue> {
        let command: UiCommand = serde_wasm_bindgen::from_value(command).map_err(|error| {
            JsValue::from_str(&format!("invalid payment disclosure command: {error}"))
        })?;
        let saved = self.payment_disclosure.clone();
        let generation = self.payment_disclosure_generation;
        let cached = self.cached_snapshot.clone();
        let result = (|| {
            let context = self
                .pending_decision
                .clone()
                .ok_or_else(|| payment_disclosure_error("no decision for payment disclosure"))?;
            let before = self
                .payment_disclosure
                .as_ref()
                .map(|committed| committed.disclosed_objects.clone())
                .unwrap_or_default();
            let answer = self.command_to_replay_answer(&context, command)?;
            self.payment_disclosure_precheck(&answer)
                .map_err(|error| payment_disclosure_error(&error))?;
            self.commit_payment_command_disclosure(&context, &answer)
            .map_err(|error| payment_disclosure_error(&error))?;
            #[derive(Serialize)]
            struct Disclosure {
                required: bool,
                active: bool,
                objects: Vec<u64>,
            }
            let objects = self
                .payment_disclosure
                .as_ref()
                .map(|committed| {
                    committed
                        .disclosed_objects
                        .difference(&before)
                        .map(|object| object.0)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            serde_wasm_bindgen::to_value(&Disclosure {
                required: !objects.is_empty(),
                active: self.payment_disclosure.is_some(),
                objects,
            })
            .map_err(|error| payment_disclosure_error(&error.to_string()))
        })();
        self.payment_disclosure = saved;
        self.payment_disclosure_generation = generation;
        self.cached_snapshot = cached;
        result
    }
}

impl WasmGame {
    /// Both browser and JSON snapshots project the same already-public facts.
    fn include_payment_disclosure_views(&self, snapshot: &mut GameSnapshot) {
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        for view in [self.payment_disclosure_view(), self.payment_disclosure_source_view()]
            .into_iter().flatten()
        {
            snapshot.include_payment_disclosure(game, &view, &self.snapshot_object_view_cache);
        }
    }

    /// The source may be owned by someone other than the payer. Keep its
    /// already-public identity inspectable while a failed/pending payment
    /// leaves its rules characteristics face down.
    fn payment_disclosure_source_view(&self) -> Option<ActiveViewedCards> {
        let committed = self.payment_disclosure.as_ref()?;
        if !committed.disclosed_objects.contains(&committed.source) { return None; }
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let source = game.object(committed.source)?;
        if !matches!(source.zone, Zone::Battlefield | Zone::Exile) || !game.is_face_down(source.id)
            || game.is_hidden_card_placeholder(source.id) { return None; }
        Some(ActiveViewedCards {
            viewer: committed.payer,
            subject: source.owner,
            zone: source.zone,
            cards: vec![source.id],
            card_stable_ids: vec![source.stable_id],
            public: true,
            acknowledged_by: Vec::new(),
            source: Some(source.id),
            description: "Disclosed source of committed action".into(),
        })
    }

    /// Presentation of already disclosed payment identities after a retry
    /// restores their pre-command zones. The real GameState/audit hash remains
    /// the canonical accepted prefix; only these exact public cards are shown.
    fn payment_disclosure_view(&self) -> Option<ActiveViewedCards> {
        let committed = self.payment_disclosure.as_ref()?;
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let cards = committed
            .disclosed_objects
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    object.zone == Zone::Hand && object.owner == committed.payer
                })
            })
            .collect::<Vec<_>>();
        if cards.is_empty() {
            return None;
        }
        Some(ActiveViewedCards {
            viewer: committed.payer,
            subject: committed.payer,
            zone: Zone::Hand,
            card_stable_ids: stable_ids_for_viewed_cards(game, &cards),
            cards,
            public: true,
            acknowledged_by: Vec::new(),
            source: Some(committed.source),
            description: "Disclosed cards in committed payment".into(),
        })
    }
}

#[wasm_bindgen]
impl WasmGame {
    /// The signed transport calls this only after verifying/reopening an
    /// already published payment envelope while restoring its accepted prefix.
    /// It retains the exact command, without paying it a second time.
    #[wasm_bindgen(js_name = retainPaymentDisclosure)]
    pub fn retain_payment_disclosure(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: UiCommand = serde_wasm_bindgen::from_value(command).map_err(|error| {
            payment_disclosure_error(&format!("invalid payment recovery command: {error}"))
        })?;
        self.retain_payment_disclosure_command(command)
    }
}

impl WasmGame {
    fn retain_payment_disclosure_command(&mut self, command: UiCommand) -> Result<JsValue, JsValue> {
        let context = self
            .pending_decision
            .clone()
            .ok_or_else(|| payment_disclosure_error("no payment decision to recover"))?;
        let answer = self.command_to_replay_answer(&context, command)?;
        self.payment_disclosure_precheck(&answer)
            .map_err(|error| payment_disclosure_error(&error))?;
        self.commit_payment_command_disclosure(&context, &answer)
            .map_err(|error| payment_disclosure_error(&error))?;
        let committed = self.payment_disclosure.as_mut().ok_or_else(|| {
            payment_disclosure_error("recovery command does not describe an identity-disclosing payment")
        })?;
        committed.required_retry = Some(format!("{answer:?}"));
        self.cached_snapshot = None;
        self.snapshot()
    }
}
