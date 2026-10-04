/// One announced casting/activation transaction whose public hand information
/// cannot be taken back. The authoritative pending state and continuation own
/// the source/payer/X/targets/cost choices; this commitment prevents their
/// checkpoint from being replaced by the pre-announcement game on failure.
#[derive(Debug, Clone)]
pub(super) struct PaymentDisclosureCommitment {
    source: ObjectId,
    payer: PlayerId,
    hand_objects: std::collections::BTreeSet<ObjectId>,
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
    fn finish_payment_disclosure(&mut self) {
        if self.payment_disclosure.take().is_some() {
            self.priority_epoch_undo_locked_by_disclosure = true;
            self.payment_disclosure_generation =
                self.payment_disclosure_generation.saturating_add(1);
            self.cached_snapshot = None;
        }
    }

    fn payment_transaction_subject(&self) -> Option<(ObjectId, PlayerId)> {
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
                | LegalAction::ActivateManaAbility { source, .. },
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
    ) {
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let mut subject = self.payment_transaction_subject();
        let mut disclosed = Vec::new();
        match (context, answer) {
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
            return;
        };
        if disclosed.is_empty() {
            return;
        }
        let committed =
            self.payment_disclosure
                .get_or_insert_with(|| PaymentDisclosureCommitment {
                    source,
                    payer,
                    hand_objects: Default::default(),
                    required_retry: None,
                });
        committed.hand_objects.extend(disclosed);
        self.cached_snapshot = None;
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
        let may_disclose = match &context {
            DecisionContext::Priority(priority) => {
                priority.actions.iter().any(|action| match action {
                    LegalAction::ActivateAbility { source, .. }
                    | LegalAction::ActivateManaAbility { source, .. } => self
                        .game
                        .object(*source)
                        .is_some_and(|object| object.zone == Zone::Hand),
                    _ => false,
                })
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
        if self.payment_disclosure.is_none() && !may_disclose {
            return self.dispatch_routed_command(command, command_decode_ms);
        }
        let before = RuntimeSavepoint::capture(self);
        let answer = self.command_to_replay_answer(&context, command.clone())?;
        self.payment_disclosure_precheck(&answer)
            .map_err(|error| payment_disclosure_error(&error))?;
        self.commit_payment_command_disclosure(&context, &answer);
        let committed = self.payment_disclosure.clone();
        let committed_generation = self.payment_disclosure_generation;
        let result = self.dispatch_routed_command(command, command_decode_ms);
        if result.is_err() {
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
                .map(|committed| committed.hand_objects.clone())
                .unwrap_or_default();
            let answer = self.command_to_replay_answer(&context, command)?;
            self.payment_disclosure_precheck(&answer)
                .map_err(|error| payment_disclosure_error(&error))?;
            self.commit_payment_command_disclosure(&context, &answer);
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
                        .hand_objects
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
    /// Presentation of already disclosed payment identities after a retry
    /// restores their pre-command zones. The real GameState/audit hash remains
    /// the canonical accepted prefix; only these exact public cards are shown.
    fn payment_disclosure_view(&self) -> Option<ActiveViewedCards> {
        let committed = self.payment_disclosure.as_ref()?;
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let cards = committed
            .hand_objects
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
        let context = self
            .pending_decision
            .clone()
            .ok_or_else(|| payment_disclosure_error("no payment decision to recover"))?;
        let answer = self.command_to_replay_answer(&context, command)?;
        self.payment_disclosure_precheck(&answer)
            .map_err(|error| payment_disclosure_error(&error))?;
        self.commit_payment_command_disclosure(&context, &answer);
        let committed = self.payment_disclosure.as_mut().ok_or_else(|| {
            payment_disclosure_error("recovery command does not describe a hand-disclosing payment")
        })?;
        committed.required_retry = Some(format!("{answer:?}"));
        self.cached_snapshot = None;
        self.snapshot()
    }
}
