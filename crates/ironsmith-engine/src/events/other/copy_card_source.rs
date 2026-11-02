//! A replacement proposal for the definition operand of a copy operation.
use std::any::Any;
use crate::{GameState,ObjectId,PlayerId};
use crate::events::{EventKind,GameEventType,ReplacementMatcher};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CopySourceOperation {SpellCopy,CastCopy}
#[derive(Debug,Clone)]
pub struct CopyCardSourceEvent {
 pub operation:CopySourceOperation,
 pub replacement_name:Option<String>,
 pub replacement_source:Option<ObjectId>,
 pub player:PlayerId,
 pub source:ObjectId,
 pub original:Option<ObjectId>,
 pub amount:u32,
}
impl GameEventType for CopyCardSourceEvent {
 fn event_kind(&self)->EventKind{EventKind::KeywordAction}
 fn is_replacement_proposal(&self)->bool{true}
 fn affected_player(&self,_:&GameState)->PlayerId{self.player}
 fn player(&self)->Option<PlayerId>{Some(self.player)}
 fn controller(&self)->Option<PlayerId>{Some(self.player)}
 fn source_object(&self)->Option<ObjectId>{Some(self.source)}
 fn object_id(&self)->Option<ObjectId>{self.original}
 fn display(&self)->String{"Choose the source definition for a copy operation".into()}
 fn as_any(&self)->&dyn Any{self}
}
#[derive(Debug,Clone,PartialEq)]
pub struct WouldSelectCopySourceMatcher {pub player:crate::target::PlayerFilter}
impl ReplacementMatcher for WouldSelectCopySourceMatcher {
 fn matches_prepared_event(&self,event:&dyn GameEventType,ctx:&crate::events::context::PreparedEventContext)->bool{
  let Some(copy)=event.as_any().downcast_ref::<CopyCardSourceEvent>()else{return false;};
  crate::filter::player_filter_matches_game(&self.player,copy.player,ctx.game,&ctx.filter_ctx)
 }
 fn display(&self)->String{"Would select a source definition for a spell copy or a cast copy".into()}
}
