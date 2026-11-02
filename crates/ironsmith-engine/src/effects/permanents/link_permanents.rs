//! One directed relationship; optionality, object selection and triggers are
//! composed by their ordinary effect families.
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::effect::EffectOutcome;
use crate::game_state::GameState;
pub use ironsmith_core::LinkPermanentsEffect;
impl EffectExecutor for LinkPermanentsEffect {
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
  let source=match crate::effects::helpers::resolve_single_object_for_effect(game,ctx,&self.source) {Ok(object)=>object,Err(ExecutionError::InvalidTarget)=>return Ok(EffectOutcome::count(0)),Err(error)=>return Err(error)};
  if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0))}
  let target=match crate::effects::helpers::resolve_single_object_for_effect(game,ctx,&self.target) {Ok(object)=>object,Err(ExecutionError::InvalidTarget)=>return Ok(EffectOutcome::count(0)),Err(error)=>return Err(error)};
  if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0))}
  let changed=game.set_directed_permanent_link(source,target,&self.relation,ctx.controller,self.require_creatures);
  Ok(EffectOutcome::count(i64::from(changed)))
 }
 fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{self.target.is_target().then_some(&self.target).or_else(||self.source.is_target().then_some(&self.source))}
 fn own_preflight_object_specs(&self)->Vec<crate::target::ChooseSpec>{vec![self.source.clone(),self.target.clone()]}
 fn decision_related_object_specs(&self)->Vec<crate::target::ChooseSpec>{vec![self.source.clone(),self.target.clone()]}
}
