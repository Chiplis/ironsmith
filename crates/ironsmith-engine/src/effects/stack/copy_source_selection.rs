//! Resolve one immutable definition operand through ordinary replacement order.
use std::sync::Arc;
use crate::{GameState,ObjectId,PlayerId};
use crate::effects::{ExecutionContext,ExecutionError};
use crate::events::{CopyCardSourceEvent,CopySourceOperation,Event};
#[derive(Debug,Clone)]
pub enum CopySourceSelection {Original,Supplied(Arc<crate::cards::CardDefinition>),Prevented,Pending}
pub fn select_copy_source_definition(game:&mut GameState,ctx:&mut ExecutionContext,operation:CopySourceOperation,player:PlayerId,original:Option<ObjectId>)->Result<CopySourceSelection,ExecutionError>{
 use crate::events::processing::{TraitEventResult,process_trait_event_with_execution_context};
 let definitions=crate::effects::cards::capture_replacement_definitions(game)?;
 let event=Event::new_with_provenance(CopyCardSourceEvent{operation,replacement_name:None,replacement_source:None,player,source:ctx.source,original,amount:1},ctx.provenance);
 match process_trait_event_with_execution_context(game,event,ctx)? {
  TraitEventResult::Proceed(event)|TraitEventResult::Modified(event)=>{
   let copy=event.inner().as_any().downcast_ref::<CopyCardSourceEvent>().ok_or_else(||ExecutionError::InternalError("copy-source proposal changed operation kind".into()))?;
   if copy.operation!=operation||copy.player!=player||copy.original!=original||copy.amount!=1{return Err(ExecutionError::IncompleteEvidence("copy-definition substitution changed the original actor, count, or operation context".into()));}
   match (&copy.replacement_name,copy.replacement_source) {
    (None,None)=>Ok(CopySourceSelection::Original),
    (Some(name),Some(source))=>Ok(CopySourceSelection::Supplied(definitions.resolve(source,name)?)),
    _=>Err(ExecutionError::IncompleteEvidence("copy-source substitution lost its exact provider or definition name".into())),
   }
  },
  TraitEventResult::Prevented=>Ok(CopySourceSelection::Prevented),
  TraitEventResult::NeedsChoice{..}|TraitEventResult::NeedsInteraction{..}=>if ctx.decision_maker.awaiting_choice(){Ok(CopySourceSelection::Pending)}else{Err(ExecutionError::InternalError("copy-source choice has no pending continuation".into()))},
  TraitEventResult::Replaced{..}|TraitEventResult::Expanded{..}=>Err(ExecutionError::IncompleteEvidence("copy-source selection requires a definition modification or prevention".into())),
 }
}

/// Read-only proposal callers never manufacture a player's answer. Forced
/// single replacements proceed; any requested decision suspends the projection.
#[derive(Default)]
pub(crate) struct ForcedCopySourceDecision {pending:bool}
impl crate::decision::DecisionMaker for ForcedCopySourceDecision {
 fn awaiting_choice(&self)->bool{self.pending}
 fn answers_player_choices(&self)->bool{false}
 fn decide_options(&mut self,_:&GameState,_:&crate::decisions::context::SelectOptionsContext)->Vec<usize>{self.pending=true;Vec::new()}
 fn decide_boolean(&mut self,_:&GameState,_:&crate::decisions::context::BooleanContext)->bool{self.pending=true;false}
 fn decide_objects(&mut self,_:&GameState,_:&crate::decisions::context::SelectObjectsContext)->Vec<ObjectId>{self.pending=true;Vec::new()}
 fn decide_colors(&mut self,_:&GameState,_:&crate::decisions::context::ColorsContext)->Vec<crate::color::Color>{self.pending=true;Vec::new()}
}
