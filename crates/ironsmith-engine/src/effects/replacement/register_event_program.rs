//! Timed event-local programs reuse ordinary event matching and Instead execution.
use crate::effect::{Effect,EffectOutcome};
use crate::effects::{ApplyReplacementEffect,EffectExecutor,ExecutionContext,ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityKind;
pub type RegisterEventReplacementEffect=ironsmith_core::RegisterEventReplacementEffect<Effect>;
impl EffectExecutor for RegisterEventReplacementEffect {
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
  let targets=match &self.target {
   Some(spec)=>crate::effects::helpers::resolve_objects_for_effect(game,ctx,spec)?.into_iter().map(Some).collect::<Vec<_>>(),
   None=>vec![None],
  };
  let mut count=0;
  for target in targets {
   let event=self.event.clone();
   let effect=if let ironsmith_core::ReplacedEventSpec::ZoneChange{mut object,from,to}=event {
    if let Some(id)=target {object.specific=Some(id);}
    let mut matcher=crate::events::zones::matchers::WouldChangeZoneMatcher::new(object,from,to)
      .with_frozen_tagged_objects(ctx.tagged_objects.clone());
    matcher.require_resolving_object_match=self.resolving_object_only;
    crate::replacement::ReplacementEffect::with_matcher(ctx.source,ctx.controller,matcher,
      crate::replacement::ReplacementAction::Instead(self.replacement_effects.clone()))
   }else{
    if target.is_some()||self.resolving_object_only{return Err(ExecutionError::Impossible("bound resolution replacement requires a zone event".into()));}
    let owner=crate::static_abilities::EventReplacementWithEffects::new(event,self.replacement_effects.clone(),self.display.clone());
    owner.generate_replacement_effect(ctx.source,ctx.controller).ok_or_else(||ExecutionError::Impossible("event replacement has no matcher".into()))?
   };
   ApplyReplacementEffect{effect,mode:self.mode}.execute_child(game,ctx)?;count+=1;
  }
  Ok(EffectOutcome::count(count))
 }
 fn visit_child_effects(&self,visit:&mut dyn FnMut(&Effect)){for child in &self.replacement_effects{visit(child);}}
 fn primary_execution_category(&self)->crate::effects::EffectExecutionCategory{crate::effects::EffectExecutionCategory::ReplacementRegistration}
}
