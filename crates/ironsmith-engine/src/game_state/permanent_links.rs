//! Directed named links retain exact battlefield object identities and the
//! controller who established them. They are not counters or copied values.
use super::*;
#[derive(Debug,Clone,PartialEq,Eq)]
pub(crate) struct DirectedPermanentLink { pub target:ObjectId, pub controller:PlayerId, pub require_creatures:bool }
impl GameState {
 fn directed_permanent_link_remains(&self,source:ObjectId,link:&DirectedPermanentLink)->bool{
  let Some(left)=self.object(source) else{return false};let Some(right)=self.object(link.target) else{return false};
  left.zone==Zone::Battlefield && right.zone==Zone::Battlefield
   && self.controller_of(left)==link.controller && self.controller_of(right)==link.controller
   && (!link.require_creatures || ((self.is_phased_out(source)||self.current_is_creature(source)) && (self.is_phased_out(link.target)||self.current_is_creature(link.target))))
 }
 pub fn linked_permanent(&self,source:ObjectId,relation:&str)->Option<ObjectId>{
  let link=self.combat_transients.directed_permanent_links.get(&(source,relation.to_ascii_lowercase()))?;
  self.directed_permanent_link_remains(source,link).then_some(link.target)
 }
 pub(crate) fn linked_permanent_for_grant(&self,source:ObjectId,relation:&str)->Option<ObjectId>{
  let link=self.combat_transients.directed_permanent_links.get(&(source,relation.to_ascii_lowercase()))?;
  let left=self.object(source)?;let right=self.object(link.target)?;
  (left.zone==Zone::Battlefield && right.zone==Zone::Battlefield && self.controller_of(left)==link.controller && self.controller_of(right)==link.controller).then_some(link.target)
 }
 pub fn set_directed_permanent_link(&mut self,source:ObjectId,target:ObjectId,relation:&str,controller:PlayerId,require_creatures:bool)->bool{
  if self.is_phased_out(source)||self.is_phased_out(target){return false}
  let link=DirectedPermanentLink{target,controller,require_creatures};
  if relation.is_empty() || !relation.chars().all(|ch|ch.is_ascii_alphabetic()||matches!(ch,' '|'-'|'\'')) || !self.directed_permanent_link_remains(source,&link){return false}
  let key=(source,relation.to_ascii_lowercase());
  let old=self.combat_transients_mut().directed_permanent_links.insert(key,link);
  self.mark_object_changed_and_continuous_state_dirty(source);self.mark_object_changed_and_continuous_state_dirty(target);
  if let Some(old)=old {self.mark_object_changed_and_continuous_state_dirty(old.target);}
  true
 }
 pub(crate) fn clear_directed_permanent_links_involving(&mut self,object:ObjectId){
  let keys=self.combat_transients.directed_permanent_links.iter().filter(|((source,_),link)|*source==object||link.target==object).map(|(key,link)|(key.clone(),link.target)).collect::<Vec<_>>();
  for (key,target) in keys {self.combat_transients_mut().directed_permanent_links.remove(&key);self.mark_object_changed_and_continuous_state_dirty(key.0);self.mark_object_changed_and_continuous_state_dirty(target);}
 }
 pub(crate) fn reconcile_directed_permanent_links(&mut self)->bool{
  let keys=self.combat_transients.directed_permanent_links.iter().filter(|((source,_),link)|!self.directed_permanent_link_remains(*source,link)).map(|(key,link)|(key.clone(),link.target)).collect::<Vec<_>>();
  let changed=!keys.is_empty();
  for (key,target) in keys {self.combat_transients_mut().directed_permanent_links.remove(&key);self.mark_object_changed_and_continuous_state_dirty(key.0);self.mark_object_changed_and_continuous_state_dirty(target);}
  changed
 }
}
