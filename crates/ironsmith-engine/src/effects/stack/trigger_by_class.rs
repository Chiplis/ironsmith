//! Select distinct triggered occurrences; normal trigger queues own their announcements.
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::effect::EffectOutcome;
use crate::game_state::GameState;
use crate::ability::AbilityKind;
use crate::triggers::{TriggeredAbilityEntry,TriggeredAbilitySourceKind,TriggerEvent};
pub use ironsmith_core::TriggerAbilitiesByClassEffect;
impl EffectExecutor for TriggerAbilitiesByClassEffect{
 fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{self.source_selector.as_ref()}
 fn own_preflight_object_specs(&self)->Vec<crate::target::ChooseSpec>{vec![self.source_selector.clone().unwrap_or_else(||crate::target::ChooseSpec::All(self.sources.clone()))]}
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
  crate::effects::composition::execute_transaction(game,ctx,||EffectOutcome::count(0),|game,ctx|{
   if self.minimum>self.maximum {return Err(ExecutionError::InvalidTarget);}
   let sources=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.source_selector.clone().unwrap_or_else(||crate::target::ChooseSpec::All(self.sources.clone())))?;
   if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0));}
   let chooser=if let Some(filter)=&self.chooser {crate::effects::helpers::resolve_player_filter_as_chooser(game,filter,ctx)?} else {ctx.controller};
   let mut entries=Vec::new();let mut options=Vec::new();
   for source in sources{
    if game.is_phased_out(source){continue;}
    let object=game.object(source).ok_or(ExecutionError::ObjectNotFound(source))?;
    let snapshot=crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)?;
    let Some(characteristics)=game.current_characteristics(source)else{continue;};
    for (index,definition) in characteristics.abilities.iter().enumerate(){
     // The explicit instruction invokes the source-owned program in the
     // declared source domain; it is not an automatic functional-zone event.

     let AbilityKind::Triggered(ability)=&definition.kind else{continue;};
     let Some(marker)=ability.trigger.downcast_ref::<crate::triggers::special::CustomTrigger>()else{continue;};
     if !marker.id.strip_prefix("externally-triggered:").is_some_and(|class|class.eq_ignore_ascii_case(&self.class)){continue;}
     let identity=crate::triggers::compute_trigger_identity(ability);
     // This describes the selected occurrence's invocation, not a fabricated
     // attack, death or other game action. Only normal queue publication emits
     // AbilityTriggeredEvent; this context event is never placed in history.
     let event=TriggerEvent::new(crate::events::AbilityTriggeredEvent::new(source,snapshot.stable_id,snapshot.controller,identity).with_source_snapshot(Some(snapshot.clone())),ctx.provenance).with_direct_trigger_invocation();
     let origin=snapshot.ability_origins.as_ref().and_then(|origins|origins.get(index));
     let entry=TriggeredAbilityEntry{source,controller:snapshot.controller,x_value:object.x_value,event_value_amount:None,ability:ability.clone(),triggering_event:event,source_stable_id:snapshot.stable_id,source_name:snapshot.name.clone(),source_snapshot:Some(snapshot.clone()),tagged_objects:Default::default(),source_kind:TriggeredAbilitySourceKind::Object,trigger_identity:identity,
      linked_exile_owner:crate::linked_exile::LinkedExileOwner::capture(source,ability.effects.linked_exile_pair,origin),source_number_owner:crate::linked_exile::LinkedExileOwner::capture(source,ability.effects.source_number_pair,origin)};
     options.push(crate::decisions::SelectableOption::new(entries.len(),format!("{}: {}",snapshot.name,ability.trigger.display())).with_object(source));entries.push(entry);
    }
   }
   // Snapshot applicable continuous instructions before asking any choices.
   let mut additional=0usize;let mut multipliers=Vec::new();
   for &source in &game.battlefield {
    if game.is_phased_out(source){continue;}
    let Some(object)=game.object(source)else{continue;};
    if game.current_controller(source)!=Some(ctx.controller){continue;}
    let Some(characteristics)=game.current_characteristics(source)else{continue;};
    for (index,definition) in characteristics.abilities.iter().enumerate(){
     if !definition.functional_zones.contains(&object.zone){continue;}
     let AbilityKind::Static(ability)=&definition.kind else{continue;};
     if !ability.is_active(game,source){continue;}
     match ability.keyword_marker_mechanic(){
      Some(ironsmith_core::KeywordMarkerMechanic::ClassTriggerChoiceAdjustment{action,class,additional:amount})
       if class.eq_ignore_ascii_case(&self.class) && self.action_name.as_ref().is_some_and(|name|name.eq_ignore_ascii_case(&action)) => {additional=additional.saturating_add(amount as usize);}
      Some(ironsmith_core::KeywordMarkerMechanic::ClassTriggerMultiplication{class,multiplier,once_each_turn}) if class.eq_ignore_ascii_case(&self.class) => {
       multipliers.push((source,index,multiplier,once_each_turn));
      }
      _=>{}
     }
    }
   }
   let maximum=(self.maximum as usize).saturating_add(additional).min(entries.len());if maximum==0{return Ok(EffectOutcome::count(0));}
   let request=crate::decisions::context::SelectOptionsContext::new(chooser,Some(ctx.source),format!("Choose {}{maximum} {} triggered abilities",if self.minimum==0 {"up to "} else {""},self.class),options,(self.minimum as usize).min(maximum),maximum);
   let chosen=ctx.decision_maker.decide_options(game,&request);if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0));}
   if chosen.len()>maximum||chosen.len()<(self.minimum as usize).min(maximum){return Err(ExecutionError::InvalidTarget);}
   let mut seen=std::collections::HashSet::new();let mut selected=Vec::new();
   for index in chosen{if !seen.insert(index){return Err(ExecutionError::InvalidTarget);}selected.push(entries.get(index).cloned().ok_or(ExecutionError::InvalidTarget)?);}
   let mut expanded=Vec::new();
   for entry in selected {
    let mut copies=1usize;
    for &(source,index,multiplier,once_each_turn) in &multipliers {
     if entry.controller!=ctx.controller || !game.object(entry.source).is_some_and(|object|object.zone==crate::zone::Zone::Battlefield){continue;}
     let receipt=format!("class-trigger-multiplier:{}:{}",source.0,index);
     if once_each_turn && game.named_turn_counter(&receipt)>0{continue;}
     let prompt=crate::decisions::context::BooleanContext::new(ctx.controller,Some(source),format!("Have this {} ability trigger {multiplier} times instead?",self.class));
     let accept=ctx.decision_maker.decide_boolean(game,&prompt);
     if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0));}
     if accept {
      copies=copies.checked_mul(multiplier as usize).filter(|n|*n<=10000).ok_or(ExecutionError::ResourceLimitExceeded{resource:"triggered occurrences",requested:(copies as u128)*(multiplier as u128),maximum:10000})?;
      if once_each_turn{game.increment_named_turn_counter(receipt);}
     }
    }
    expanded.extend(std::iter::repeat_n(entry,copies));
   }
   let selected=expanded;
   let count=i32::try_from(selected.len()).map_err(|_|ExecutionError::ResourceLimitExceeded{resource:"selected triggered occurrences",requested:selected.len() as u128,maximum:i32::MAX as u128})?;
   crate::game_loop::defer_forced_trigger_occurrences(game,selected);Ok(EffectOutcome::count(count))
  })
 }
}

#[cfg(test)]
mod invocation_tests {
 use super::*;
 use crate::{Ability,CardId,Effect,PlayerId,Zone};
 use crate::cards::builders::CardDefinitionBuilder;
 use crate::decision::{DecisionMaker,SelectFirstDecisionMaker};
 use crate::target::{ObjectFilter,PlayerFilter};
 const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
 fn source()->crate::CardDefinition {
  CardDefinitionBuilder::new(CardId::new(),"Outside-zone program").card_types(vec![crate::CardType::Creature])
   .power_toughness(crate::card::PowerToughness::fixed(2,2))
   .with_ability(Ability::triggered(crate::triggers::Trigger::custom("externally-triggered:echo bell","An externally invoked program".into()),vec![Effect::gain_life(2)])).build()
 }
 struct Choose {accept:bool,players:Vec<PlayerId>}
 impl DecisionMaker for Choose {
  fn decide_options(&mut self,g:&GameState,c:&crate::decisions::context::SelectOptionsContext)->Vec<usize> {
   if c.description.contains("echo bell triggered abilities") {
    self.players.push(c.player);return if self.accept {vec![0]} else {Vec::new()};
   }
   SelectFirstDecisionMaker.decide_options(g,c)
  }
 }
 #[test]
 fn explicit_program_chooser_is_not_the_program_controller_and_no_trigger_cause_is_fabricated() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let card=game.create_object_from_definition(&source(),A,Zone::Hand);
  let effect=TriggerAbilitiesByClassEffect {class:"echo bell".into(),sources:ObjectFilter::default().you_control().in_zone(Zone::Hand),source_selector:None,maximum:1,minimum:1,chooser:Some(PlayerFilter::Specific(B)),action_name:None};
  let mut dm=Choose{accept:true,players:Vec::new()};
  let outcome=effect.execute(&mut game,&mut ExecutionContext::new(card,A,&mut dm)).unwrap();
  assert!(outcome.status.is_success());assert_eq!(dm.players,vec![B]);
  let mut queue=crate::triggers::TriggerQueue::new();
  crate::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  assert_eq!(queue.entries.len(),1);assert_eq!(queue.entries[0].controller,A);
  let notifications=queue.clone().take_ability_triggered_events();assert_eq!(notifications.len(),1);
  let notification=notifications[0].0.downcast::<crate::events::AbilityTriggeredEvent>().unwrap();
  assert!(notification.cause_kind.is_none(),"an explicit invocation is not caused by another ability triggering");
  assert!(queue.entries[0].triggering_event.is_direct_trigger_invocation());
  crate::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  crate::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(A).unwrap().life,22);assert_eq!(game.player(B).unwrap().life,20);
 }
 #[test]
 fn a_mandatory_existing_program_cannot_be_declined_but_zero_minimum_remains_optional() {
  for minimum in [0,1] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let card=game.create_object_from_definition(&source(),A,Zone::Battlefield);
   let effect=TriggerAbilitiesByClassEffect {class:"echo bell".into(),sources:ObjectFilter::default().you_control().in_zone(Zone::Battlefield),source_selector:None,maximum:1,minimum,chooser:None,action_name:None};
   let mut dm=Choose{accept:false,players:Vec::new()};
   let result=effect.execute(&mut game,&mut ExecutionContext::new(card,A,&mut dm));
   if minimum==0 {assert!(result.is_ok());} else {assert!(matches!(result,Err(ExecutionError::InvalidTarget)));}
   assert!(game.effect_store.pending_trigger_entries.is_empty());assert_eq!(game.player(A).unwrap().life,20);
  }
 }
}
