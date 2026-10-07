// The command authorizes opening one opaque exile incarnation, before any
// spell/land proposal. This requirement is derived without reading its face.
impl WasmGame {
    fn blind_exile_opening_requirements(
        &mut self, command: &UiCommand,
    ) -> Result<Option<Vec<CryptoRequirementView>>, JsValue> {
        if let Some(DecisionContext::SelectOptions(options)) = self.pending_decision.as_ref()
            && options.exile_face_down_choice {
            // The generic public declaration chooses no private identity.
            // Validate its option shape without probing the concealed face.
            self.command_to_replay_answer(&DecisionContext::SelectOptions(options.clone()), command.clone())?;
            return Ok(Some(Vec::new()));
        }
        let UiCommand::PriorityAction { action_index, action_ref } = command else { return Ok(None); };
        let Some(DecisionContext::Priority(priority)) = self.pending_decision.as_ref() else { return Ok(None); };
        let candidate_is_opening = matches!(action_ref, Some(PriorityActionRef::OpenExiledCardForPlay { .. } | PriorityActionRef::CastExiledCardFaceDown { .. }))
            || (action_ref.is_none() && action_index.and_then(|index| priority.actions.get(index))
                .is_some_and(|action| matches!(action, LegalAction::OpenExiledCardForPlay { .. } | LegalAction::CastExiledCardFaceDown { .. })));
        if !candidate_is_opening { return Ok(None); }
        let action = resolve_priority_action(&self.game, priority, *action_index, action_ref.as_ref())
            .map_err(|error| payment_disclosure_error(&error.to_string()))?
            .ok_or_else(|| payment_disclosure_error("stale blind exile opening authority"))?;
        if matches!(action, LegalAction::CastExiledCardFaceDown { .. }) { return Ok(Some(Vec::new())); }
        let LegalAction::OpenExiledCardForPlay { card_id, .. } = action else { unreachable!(); };
        let Some(info) = self.game.hidden_card_info(card_id) else {
            // A local fully known game needs no cryptographic material, but
            // still crosses the same irreversible native disclosure boundary.
            return Ok(Some(Vec::new()));
        };
        let card = HiddenAuditCard {
            object_id: card_id, owner: info.owner, zone: Zone::Exile,
            slot: info.slot, commitment: info.commitment.clone(),
            origin_slot: info.origin_slot, origin_commitment: info.origin_commitment.clone(),
            public_slot: info.public_slot, public_commitment: info.public_commitment.clone(),
            card: None, face_down: true, foretold: false,
        };
        let mut requirement = CryptoRequirementView::hidden_open(
            "public_open", &card, None, "public", "announce play of unseen exiled card",
        );
        requirement.timing = Some("pre".into());
        Ok(Some(vec![requirement]))
    }
}
