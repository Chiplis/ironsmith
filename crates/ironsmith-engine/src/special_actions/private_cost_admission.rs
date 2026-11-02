//! Untrusted private-source cost reservations never authorize a commit.
//!
//! The transport authenticates the final public opening into the replay root.
//! This owner reselects that exact action and compares its complete cost before
//! executing any payment on the authoritative game. Pending work stays isolated.
//! The adapter replays this reservation against its immutable announcement
//! checkpoint, allowing only verified identity hydration before execution.
//! Unknown private rule bodies cannot be bound from a placeholder; public
//! context changes require a new action root, never a substituted reservation.
use super::*;
use crate::ids::StableId;
use std::sync::Arc;

type CanonicalComponent = ironsmith_core::Cost<Arc<str>>;
type CanonicalCost = ironsmith_core::TotalCost<CanonicalComponent>;
type CanonicalMethod = ironsmith_core::AlternativeCastingMethod<Arc<str>,CanonicalComponent,crate::static_abilities::ThisSpellCostCondition>;
type IntrinsicOccurrence = (crate::continuous::AbilityOrigin,Option<crate::ids::CardId>,crate::ability::Ability);

/// Portable reference to an already-public permission provider. Native
/// AbilityOrigin may contain process-local occurrence IDs, so it is resolved
/// from the public provider's slot inside the immutable announcement world.
#[cfg_attr(feature="serialization",derive(serde::Serialize,serde::Deserialize))]
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum PrivateGrantIdentityReference {
    SourceAbility { ability_index:usize },
    Stored { id:u64 },
    LinkedFace { face_name:String, slot:usize },
}
fn public_provider(game:&GameState,source:ObjectId)->Result<(),PrivateCostAdmissionError>{
    if game.object(source).is_none()||game.is_hidden_card_placeholder(source)
        ||game.hidden_identity_is_private(source)||game.is_face_down(source) {
        return Err(PrivateCostAdmissionError::InvalidOccurrence);
    }
    Ok(())
}
pub fn resolve_private_grant_selection(
    game:&GameState,source:ObjectId,index:usize,reference:&PrivateGrantIdentityReference,
)->Result<crate::alternative_cast::GrantSelection,PrivateCostAdmissionError>{
    use crate::grant_registry::GrantPermissionIdentity;
    let identity=match reference {
        PrivateGrantIdentityReference::Stored{id}=>GrantPermissionIdentity::Stored(*id),
        PrivateGrantIdentityReference::SourceAbility{ability_index}=>{
            public_provider(game,source)?;
            let chars=game.current_characteristics(source).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            if !matches!(chars.abilities.get(*ability_index).map(|ability|&ability.kind),Some(crate::ability::AbilityKind::Static(_))) {
                return Err(PrivateCostAdmissionError::InvalidOccurrence);
            }
            let origin=chars.abilities.origin(*ability_index).cloned().ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            let printed_face=matches!(&origin,crate::continuous::AbilityOrigin::Printed(_)).then(||game.object(source).and_then(|object|object.card)).flatten();
            GrantPermissionIdentity::Static{source,origin,printed_face}
        }
        PrivateGrantIdentityReference::LinkedFace{face_name,slot}=>{
            public_provider(game,source)?;
            let object=game.object(source).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            if object.linked_face_layout!=crate::card::LinkedFaceLayout::Split||matches!(object.zone,Zone::Battlefield|Zone::Stack){return Err(PrivateCostAdmissionError::InvalidOccurrence);}
            let definition=game.linked_face_definition_by_name_or_id(object.other_face_name.as_deref(),object.other_face).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            if definition.card.name.as_str()!=face_name.as_str()||*slot>=definition.abilities.len(){return Err(PrivateCostAdmissionError::InvalidOccurrence);}
            GrantPermissionIdentity::LinkedFace{source,face:definition.card.id,slot:*slot}
        }
    };
    Ok(crate::alternative_cast::GrantSelection{identity,source,index})
}
pub fn encode_private_grant_identity(
    game:&GameState,key:&crate::alternative_cast::GrantSelection,
)->Result<PrivateGrantIdentityReference,PrivateCostAdmissionError>{
    use crate::grant_registry::GrantPermissionIdentity;
    let reference=match &key.identity {
        GrantPermissionIdentity::Stored(id)=>PrivateGrantIdentityReference::Stored{id:*id},
        GrantPermissionIdentity::Static{source,origin,printed_face}=>{
            if *source!=key.source{return Err(PrivateCostAdmissionError::InvalidOccurrence);}
            public_provider(game,key.source)?;
            let chars=game.current_characteristics(key.source).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            let ability_index=chars.abilities.iter().enumerate().find_map(|(index,_)|(chars.abilities.origin(index)==Some(origin)).then_some(index)).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            let expected=matches!(origin,crate::continuous::AbilityOrigin::Printed(_)).then(||game.object(key.source).and_then(|object|object.card)).flatten();
            if &expected!=printed_face{return Err(PrivateCostAdmissionError::InvalidOccurrence);}
            PrivateGrantIdentityReference::SourceAbility{ability_index}
        }
        GrantPermissionIdentity::LinkedFace{source,face,slot}=>{
            if *source!=key.source{return Err(PrivateCostAdmissionError::InvalidOccurrence);}
            public_provider(game,key.source)?;
            let object=game.object(key.source).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            let definition=game.linked_face_definition_by_name_or_id(object.other_face_name.as_deref(),object.other_face).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
            if definition.card.id!=*face||*slot>=definition.abilities.len(){return Err(PrivateCostAdmissionError::InvalidOccurrence);}
            PrivateGrantIdentityReference::LinkedFace{face_name:definition.card.name.to_string(),slot:*slot}
        }
    };
    if resolve_private_grant_selection(game,key.source,key.index,&reference)?.identity!=key.identity{return Err(PrivateCostAdmissionError::InvalidOccurrence);}
    Ok(reference)
}

#[derive(Debug,Clone,PartialEq)]
pub enum PrivateActionOccurrence {
    Preparation { method_index:usize, preparation:crate::alternative_cast::ExilePreparation },
    GrantedPreparation { grant:crate::alternative_cast::GrantSelection, preparation:crate::alternative_cast::ExilePreparation },
    Intrinsic { ability_index:usize },
}
impl PrivateActionOccurrence {
    fn action(&self,source:ObjectId)->SpecialAction {match self {
        Self::Preparation{method_index,preparation}=>SpecialAction::PrepareExile{card_id:source,method_index:*method_index,preparation:preparation.clone()},
        Self::GrantedPreparation{grant,preparation}=>SpecialAction::PrepareGrantedExile{card_id:source,grant:grant.clone(),preparation:preparation.clone()},
        Self::Intrinsic{ability_index}=>SpecialAction::PerformIntrinsicAction{source_id:source,ability_index:*ability_index},
    }}
}

/// An immutable payment intent, deliberately distinct from authorization.
/// The exact native incarnation, actor and occurrence cannot be substituted by
/// later payment answers. Runtime cost display equality is never evidence.
#[derive(Debug,Clone)]
pub struct PrivateCostReservation {
    source:ObjectId,
    stable_id:StableId,
    hidden_source:Option<(PlayerId,Option<u64>,u16,String)>,
    actor:PlayerId,
    turn:u32,
    occurrence:PrivateActionOccurrence,
    cost:crate::cost::TotalCost,
    canonical_cost:CanonicalCost,
    known_method:Option<CanonicalMethod>,
    known_intrinsic:Option<IntrinsicOccurrence>,
}
impl PrivateCostReservation {
    pub fn source(&self)->ObjectId{self.source}
    pub fn actor(&self)->PlayerId{self.actor}
    pub fn hidden_incarnation(&self)->Option<u64>{self.hidden_source.as_ref().and_then(|binding|binding.1)}
    pub fn occurrence(&self)->&PrivateActionOccurrence{&self.occurrence}
    pub fn cost(&self)->&crate::cost::TotalCost{&self.cost}
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum PrivateCostAdmissionError {
    WrongSource,
    WrongActor,
    StaleTurn,
    InvalidTiming,
    MissingCanonicalCost,
    InvalidOccurrence,
    CostMismatch,
    AuthenticatedOpeningRequired,
    SourceRemainsPrivate,
    ActionFailed(String),
}
impl std::fmt::Display for PrivateCostAdmissionError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{write!(f,"private action cost admission: {self:?}")}
}
impl std::error::Error for PrivateCostAdmissionError{}

/// Stage output has no mutation of the caller's game. Only a completed local
/// preview may be inspected to derive the final command's public-move proof.
/// It is not a permit to install that speculative game as authoritative state.
pub enum PrivateActionStage {
    AwaitingDecision,
    /// A typed phase only. The adapter may inspect its replay decision maker's
    /// exact pending snapshot, and must still verify the public-source prefix.
    AwaitingPostPaymentDecision,
    AwaitingAuthenticatedOpening,
    CompletedPreview(Box<GameState>),
}

fn canonical_effect(effect:crate::effect::Effect)->Result<Arc<str>,PrivateCostAdmissionError>{
    effect.serialized_model().map(Arc::<str>::from).ok_or(PrivateCostAdmissionError::MissingCanonicalCost)
}
fn canonical_component(component:crate::costs::Cost)->Result<CanonicalComponent,PrivateCostAdmissionError>{
    component.compiled_model().cloned().ok_or(PrivateCostAdmissionError::MissingCanonicalCost)?.try_map_effect(canonical_effect)
}
fn canonical_method(method:crate::alternative_cast::AlternativeCastingMethod)->Result<CanonicalMethod,PrivateCostAdmissionError>{
    method.try_map(canonical_effect,canonical_component)
}
fn intrinsic_occurrence(game:&GameState,source:ObjectId,index:usize)->Result<IntrinsicOccurrence,PrivateCostAdmissionError>{
    let chars=game.current_characteristics(source).ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
    let origin=chars.abilities.origin(index).cloned().ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
    let ability=chars.abilities.get(index).cloned().ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
    if !matches!(&ability.kind,crate::ability::AbilityKind::Static(_)){return Err(PrivateCostAdmissionError::InvalidOccurrence);}
    Ok((origin,game.object(source).and_then(|object|object.card),ability))
}
fn selected_method(game:&GameState,reservation:&PrivateCostReservation)->Result<Option<CanonicalMethod>,PrivateCostAdmissionError>{
    let method=match &reservation.occurrence {
        PrivateActionOccurrence::Preparation{method_index,..}=>Some(game.object(reservation.source).and_then(|object|object.alternative_casts.get(*method_index)).cloned().ok_or(PrivateCostAdmissionError::InvalidOccurrence)?),
        PrivateActionOccurrence::GrantedPreparation{grant,preparation}=>Some(super::selected_granted_exile_preparation(game,reservation.actor,reservation.source,grant,preparation).map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?.method),
        PrivateActionOccurrence::Intrinsic{..}=>None,
    };
    method.map(canonical_method).transpose()
}
fn canonical_cost(cost:&crate::cost::TotalCost)->Result<CanonicalCost,PrivateCostAdmissionError>{
    cost.clone().try_map(canonical_component)
}
fn failure(error:ActionError)->PrivateCostAdmissionError{PrivateCostAdmissionError::ActionFailed(format!("{error:?}"))}
fn hidden_source_binding(game:&GameState,source:ObjectId)->Option<(PlayerId,Option<u64>,u16,String)>{
    game.hidden_card_info(source).map(|info|(info.owner,info.incarnation,
        info.origin_slot.unwrap_or(info.slot),info.origin_commitment.clone().unwrap_or_else(||info.commitment.clone())))
}
fn public_binding(game:&GameState,reservation:&PrivateCostReservation)->Result<(),PrivateCostAdmissionError>{
    let object=game.object(reservation.source).ok_or(PrivateCostAdmissionError::WrongSource)?;
    if object.stable_id!=reservation.stable_id||object.zone!=Zone::Hand
        ||hidden_source_binding(game,reservation.source)!=reservation.hidden_source{return Err(PrivateCostAdmissionError::WrongSource);}
    if object.owner!=reservation.actor||game.hidden_card_info(reservation.source).is_some_and(|info|info.owner!=reservation.actor){return Err(PrivateCostAdmissionError::WrongActor);}
    if game.turn.turn_number!=reservation.turn{return Err(PrivateCostAdmissionError::StaleTurn);}
    if !game.team_has_priority(reservation.actor){return Err(PrivateCostAdmissionError::InvalidTiming);}
    if let PrivateActionOccurrence::Preparation{method_index,preparation}=&reservation.occurrence {
        super::can_prepare_exile(game,reservation.actor,reservation.source,*method_index,preparation)
            .map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?;
    }
    Ok(())
}
pub fn verify_public_source_completion(candidate:&GameState,reservation:&PrivateCostReservation)->Result<(),PrivateCostAdmissionError>{
    // Imported opening flags prove identity, not a rules-authored disclosure.
    // Require this ordinary action's exact zone-result chain from the hand
    // incarnation to a new, public, face-up destination. A retained hand face
    // cannot satisfy this predicate merely because the transport hydrated it.
    let destination=candidate.current_object_id_after_zone_change(reservation.source)
        .filter(|destination|*destination!=reservation.source)
        .ok_or(PrivateCostAdmissionError::SourceRemainsPrivate)?;
    let object=candidate.object(destination).ok_or(PrivateCostAdmissionError::SourceRemainsPrivate)?;
    if object.stable_id!=reservation.stable_id||object.zone.is_hidden()
        ||candidate.is_face_down(destination) {
        return Err(PrivateCostAdmissionError::SourceRemainsPrivate);
    }
    Ok(())
}
fn authenticated_action(game:&GameState,reservation:&PrivateCostReservation)->Result<SpecialAction,PrivateCostAdmissionError>{
    public_binding(game,reservation)?;
    if game.is_hidden_card_placeholder(reservation.source){return Err(PrivateCostAdmissionError::AuthenticatedOpeningRequired);}
    let action=reservation.occurrence.action(reservation.source);
    // Intrinsic validation includes its current occurrence, actor, source zone,
    // condition, timing and per-object/ability use budget. Preparation validates
    // its method ordinal and complete declared preparation above.
    if let PrivateActionOccurrence::GrantedPreparation{grant,preparation}=&reservation.occurrence {
        super::selected_granted_exile_preparation(game,reservation.actor,reservation.source,grant,preparation)
            .map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?;
    }
    if let PrivateActionOccurrence::Intrinsic{ability_index}=&reservation.occurrence {
        super::intrinsic_action::validate(game,reservation.actor,reservation.source,*ability_index)
            .map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?;
    }
    if let Some(method)=&reservation.known_method {
        if selected_method(game,reservation)?.as_ref()!=Some(method){return Err(PrivateCostAdmissionError::InvalidOccurrence);}
    }
    if let Some(occurrence)=&reservation.known_intrinsic {
        let PrivateActionOccurrence::Intrinsic{ability_index}=&reservation.occurrence else{return Err(PrivateCostAdmissionError::InvalidOccurrence)};
        let current=intrinsic_occurrence(game,reservation.source,*ability_index)?;
        let same_definition=match (&occurrence.2.kind,&current.2.kind) {
            (crate::ability::AbilityKind::Static(left),crate::ability::AbilityKind::Static(right))=>Arc::ptr_eq(&left.0,&right.0),
            _=>false,
        };
        if occurrence.0!=current.0||occurrence.1!=current.1
            ||occurrence.2.functional_zones!=current.2.functional_zones||occurrence.2.designations!=current.2.designations
            ||(!same_definition&&!crate::ability_program_identity::ability_programs_equivalent(std::slice::from_ref(&occurrence.2),std::slice::from_ref(&current.2))) {
            return Err(PrivateCostAdmissionError::InvalidOccurrence);
        }
    }
    let payment=action.payment_spec(game,reservation.actor).map_err(failure)?
        .ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
    if payment.source!=reservation.source||canonical_cost(&payment.cost)?!=reservation.canonical_cost {
        return Err(PrivateCostAdmissionError::CostMismatch);
    }
    Ok(action)
}

/// Export the exact complete price selected on a known source. The adapter
/// serializes this through the canonical wire-cost codec. No hidden placeholder
/// or inferred mana-only fallback is accepted as an authored price here.
pub fn private_action_cost(
    game:&GameState,actor:PlayerId,source:ObjectId,occurrence:&PrivateActionOccurrence,
)->Result<crate::cost::TotalCost,PrivateCostAdmissionError>{
    if game.is_hidden_card_placeholder(source){return Err(PrivateCostAdmissionError::AuthenticatedOpeningRequired);}
    match occurrence {
        PrivateActionOccurrence::Preparation{method_index,preparation}=>
            super::can_prepare_exile(game,actor,source,*method_index,preparation).map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?,
        PrivateActionOccurrence::GrantedPreparation{grant,preparation}=>{
            super::selected_granted_exile_preparation(game,actor,source,grant,preparation).map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?;
        }
        PrivateActionOccurrence::Intrinsic{ability_index}=>{
            super::intrinsic_action::validate(game,actor,source,*ability_index).map_err(|_|PrivateCostAdmissionError::InvalidOccurrence)?;
        }
    }
    let payment=occurrence.action(source).payment_spec(game,actor).map_err(failure)?
        .ok_or(PrivateCostAdmissionError::InvalidOccurrence)?;
    canonical_cost(&payment.cost)?;
    Ok(payment.cost)
}

/// `cost` must come through the trusted wire-cost decoder, never a supplied
/// executor or an Oracle/display string. It remains an untrusted reservation
/// even when this peer privately knows the face.
pub fn reserve_private_action(
    game:&GameState,actor:PlayerId,source:ObjectId,occurrence:PrivateActionOccurrence,
    cost:crate::cost::TotalCost,
)->Result<PrivateCostReservation,PrivateCostAdmissionError>{
    let object=game.object(source).ok_or(PrivateCostAdmissionError::WrongSource)?;
    let mut reservation=PrivateCostReservation{source,stable_id:object.stable_id,hidden_source:hidden_source_binding(game,source),actor,turn:game.turn.turn_number,
        occurrence,canonical_cost:canonical_cost(&cost)?,cost,known_method:None,known_intrinsic:None};
    public_binding(game,&reservation)?;
    if !game.is_hidden_card_placeholder(source){
        authenticated_action(game,&reservation)?;
        reservation.known_method=selected_method(game,&reservation)?;
        if let PrivateActionOccurrence::Intrinsic{ability_index}=&reservation.occurrence {
            reservation.known_intrinsic=Some(intrinsic_occurrence(game,source,*ability_index)?);
        }
    }
    Ok(reservation)
}

/// Wire references must declare the exact hidden generation. Missing/old
/// metadata cannot silently become the currently known card incarnation.
pub fn reserve_private_action_with_incarnation(
    game:&GameState,actor:PlayerId,source:ObjectId,hidden_incarnation:Option<u64>,
    occurrence:PrivateActionOccurrence,cost:crate::cost::TotalCost,
)->Result<PrivateCostReservation,PrivateCostAdmissionError>{
    if let Some(info)=game.hidden_card_info(source) {
        if hidden_incarnation.is_none()||hidden_incarnation!=info.incarnation {
            return Err(PrivateCostAdmissionError::WrongSource);
        }
    } else if hidden_incarnation.is_some() {return Err(PrivateCostAdmissionError::WrongSource);}
    reserve_private_action(game,actor,source,occurrence,cost)
}

/// Replays the same recorded decisions on an isolated candidate. A peer with
/// only a placeholder can reproduce cost prompts, but cannot publish payment
/// or invent the source's action body. An owner with a private face can produce
/// the completed candidate used by the existing final public-move proof path.
pub fn stage_private_action(
    game:&GameState,reservation:&PrivateCostReservation,dm:&mut impl DecisionMaker,
)->Result<PrivateActionStage,PrivateCostAdmissionError>{
    public_binding(game,reservation)?;
    let mut candidate=game.clone();
    if game.is_hidden_card_placeholder(reservation.source){
        let payment=SpecialActionPayment{source:reservation.source,cost:reservation.cost.clone(),reason:crate::costs::PaymentReason::Other};
        let result=super::payment::pay_special_action_payment_with_x(&mut candidate,reservation.actor,&payment,None,None,dm);
        if dm.awaiting_choice(){return Ok(PrivateActionStage::AwaitingDecision);}
        result.map_err(failure)?;
        return Ok(PrivateActionStage::AwaitingAuthenticatedOpening);
    }
    let action=authenticated_action(game,reservation)?;
    let mut payment_completed=false;
    let result=super::perform_with_payment_completion(action,&mut candidate,reservation.actor,dm,&mut payment_completed);
    if dm.awaiting_choice(){return Ok(if payment_completed {PrivateActionStage::AwaitingPostPaymentDecision} else {PrivateActionStage::AwaitingDecision});}
    result.map_err(failure)?;
    verify_public_source_completion(&candidate,reservation)?;
    Ok(PrivateActionStage::CompletedPreview(Box::new(candidate)))
}

/// The only reservation commit path. A tracked source must have an actually
/// authenticated public opening in this replay root, not merely a local private
/// view. Verify the complete action/cost BEFORE paying. Failed or suspended
/// execution publishes neither payments, original actions nor partial receipts.
pub fn commit_private_action(
    game:&mut GameState,reservation:&PrivateCostReservation,dm:&mut impl DecisionMaker,
)->Result<bool,PrivateCostAdmissionError>{
    public_binding(game,reservation)?;
    if game.hidden_card_info(reservation.source).is_some()
        && !game.is_publicly_revealed_hidden_card(reservation.source){
        return Err(PrivateCostAdmissionError::AuthenticatedOpeningRequired);
    }
    let action=authenticated_action(game,reservation)?;
    let mut candidate=game.clone();
    let result=super::perform(action,&mut candidate,reservation.actor,dm);
    if dm.awaiting_choice(){return Ok(false);}
    result.map_err(failure)?;
    verify_public_source_completion(&candidate,reservation)?;
    *game=candidate;
    Ok(true)
}

#[cfg(test)]
#[path="private_cost_admission_tests.rs"]mod tests;
