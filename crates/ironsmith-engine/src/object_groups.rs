//! Persistent object associations are independent of copiable characteristics.
use std::collections::{BTreeMap,BTreeSet};
use crate::{GameState,ObjectId,Zone,CardType};
#[derive(Debug,Clone,Default)]
pub struct ObjectGroupStore {next:u64,groups:BTreeMap<u64,ObjectGroup>,membership:BTreeMap<ObjectId,u64>,owned:BTreeMap<ObjectId,u64>}
#[derive(Debug,Clone,Default)]
struct ObjectGroup {owner:Option<ObjectId>,members:BTreeSet<ObjectId>,combat_together:bool}
#[derive(Debug,Clone)]
pub struct ObjectGroupRequest {pub owner:Option<ObjectId>,pub subjects:Vec<ObjectId>,pub combat_together:bool}
impl ObjectGroupStore {
 fn ensure_group(&mut self,owner:Option<ObjectId>,combat_together:bool)->u64{
  let existing=owner.and_then(|owner|self.membership.get(&owner).or_else(||self.owned.get(&owner)).copied());
  let id=existing.unwrap_or_else(||{let id=self.next;self.next=self.next.checked_add(1).expect("object group identifier exhausted");self.groups.insert(id,ObjectGroup{owner,members:BTreeSet::new(),combat_together});if let Some(owner)=owner{self.owned.insert(owner,id);}id});
  self.groups.get_mut(&id).unwrap().combat_together|=combat_together;id
 }
 fn assign(&mut self,subject:ObjectId,id:u64){
  if self.groups.get(&id).is_some_and(|group|group.owner==Some(subject)){return}
  if let Some(previous)=self.membership.insert(subject,id){if let Some(group)=self.groups.get_mut(&previous){group.members.remove(&subject);}}
  self.groups.get_mut(&id).unwrap().members.insert(subject);
 }
 pub fn link(&mut self,owner:Option<ObjectId>,subjects:&[ObjectId],combat_together:bool)->u64{let id=self.ensure_group(owner,combat_together);for subject in subjects{self.assign(*subject,id)}id}
 pub fn members_owned_by(&self,owner:ObjectId)->Vec<ObjectId>{self.owned.get(&owner).and_then(|id|self.groups.get(id)).map(|group|group.members.iter().copied().collect()).unwrap_or_default()}
 pub fn members_with(&self,object:ObjectId)->Vec<ObjectId>{self.membership.get(&object).or_else(||self.owned.get(&object)).and_then(|id|self.groups.get(id)).map(|group|group.members.iter().copied().collect()).unwrap_or_default()}
 fn coordinated_actors(&self)->Vec<(Vec<ObjectId>,Vec<ObjectId>)>{self.groups.values().filter(|group|group.combat_together).map(|group|{let mut actors=group.members.clone();if let Some(owner)=group.owner{actors.insert(owner);}(actors.into_iter().collect(),group.members.iter().copied().collect())}).collect()}
}
impl GameState {
 pub fn link_object_group(&mut self,owner:Option<ObjectId>,subjects:&[ObjectId],combat_together:bool)->u64{let id=self.object_groups.link(owner,subjects,combat_together);self.mark_continuous_state_dirty();id}
 /// Resolve conflicting simultaneous associations before committing any membership.
 pub fn link_object_groups_with_dm(&mut self,requests:&[ObjectGroupRequest],_source:ObjectId,dm:&mut (impl crate::decision::DecisionMaker+?Sized))->Result<bool,crate::effects::ExecutionError>{
  let mut staged=self.object_groups.clone();let mut candidates:BTreeMap<ObjectId,BTreeSet<u64>>=BTreeMap::new();
  for request in requests{let id=staged.ensure_group(request.owner,request.combat_together);for subject in &request.subjects{if staged.groups[&id].owner!=Some(*subject){candidates.entry(*subject).or_default().insert(id);}}}
  let mut selected=Vec::new();let order=self.team_apnap_player_order();
  let mut candidates=candidates.into_iter().collect::<Vec<_>>();
  candidates.sort_by_key(|(subject,_)|self.object(*subject).map(|object|(order.iter().position(|player|*player==self.controller_of(object)).unwrap_or(usize::MAX),object.stable_id)));
  for (subject,groups) in candidates{
   let id=if groups.len()==1{*groups.first().unwrap()}else{
    let object=self.object(subject).ok_or_else(||crate::effects::ExecutionError::IncompleteEvidence("group chooser object is absent".into()))?;
    let controller=self.controller_of(object);let options=groups.iter().map(|id|{
     let label=staged.groups[id].owner.and_then(|owner|self.object(owner)).map(|owner|format!("Group owned by {}",owner.name)).unwrap_or_else(||format!("Group {}",id+1));(label,*id)
    }).collect::<Vec<_>>();
    let choice=crate::decisions::ask_choose_one(self,dm,controller,subject,&options);
    if dm.awaiting_choice(){return Ok(false)}
    let choice=choice.ok_or_else(||crate::effects::ExecutionError::IncompleteEvidence("conflicting group insertion requires its controller's choice".into()))?;
    if !groups.contains(&choice){return Err(crate::effects::ExecutionError::IncompleteEvidence("chosen group was not offered".into()))}choice
   };selected.push((subject,id));
  }
  for (subject,id) in selected{staged.assign(subject,id)}self.object_groups=staged;self.mark_continuous_state_dirty();Ok(true)
 }
 pub fn object_group_members_owned_by(&self,owner:ObjectId)->Vec<ObjectId>{self.object_groups.members_owned_by(owner)}
 pub fn object_group_members_with(&self,object:ObjectId)->Vec<ObjectId>{self.object_groups.members_with(object)}
 pub(crate) fn coordinated_combat_groups(&self)->Vec<(Vec<ObjectId>,Vec<ObjectId>)>{
  let live=|id:&ObjectId|self.object(*id).is_some_and(|object|object.zone==Zone::Battlefield&&!self.is_phased_out(*id)&&self.object_has_card_type(*id,CardType::Creature));
  self.object_groups.coordinated_actors().into_iter().map(|(actors,members)|(actors.into_iter().filter(live).collect(),members.into_iter().filter(live).collect())).collect()
 }
}
