//! Choose one available compiler-authored optional group, then register a
//! layer3 deletion. All executable alternatives are precompiled typed data.
use crate::continuous::{ContinuousEffect,Modification};
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::RemoveOptionalTextFragmentEffect;
impl EffectExecutor for RemoveOptionalTextFragmentEffect {
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
  let objects=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.target)?;
  let Some(target)=objects.first().copied()else{return Ok(EffectOutcome::count(0))};
  let Some(plan)=crate::continuous::optional_text_fragments::current_plan(game,target)?else{return Ok(EffectOutcome::count(0))};
  let choices=plan.labels.iter().enumerate().filter(|(index,_)|plan.current_removed&(1u64<<*index)==0).map(|(index,label)|(index,label.clone())).collect::<Vec<_>>();
  if choices.is_empty(){return Ok(EffectOutcome::count(0))}
  use crate::effects::composition::scoped_choice::{select_typed_option,SingleChoicePolicy};
  let Some(index)=select_typed_option(game,ctx,ctx.controller,"Choose a bracketed text fragment to remove",&choices,SingleChoicePolicy::Strict)?else{return Ok(EffectOutcome::resolved())};
  let ids=super::register_resolved_continuous_batch(game,[ContinuousEffect::from_resolution(ctx.source,ctx.controller,vec![target],Modification::RemoveOptionalTextFragments(1u64<<index)).until(self.until.clone())]);
  ctx.created_continuous_effects.extend(ids);
  Ok(EffectOutcome::with_objects(vec![target]))
 }
 fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{self.target.is_target().then_some(&self.target)}
 fn get_target_count(&self)->Option<crate::effect::ChoiceCount>{Some(self.target.count())}
 fn decision_related_object_specs(&self)->Vec<crate::target::ChooseSpec>{vec![self.target.clone()]}
}
