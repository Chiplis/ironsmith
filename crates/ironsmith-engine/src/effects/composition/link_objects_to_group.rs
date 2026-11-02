//! Link ordinary resolved objects without granting copiable abilities.
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::effect::EffectOutcome;
use crate::{GameState,Zone};
pub use ironsmith_core::LinkObjectsToGroupEffect;
impl EffectExecutor for LinkObjectsToGroupEffect {
 fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{self.groups.iter().find_map(|group|if group.subjects.is_target(){Some(&group.subjects)}else{group.owner.as_ref().filter(|owner|owner.is_target())})}
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
  let mut requests=Vec::new();let mut subjects_all=std::collections::BTreeSet::new();
  for group in &self.groups {
   let subjects=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&group.subjects)?.into_iter().filter(|id|game.object(*id).is_some_and(|object|object.zone==Zone::Battlefield&&!game.is_phased_out(*id))).collect::<Vec<_>>();
   let owner=match group.owner.as_ref().map(|owner|owner.base()){
    None=>None,
    Some(crate::target::ChooseSpec::Source)=>Some(ctx.source),
    Some(crate::target::ChooseSpec::SpecificObject(id))=>Some(*id),
    Some(crate::target::ChooseSpec::Tagged(tag))=>ctx.get_tagged_all(tag).and_then(|objects|objects.first()).map(|snapshot|snapshot.object_id),
    Some(_)=>{let objects=crate::effects::helpers::resolve_objects_for_effect(game,ctx,group.owner.as_ref().unwrap())?;if objects.len()!=1{return Err(ExecutionError::IncompleteEvidence("group owner requires one exact object".into()))}Some(objects[0])},
   };
   if group.owner.is_some()&&owner.is_none(){return Err(ExecutionError::IncompleteEvidence("group owner reference is unbound".into()))}
   subjects_all.extend(subjects.iter().copied());requests.push(crate::object_groups::ObjectGroupRequest{owner,subjects,combat_together:group.combat_together});
  }
  if !game.link_object_groups_with_dm(&requests,ctx.source,&mut ctx.decision_maker)?{return Ok(EffectOutcome::count(0))}
  Ok(EffectOutcome::with_objects(subjects_all.into_iter().collect()))
 }
}
