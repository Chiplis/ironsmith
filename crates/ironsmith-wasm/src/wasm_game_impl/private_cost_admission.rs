use ironsmith::special_actions::private_cost_admission::{
    PrivateActionOccurrence, PrivateActionStage, PrivateCostReservation,
    commit_private_action, private_action_cost, reserve_private_action_with_incarnation,
    stage_private_action, verify_public_source_completion,
    PrivateGrantIdentityReference, encode_private_grant_identity, resolve_private_grant_selection,
};

pub(super) const PRIVATE_COST_SOURCE_OPENING_REASON: &str = "private_action_completed_cost_source_opening";

fn private_occurrence<'a>(game:&GameState,action: &'a SpecialActionRef) -> Option<(ObjectId, PrivateActionOccurrence, &'a Option<PrivateCostReservationRef>)> {
    match action {
        SpecialActionRef::PrepareExile {card_id,method_index,preparation,cost_reservation} if matches!(preparation,ironsmith::alternative_cast::ExilePreparation::FaceUpSorcery{..}) => Some((ObjectId::from_raw(*card_id),PrivateActionOccurrence::Preparation{method_index:*method_index,preparation:preparation.clone()},cost_reservation)),
        SpecialActionRef::PrepareGrantedExile {card_id,grant,preparation,cost_reservation} if matches!(preparation,ironsmith::alternative_cast::ExilePreparation::FaceUpSorcery{..}) => {
            let identity:PrivateGrantIdentityReference=serde_json::from_value(cost_reservation.as_ref()?.grant_identity.clone()?).ok()?;
            let selected=resolve_private_grant_selection(game,ObjectId::from_raw(grant.source),grant.index,&identity).ok()?;
            Some((ObjectId::from_raw(*card_id),PrivateActionOccurrence::GrantedPreparation{grant:selected,preparation:preparation.clone()},cost_reservation))
        },
        SpecialActionRef::PerformIntrinsicAction {source_id,ability_index,cost_reservation} => Some((ObjectId::from_raw(*source_id),PrivateActionOccurrence::Intrinsic{ability_index:*ability_index},cost_reservation)),
        _=>None,
    }
}

fn private_claim_mut(action:&mut SpecialActionRef)->Option<&mut Option<PrivateCostReservationRef>> {
    match action {
        SpecialActionRef::PrepareExile{cost_reservation,..}|SpecialActionRef::PrepareGrantedExile{cost_reservation,..}|SpecialActionRef::PerformIntrinsicAction{cost_reservation,..}=>Some(cost_reservation),
        _=>None,
    }
}

pub(super) fn clear_private_cost_reservation(action:&mut PriorityActionRef) {
    if let PriorityActionRef::SpecialAction{action}=action {
        if let Some(claim)=private_claim_mut(action){*claim=None;}
    }
}

pub(super) fn attach_private_cost_reservation(game:&GameState,legal:&LegalAction,reference:&mut PriorityActionRef) {
    let PriorityActionRef::SpecialAction{action}=reference else{return};
    let (source,occurrence,grant_identity)=match legal {
        LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::PrepareGrantedExile{card_id,grant,preparation})
            if matches!(preparation,ironsmith::alternative_cast::ExilePreparation::FaceUpSorcery{..})=>{
                let identity=encode_private_grant_identity(game,grant).ok().and_then(|value|serde_json::to_value(value).ok());
                (*card_id,PrivateActionOccurrence::GrantedPreparation{grant:grant.clone(),preparation:preparation.clone()},identity)
            },
        _=>{let Some((source,occurrence,_))=private_occurrence(game,action) else{return};(source,occurrence,None)}
    };
    let Some(object)=game.object(source) else{return};
    let Some(hidden)=game.hidden_card_info(source) else{return};
    if object.zone!=Zone::Hand || game.is_publicly_revealed_hidden_card(source) {return;}
    // Encoding failure remains an invalid reservation, never a reason to open
    // the private card through the ordinary unconditional command-ID path.
    let cost=private_action_cost(game,object.owner,source,&occurrence)
        .ok().and_then(|cost|ironsmith_runtime_catalog::artifact_materializer::encode_runtime_total_cost(cost).ok())
        .and_then(|cost|serde_json::to_value(cost).ok()).unwrap_or(serde_json::Value::Null);
    let claim=PrivateCostReservationRef{actor:object.owner.0,source_stable_id:object.stable_id.0.0,hidden_incarnation:hidden.incarnation,cost,grant_identity};
    if let Some(slot)=private_claim_mut(action){*slot=Some(claim);}
}

impl WasmGame {
    fn private_cost_reservation_for_command(&self,context:&DecisionContext,command:&UiCommand)->Result<Option<PrivateCostReservation>,JsValue> {
        let UiCommand::PriorityAction{action_ref:Some(PriorityActionRef::SpecialAction{action}),..}=command else{return Ok(None)};
        let raw_private_source=match action {
            SpecialActionRef::PrepareExile{card_id,preparation,cost_reservation,..}|SpecialActionRef::PrepareGrantedExile{card_id,preparation,cost_reservation,..}
                if matches!(preparation,ironsmith::alternative_cast::ExilePreparation::FaceUpSorcery{..})=>Some((*card_id,cost_reservation)),
            SpecialActionRef::PerformIntrinsicAction{source_id,cost_reservation,..}=>Some((*source_id,cost_reservation)),
            _=>None,
        };
        if let Some((id,None))=raw_private_source {
            let source=ObjectId::from_raw(id);
            if self.game.object(source).is_some_and(|object|object.zone==Zone::Hand) && self.game.hidden_card_info(source).is_some() && !self.game.is_publicly_revealed_hidden_card(source) {
                return Err(payment_disclosure_error("private source action requires an authenticated cost reservation"));
            }
        }
        let occurrence=private_occurrence(&self.game,action);
        if occurrence.is_none() && private_claim_mut(&mut action.clone()).is_some_and(|claim|claim.is_some()) {
            return Err(payment_disclosure_error("private action permission identity is invalid"));
        }
        let Some((source,occurrence,claim))=occurrence else{return Ok(None)};
        let Some(claim)=claim else {
            if self.game.object(source).is_some_and(|object|object.zone==Zone::Hand) && self.game.hidden_card_info(source).is_some() && !self.game.is_publicly_revealed_hidden_card(source) {
                return Err(payment_disclosure_error("private source action requires an authenticated cost reservation"));
            }
            return Ok(None);
        };
        let DecisionContext::Priority(priority)=context else{return Err(payment_disclosure_error("private source action requires priority"))};
        let actor=priority.player;
        let object=self.game.object(source).ok_or_else(||payment_disclosure_error("private action source is missing"))?;
        if actor.0!=claim.actor || object.stable_id.0.0!=claim.source_stable_id {
            return Err(payment_disclosure_error("private action actor or source incarnation changed"));
        }
        let wire:ironsmith_core::TotalCost<ironsmith_compiled_artifact::WireCost>=serde_json::from_value(claim.cost.clone())
            .map_err(|_|payment_disclosure_error("invalid private action cost algebra"))?;
        let cost=ironsmith_runtime_catalog::artifact_materializer::restore_runtime_total_cost(wire)
            .map_err(|_|payment_disclosure_error("invalid private action executable cost"))?;
        reserve_private_action_with_incarnation(&self.game,actor,source,claim.hidden_incarnation,occurrence,cost)
            .map(Some).map_err(|_|payment_disclosure_error("private action reservation failed validation"))
    }

    fn request_private_cost_source_opening(&mut self,reservation:&PrivateCostReservation)->Result<(),String> {
        if !self.previewing_crypto_requirements {return Err("private action requires its final authenticated source opening".into());}
        let audit=self.capture_crypto_audit_state();
        let mut card=audit.hidden_by_id.get(&reservation.source()).cloned().ok_or("private source has no authenticated ciphertext anchor")?;
        // This is a proof requirement, not a synthetic event or published move.
        // Only the owner preview can establish the actual final public move;
        // placeholder peers reproduce its request after completing cost inputs.
        card.zone=Zone::Exile;
        let mut requirement=CryptoRequirementView::hidden_open("public_open",&card,None,"public",PRIVATE_COST_SOURCE_OPENING_REASON);
        requirement.timing=Some("pre".to_string());
        requirement.from=Some("hand".to_string());
        requirement.to=Some("exile".to_string());
        self.last_crypto_requirements.push(requirement);
        Ok(())
    }

    fn execute_private_cost_replay(&mut self,reservation:&PrivateCostReservation,dm:&mut WasmReplayDecisionMaker)->Result<GameProgress,String> {
        if self.game.is_publicly_revealed_hidden_card(reservation.source()) {
            if !commit_private_action(&mut self.game,reservation,dm).map_err(|error|error.to_string())? {return Ok(GameProgress::Continue);}
            self.priority_state.record_completed_player_action(&mut self.game,reservation.actor());
            return advance_priority_with_dm(&mut self.game,&mut self.trigger_queue,dm).map_err(|error|error.to_string());
        }
        let stage=match stage_private_action(&self.game,reservation,dm) {
            Ok(stage)=>stage,
            Err(error)=>{dm.pending_context=None;dm.pending_game=None;dm.viewed_cards=None;dm.audit_viewed_cards.clear();return Err(error.to_string());}
        };
        if matches!(&stage,PrivateActionStage::AwaitingDecision|PrivateActionStage::AwaitingAuthenticatedOpening) {
            let allowed_known_hand=|id:ObjectId| id!=reservation.source()
                && self.game.object(id).is_some_and(|object|object.owner==reservation.actor() && object.zone==Zone::Hand)
                && !self.game.is_hidden_card_placeholder(id);
            let unauthorized_view=dm.viewed_cards.iter().chain(dm.audit_viewed_cards.iter()).any(|view| {
                view.viewer!=reservation.actor() || view.subject!=reservation.actor() || view.zone!=Zone::Hand
                    || view.cards.iter().any(|id|!allowed_known_hand(*id))
            });
            let unauthorized_hidden_prompt=dm.pending_context.as_ref().is_some_and(|context|
                context.hidden_card_views().iter().flat_map(|view|view.object_ids.iter()).any(|id|!allowed_known_hand(*id)));
            if unauthorized_view || unauthorized_hidden_prompt {
                dm.pending_context=None;dm.pending_game=None;dm.viewed_cards=None;dm.audit_viewed_cards.clear();
                return Err("untrusted private price cannot acquire library, blind exile, or another player's private knowledge".into());
            }
        }
        match stage {
            PrivateActionStage::AwaitingDecision=>{
                let unauthorized=dm.pending_context.as_ref().is_some_and(|context| {
                    context.player()!=reservation.actor()
                        || context.hidden_card_views().iter().flat_map(|view|view.object_ids.iter()).any(|id| {
                            *id==reservation.source() || self.game.object(*id).is_none_or(|object|object.owner!=reservation.actor())
                        })
                        || matches!(context,DecisionContext::SelectObjects(objects) if objects.candidates.iter().any(|candidate| {
                            self.game.object(candidate.id).is_some_and(|object|object.zone.is_hidden() && (object.zone!=Zone::Hand || candidate.id==reservation.source() || object.owner!=reservation.actor() || self.game.is_hidden_card_placeholder(candidate.id)))
                        }))
                });
                if unauthorized {
                    dm.pending_context=None;dm.pending_game=None;dm.viewed_cards=None;dm.audit_viewed_cards.clear();
                    return Err("untrusted private price cannot request delegated private decisions or source disclosure".into());
                }
                Ok(GameProgress::Continue)
            }
            PrivateActionStage::AwaitingPostPaymentDecision=>{
                let candidate=dm.pending_game.as_deref().ok_or("completed private payment has no exact instruction snapshot")?;
                verify_public_source_completion(candidate,reservation).map_err(|error|error.to_string())?;
                if !self.previewing_crypto_requirements {
                    dm.pending_context=None;dm.pending_game=None;dm.viewed_cards=None;dm.audit_viewed_cards.clear();
                    return Err("private action body requires authenticated final source opening".into());
                }
                // The native phase proves all prices complete. Its exact body
                // suspension snapshot can now feed the actual move extractor.
                self.game=candidate.clone();
                Ok(GameProgress::Continue)
            }
            PrivateActionStage::AwaitingAuthenticatedOpening=>{
                self.request_private_cost_source_opening(reservation)?;
                let context=ironsmith::game_loop::priority_context(&self.game,reservation.actor()).map_err(|error|error.to_string())?;
                Ok(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(context)))
            }
            PrivateActionStage::CompletedPreview(candidate)=>{
                if !self.previewing_crypto_requirements {return Err("completed private action needs authenticated final source opening".into());}
                // RuntimeSavepoint restores this speculative world. It exists
                // only for the established actual-public-move proof extractor.
                self.game=*candidate;
                self.priority_state.record_completed_player_action(&mut self.game,reservation.actor());
                advance_priority_with_dm(&mut self.game,&mut self.trigger_queue,dm).map_err(|error|error.to_string())
            }
        }
    }
}


#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct VerifiedPublicPrivateActionOpening {
    object_id:u64,
    owner:u8,
    slot:u16,
    commitment:String,
}

#[wasm_bindgen]
impl WasmGame {
    /// Called only after the peer adapter verifies a PUBLIC manifest/position
    /// proof. Private owner lookups keep using revealHidden* alone. Hydrate the
    /// retained announcement checkpoint, never replace it by the live game.
    #[wasm_bindgen(js_name=recordVerifiedPublicHiddenOpening)]
    pub fn record_verified_public_hidden_opening(&mut self,input:JsValue)->Result<(),JsValue> {
        let input:VerifiedPublicPrivateActionOpening=serde_wasm_bindgen::from_value(input)
            .map_err(|_|payment_disclosure_error("invalid verified public source opening"))?;
        self.record_verified_public_hidden_opening_input(input)
    }

    fn record_verified_public_hidden_opening_input(&mut self,input:VerifiedPublicPrivateActionOpening)->Result<(),JsValue> {
        let source=ObjectId::from_raw(input.object_id);
        let Some(object)=self.game.object(source) else{return Ok(())};
        let Some(info)=self.game.hidden_card_info(source) else{return Ok(())};
        if object.owner.0!=input.owner || input.commitment.is_empty()
            || !((info.slot==input.slot && info.commitment==input.commitment)
                || (info.public_slot==Some(input.slot) && info.public_commitment.as_deref()==Some(input.commitment.as_str()))
                || (info.origin_slot==Some(input.slot) && info.origin_commitment.as_deref()==Some(input.commitment.as_str())))
            || self.game.is_hidden_card_placeholder(source) {
            return Err(payment_disclosure_error("verified public opening does not match its known source"));
        }
        let stable=object.stable_id;
        let definition=object.to_card_definition();
        let info=info.clone();
        let registry=&self.registry;
        let counters=snapshot_id_counters();
        let install=|checkpoint:&mut ReplayCheckpoint|->Result<(),JsValue>{
            let Some(original)=checkpoint.game.object(source) else{return Ok(())};
            if original.stable_id!=stable {return Err(payment_disclosure_error("verified source checkpoint incarnation changed"));}
            let Some(anchor)=checkpoint.game.hidden_card_info(source) else{return Ok(())};
            if anchor.incarnation!=info.incarnation || anchor.owner!=info.owner
                || !((anchor.slot==input.slot && anchor.commitment==input.commitment)
                    || (anchor.public_slot==Some(input.slot) && anchor.public_commitment.as_deref()==Some(input.commitment.as_str()))
                    || (anchor.origin_slot==Some(input.slot) && anchor.origin_commitment.as_deref()==Some(input.commitment.as_str()))) {
                return Err(payment_disclosure_error("verified source checkpoint ciphertext changed"));
            }
            // Preserve the root's original zone/ciphertext and every public
            // game decision. Only authenticated face knowledge is imported.
            checkpoint.id_counters.card=checkpoint.id_counters.card.max(counters.card);
            checkpoint.game.register_linked_face_family_from_catalog(&definition,registry);
            checkpoint.game.reveal_hidden_card_with_definition(source,&definition)
                .ok_or_else(||payment_disclosure_error("verified source checkpoint identity differs"))?;
            checkpoint.game.mark_hidden_cards_publicly_revealed(&[source]);
            Ok(())
        };
        if let Some(replay)=self.pending_replay_action.as_mut(){install(&mut replay.checkpoint)?;}
        if let Some(continuation)=self.pending_live_continuation.as_mut(){install(&mut continuation.checkpoint)?;continuation.speculative_progress=None;}
        if let Some(checkpoint)=self.pending_action_checkpoint.as_mut(){install(checkpoint)?;}
        if let Some(checkpoint)=self.priority_epoch_checkpoint.as_mut(){install(checkpoint)?;}
        self.game.mark_hidden_cards_publicly_revealed(&[source]);
        Ok(())
    }
}
