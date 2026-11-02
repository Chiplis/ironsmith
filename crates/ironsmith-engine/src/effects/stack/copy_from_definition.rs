//! Copies an explicitly supplied definition; there is no catalog fallback.
use crate::effect::{EffectOutcome,PriorEffectAction};
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::{GameState,ObjectId,PlayerId,Zone};
pub type CopyFromDefinitionEffect=ironsmith_core::CopyFromDefinitionEffect<crate::cards::CardDefinition>;
/// A fresh definition has no original spell's announced choices. Until a normal
/// new-copy announcement supplies them, reject definitions requiring choices.
pub(crate) fn definition_copy_source(game:&mut GameState,definition:&crate::cards::CardDefinition,player:PlayerId)->Result<(crate::object::Object,crate::game_state::StackEntry),ExecutionError>{
 if let Some(program)=&definition.spell_effect {
  fn has_modes(effect:&crate::effect::Effect)->bool {
   if effect.downcast_ref::<crate::effects::ChooseModeEffect>().is_some(){return true;}
   let mut result=false;effect.0.visit_child_effects(&mut |child|{result|=has_modes(child);});result
  }
  if !crate::game_loop::extract_target_requirements_from_program_with_modes(game,program,player,None,None).is_empty()
   || program.iter().any(has_modes) {
    return Err(ExecutionError::IncompleteEvidence("a fresh copy-source definition requires its own target or mode announcement".into()));
  }
 }
 game.register_ability_counter_definitions(definition).map_err(ExecutionError::IncompleteEvidence)?;
 game.register_card_subtypes(&definition.card);
 let virtual_id=game.new_object_id();
 let object=crate::object::Object::from_card_definition(virtual_id,definition,player,Zone::Stack);
 let entry=crate::game_state::StackEntry::new(virtual_id,player);
 Ok((object,entry))
}
impl EffectExecutor for CopyFromDefinitionEffect {
 fn clone_box(&self)->Box<dyn EffectExecutor>{Box::new(self.clone())}
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
  crate::effects::composition::execute_transaction(game,ctx,||EffectOutcome::count(0),|game,ctx|{
   let player=crate::effects::helpers::resolve_player_filter(game,&self.player,ctx)?;
   let count=crate::effects::helpers::resolve_nonnegative_u32(game,&self.count,ctx)?;
   let definition=ctx.bind_definition_scope_definition(self.definition.clone())?;
   let (source,mut entry)=definition_copy_source(game,&definition,player)?;entry.provenance=ctx.provenance;
   let mut ids=Vec::<ObjectId>::new();
   for _ in 0..count {
    if let Some(id)=super::copy_spell::create_stack_copy_from_object(game,&source,source.id,&entry,player,&[],|_|{},None)?{
     ids.push(id);super::copy_spell::queue_stack_copy_completion(game,ctx,id);
    }
   }
   let snapshots=ids.iter().filter_map(|id|crate::snapshot::ObjectSnapshot::from_object_id(game,*id)).collect();
   Ok(EffectOutcome::with_objects(ids).with_action_objects(PriorEffectAction::Copied,Some(player),snapshots))
  })
 }
}
