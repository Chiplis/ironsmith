//! Source-authored event choices among ordinary externally invocable programs.
use crate::ability::AbilityKind;
use crate::effects::ExecutionError;
use crate::filter::{ObjectFilterExt as _,ObjectFilter};
use crate::game_state::GameState;
use crate::ids::PlayerId;
#[cfg(test)]
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;
use super::{TriggerContext,TriggerEvent,TriggeredAbilityEntry,TriggeredAbilitySourceKind};

#[derive(Debug,Clone)]
pub(crate) struct PendingTriggerClassChoice {
 pub event:TriggerEvent,
 pub class:String,
 pub official_keyword:bool,
 pub controller:PlayerId,
 pub maximum:u32,
 pub optional:bool,
 pub automatic:bool,
 pub caused_by_event:bool,
 pub instance:u32,
 /// One option per original ability, including that ability's captured copies.
 pub candidates:Vec<Vec<TriggeredAbilityEntry>>,
}

/// Freeze ordinary retained entries from exact source snapshots. No program
/// executes and no AbilityTriggered event exists until a choice is accepted.
pub(crate) fn entries_from_snapshot(game:&GameState,snapshot:&ObjectSnapshot,class:&str,official_keyword:bool,event:&TriggerEvent)
 ->Vec<TriggeredAbilityEntry> {
 let mut entries=Vec::new();
 for (index,definition) in snapshot.abilities.iter().enumerate() {
  if !definition.functional_zones.contains(&snapshot.zone) {continue;}
  let AbilityKind::Triggered(ability)=&definition.kind else {continue;};
  let eligible=if official_keyword {
   ability.effects.keyword.as_deref().is_some_and(|keyword|keyword.eq_ignore_ascii_case(class))
  } else {
   ability.trigger.downcast_ref::<super::special::CustomTrigger>().is_some_and(|marker|
    marker.id.strip_prefix("externally-triggered:").is_some_and(|value|value.eq_ignore_ascii_case(class)))
  };
  if !eligible {continue;}
  let origin=snapshot.ability_origins.as_ref().and_then(|origins|origins.get(index));
  let identity=super::compute_trigger_identity(ability);
  entries.push(TriggeredAbilityEntry {
   source:snapshot.object_id,controller:snapshot.controller,x_value:snapshot.x_value,event_value_amount:None,
   ability:ability.clone(),triggering_event:event.clone(),source_stable_id:snapshot.stable_id,
   source_name:snapshot.name.clone(),source_snapshot:Some(snapshot.clone()),tagged_objects:Default::default(),
   source_kind:TriggeredAbilitySourceKind::Object,trigger_identity:identity,
   linked_exile_owner:crate::linked_exile::LinkedExileOwner::capture(snapshot.object_id,ability.effects.linked_exile_pair,origin),
   source_number_owner:crate::linked_exile::LinkedExileOwner::capture(snapshot.object_id,ability.effects.source_number_pair,origin),
  });
 }
 let _=game;
 entries
}

fn live_sources(game:&GameState,event:&TriggerEvent)->Result<Vec<ObjectSnapshot>,ExecutionError> {
 let mut snapshots=Vec::new();
 for object in game.objects_in_deterministic_order() {
  if object.zone!=Zone::Battlefield {continue;}
  // Pre-event grants/controller belong to this exact still-live incarnation.
  // No unrelated departed lookback or later stable-ID incarnation is admitted.
  if let Some(snapshot)=event.lookback_source_snapshots().iter().find(|snapshot|
   snapshot.object_id==object.id&&snapshot.zone==Zone::Battlefield) {snapshots.push(snapshot.clone());}
  else if !game.is_phased_out(object.id) {
   snapshots.push(ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)?);
  }
 }
 Ok(snapshots)
}

/// Batched physical departures retain one decision per event subject. Project
/// only the immutable event view; no move, history or observer is re-published.
fn selection_events(event:&TriggerEvent)->Vec<TriggerEvent> {
 let Some(zone)=event.downcast::<crate::events::ZoneChangeEvent>() else {return vec![event.clone()];};
 if zone.objects.len()<2 {return vec![event.clone()];}
 zone.snapshots().iter().map(|snapshot| {
  let mut part=zone.clone();part.objects=vec![snapshot.object_id];part.snapshot=Some(snapshot.clone());part.snapshots=vec![snapshot.clone()];
  // Departure choices read origin LKI, never an unrelated first arrival.
  part.destination_snapshots.retain(|arrival|arrival.stable_id==snapshot.stable_id);
  part.result_objects=part.destination_snapshots.iter().map(|arrival|arrival.object_id).collect();
  part.destination_emerge_sacrifices.retain(|id,_|part.result_objects.contains(id));
  event.with_inner_event(part)
 }).collect()
}

pub(crate) fn has_event_choice_rules<'a>(game:&GameState,events:impl IntoIterator<Item=&'a TriggerEvent>)
 ->Result<bool,ExecutionError> {
 let has=|snapshot:&ObjectSnapshot|snapshot.abilities.iter().any(|definition|matches!(&definition.kind,
  AbilityKind::Static(rule) if matches!(rule.compiled_model().map(|model|&model.payload),
   Some(ironsmith_core::StaticAbilityPayload::TriggerClassEventChoice {..}))));
 for event in events {
  if event.snapshot().is_some_and(&has)||event.lookback_source_snapshots().iter().any(&has)
   ||event.downcast::<crate::events::ZoneChangeEvent>().is_some_and(|zone|zone.snapshots().iter().any(&has)) {return Ok(true);}
 }
 for object in game.objects_in_deterministic_order() {
  if object.zone!=Zone::Battlefield||game.is_phased_out(object.id){continue;}
  let Some(chars)=game.try_current_characteristics(object.id).map_err(ExecutionError::ContinuousDiscovery)? else {continue;};
  if chars.abilities.iter().any(|definition|matches!(&definition.kind,AbilityKind::Static(rule)
   if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::TriggerClassEventChoice {..})))) {return Ok(true);}
 }
 Ok(false)
}

pub(crate) fn capture_event_choices(game:&GameState,event:&TriggerEvent)->Result<Vec<PendingTriggerClassChoice>,ExecutionError> {
 if game.action_observations_suppressed(){return Ok(Vec::new());}
 // Ordinary games with no authored rule need no source snapshot packet.
 let mut has_rule=false;
 for object in game.objects_in_deterministic_order() {
  if object.zone!=Zone::Battlefield||game.is_phased_out(object.id){continue;}
  let characteristics=game.try_current_characteristics(object.id).map_err(ExecutionError::ContinuousDiscovery)?;
  if characteristics.is_some_and(|characteristics|characteristics.abilities.iter().any(|definition|
   matches!(&definition.kind,AbilityKind::Static(rule) if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::TriggerClassEventChoice {..}))))) {
   has_rule=true;break;
  }
 }
 if !has_rule {
  has_rule=event.downcast::<crate::events::ZoneChangeEvent>().is_some_and(|zone|zone.snapshots().iter().any(|snapshot|
   snapshot.abilities.iter().any(|definition|matches!(&definition.kind,AbilityKind::Static(rule) if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::TriggerClassEventChoice {..}))))));
 }
 if !has_rule {
  has_rule=event.lookback_source_snapshots().iter().filter(|snapshot|snapshot.zone==Zone::Battlefield
   && game.object(snapshot.object_id).is_some_and(|object|object.zone==Zone::Battlefield))
   .any(|snapshot|snapshot.abilities.iter().any(|definition|matches!(&definition.kind,AbilityKind::Static(rule)
    if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::TriggerClassEventChoice {..})))));
 }
 if !has_rule {return Ok(Vec::new());}
 let live=live_sources(game,event)?;
 // Only this original physical batch supplies peer LKI. Other historical
 // lookbacks cannot make a departed unrelated permanent eligible again.
 let batch_origins=event.downcast::<crate::events::ZoneChangeEvent>()
  .filter(|zone|zone.from==Zone::Battlefield).map(|zone|zone.snapshots().to_vec()).unwrap_or_default();
 let mut pending:Vec<PendingTriggerClassChoice>=Vec::new();
 for event in selection_events(event) {
  let mut providers=live.clone();
  for source in &batch_origins {if !providers.iter().any(|provider|provider.object_id==source.object_id) {providers.push(source.clone());}}
  struct Rule {controller:PlayerId,class:String,official_keyword:bool,sources:Option<ObjectFilter>,include_subject:bool,destination:bool,
   maximum:u32,optional:bool,automatic:bool,caused_by_event:bool,occurrences:u32,provider:ObjectSnapshot}
  let mut rules=Vec::new();
  for provider in &providers {
   for definition in provider.abilities.iter() {
    if !definition.functional_zones.contains(&provider.zone) {continue;}
    let AbilityKind::Static(rule)=&definition.kind else {continue;};
    let Some(ironsmith_core::StaticAbilityPayload::TriggerClassEventChoice {class,official_keyword,event:matcher,sources,include_event_source,destination_subject,maximum,optional,automatic,caused_by_event})=
     rule.compiled_model().map(|model|&model.payload) else {continue;};
    let mut context=TriggerContext::for_source(provider.object_id,provider.controller,game);
    context.filter_ctx.source_snapshot=Some(provider.clone());
    if !matcher.matches(&event,&context) {continue;}
    matcher.compiled_model().ok_or_else(||ExecutionError::IncompleteEvidence(
     "trigger-class event rule has no complete typed event model".into()))?;
    let occurrences=matcher.trigger_count_with_context(&event,&context);
    if occurrences>10000 {return Err(ExecutionError::ResourceLimitExceeded {resource:"event-trigger program choices",requested:u128::from(occurrences),maximum:10000});}
    rules.push(Rule {controller:provider.controller,class:class.clone(),official_keyword:*official_keyword,sources:sources.clone(),include_subject:*include_event_source,
     destination:*destination_subject,maximum:*maximum,optional:*optional,automatic:*automatic,caused_by_event:*caused_by_event,occurrences,provider:provider.clone()});
   }
  }
  for Rule {controller,class,official_keyword,sources:filter,include_subject:include_departed,destination:destination_subject,
   maximum,optional,automatic,caused_by_event,occurrences,provider:source} in rules {
   let mut context=game.filter_context_for(controller,Some(source.object_id));
   context.source_snapshot=Some(source.clone());
   let mut eligible=live.iter().filter(|snapshot|filter.as_ref().is_some_and(|filter|filter.matches_snapshot(snapshot,&context,game))).cloned().collect::<Vec<_>>();
   for snapshot in &batch_origins {
    if filter.as_ref().is_some_and(|filter|filter.matches_snapshot(snapshot,&context,game))
     && !eligible.iter().any(|old|old.object_id==snapshot.object_id) {eligible.push(snapshot.clone());}
   }
   if include_departed {
    // The source of the matching event is allowed by its immutable event-time
    // controller, independently of the current destination incarnation.
    let subjects=if let Some(zone)=event.downcast::<crate::events::ZoneChangeEvent>() {
     if destination_subject {
      if zone.destination_snapshots.is_empty() {return Err(ExecutionError::IncompleteEvidence("event-arrival program selection lacks exact completed destination snapshots".into()));}
      zone.destination_snapshots.clone()
     } else {zone.snapshots().to_vec()}
    }
     else if let Some(snapshot)=event.snapshot() {vec![snapshot.clone()]}
     else if let Some(object)=event.object_id().and_then(|id|game.object(id)) {
      vec![ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)?]
     } else {Vec::new()};
    for snapshot in &subjects {
     if !eligible.iter().any(|old|old.object_id==snapshot.object_id) {eligible.push(snapshot.clone());}
    }
   }
   let mut entries=eligible.iter().flat_map(|snapshot|entries_from_snapshot(game,snapshot,&class,official_keyword,&event)).collect::<Vec<_>>();
   if caused_by_event {super::check::retain_unsuppressed_trigger_occurrences(game,&mut entries);}
   else {for entry in &mut entries {entry.triggering_event=entry.triggering_event.clone().with_direct_trigger_invocation();}}
   let candidates=entries.into_iter().map(|entry|super::check::with_additional_trigger_copies(game,vec![entry]))
    .filter(|copies|!copies.is_empty()).collect::<Vec<_>>();
   if maximum>0&&!candidates.is_empty() {
    let same_candidates=|other:&Vec<Vec<TriggeredAbilityEntry>>|other.len()==candidates.len()
     && other.iter().zip(&candidates).all(|(left,right)|left.len()==right.len()
      && left.iter().zip(right).all(|(left,right)|left.source==right.source&&left.trigger_identity==right.trigger_identity));
    for instance in 0..occurrences {
     if !pending.iter().any(|prior|prior.event.ptr_eq(&event)&&prior.event.object_id()==event.object_id()&&prior.instance==instance
      && prior.class.eq_ignore_ascii_case(&class)&&prior.official_keyword==official_keyword&&prior.controller==controller&&prior.maximum==maximum
      && prior.optional==optional&&prior.automatic==automatic&&prior.caused_by_event==caused_by_event&&same_candidates(&prior.candidates)) {
      pending.push(PendingTriggerClassChoice{event:event.clone(),class:class.clone(),official_keyword,controller,maximum,optional,automatic,caused_by_event,instance,candidates:candidates.clone()});
     }
    }
   }
  }
 }
 Ok(pending)
}

#[cfg(test)]
mod tests {
 use super::*;
 use super::super::Trigger;
 use crate::{Ability,Effect,CardId};
 use crate::cards::builders::CardDefinitionBuilder;
 use crate::decision::{DecisionMaker,SelectFirstDecisionMaker};
 use crate::types::CardType;
 const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
 fn definition(name:&str,gain:i32,rule:bool,creature:bool)->crate::CardDefinition {
  let program=Ability::triggered(Trigger::custom("externally-triggered:echo bell","Externally triggered ability named echo bell".into()),vec![Effect::gain_life(gain)]);
  let mut builder=CardDefinitionBuilder::new(CardId::new(),name).card_types(vec![if creature {CardType::Creature} else {CardType::Artifact}])
   .power_toughness(crate::card::PowerToughness::fixed(2,2)).with_ability(program);
  if rule {
   let event=Trigger::from_model(ironsmith_core::trigger_model::Trigger::dies(ObjectFilter::creature().you_control())).unwrap();
   let model=crate::static_abilities::CompiledStaticAbility::trigger_class_event_choice("echo bell".into(),event,
    Some(ObjectFilter::default().you_control().in_zone(Zone::Battlefield)),true,false,1,true,false,false);
   builder=builder.with_ability(Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model)));
  }
  builder.build()
 }
 fn departure(game:&mut GameState,id:ObjectId,to:Zone)->TriggerEvent {
  game.move_object_by_effect(id,to).unwrap();
  // Transfer the real producer observation to this test's explicit capture
  // owner; constructing a second event would represent a second occurrence.
  let mut pending=game.take_pending_trigger_events();
  let matches=|event:&TriggerEvent|event.downcast::<crate::events::ZoneChangeEvent>()
   .is_some_and(|zone|zone.from==Zone::Battlefield&&zone.to==to&&zone.objects.contains(&id));
  assert_eq!(pending.iter().filter(|event|matches(event)).count(),1);
  let index=pending.iter().position(matches).unwrap();let event=pending.remove(index);
  game.effect_store.pending_trigger_events.extend(pending);
  event
 }
 struct Pick {source:Option<ObjectId>,offers:usize,option_counts:Vec<usize>}
 impl DecisionMaker for Pick {
  fn decide_options(&mut self,g:&GameState,c:&crate::decisions::context::SelectOptionsContext)->Vec<usize> {
   if c.description.contains("echo bell triggered program") {
    self.offers+=1;self.option_counts.push(c.options.len());
    return self.source.and_then(|source|c.options.iter().find(|option|option.object_id==Some(source)).map(|option|vec![option.index])).unwrap_or_default();
   }
   SelectFirstDecisionMaker.decide_options(g,c)
  }
 }
 #[test]
 fn duplicate_source_rules_offer_once_and_dying_program_retains_its_own_lki() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&definition("First provider",1,true,false),A,Zone::Battlefield);
  game.create_object_from_definition(&definition("Second provider",2,true,false),A,Zone::Battlefield);
  let victim=game.create_object_from_definition(&definition("Dying provider",9,true,true),A,Zone::Battlefield);
  let event=departure(&mut game,victim,Zone::Graveyard);
  let choices=capture_event_choices(&game,&event).unwrap();assert_eq!(choices.len(),1);
  assert_eq!(choices[0].candidates.len(),3);
  let mut queue=super::super::TriggerQueue::new();queue.class_event_choices=choices;
  assert!(queue.entries.is_empty());assert!(!queue.has_ability_triggered_events(),"unselected candidates do not trigger");
  let mut dm=Pick{source:Some(victim),offers:0,option_counts:Vec::new()};
  crate::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  assert_eq!(dm.offers,1);assert_eq!(dm.option_counts,vec![3]);assert_eq!(game.stack.len(),1);
  crate::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(A).unwrap().life,29,"the chosen departed source owns its original body");
 }
 #[test]
 fn decline_wrong_controller_and_replaced_destination_never_create_occurrences() {
  for (owner,destination,expect_offer) in [(A,Zone::Graveyard,true),(B,Zone::Graveyard,false),(A,Zone::Exile,false)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   game.create_object_from_definition(&definition("Rule provider",1,true,false),A,Zone::Battlefield);
   let victim=game.create_object_from_definition(&definition("Subject",9,false,true),owner,Zone::Battlefield);
   let event=departure(&mut game,victim,destination);
   let choices=capture_event_choices(&game,&event).unwrap();assert_eq!(!choices.is_empty(),expect_offer);
   let mut queue=super::super::TriggerQueue::new();queue.class_event_choices=choices;
   let mut dm=Pick{source:None,offers:0,option_counts:Vec::new()};
   crate::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
   assert_eq!(dm.offers,usize::from(expect_offer));assert!(game.stack.is_empty());
   assert!(!queue.has_ability_triggered_events());assert_eq!(game.player(A).unwrap().life,20);
  }
 }
 #[test]
 fn nested_action_queue_transfer_retains_unselected_candidates_and_batched_deaths_are_independent() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&definition("Rule provider",1,true,false),A,Zone::Battlefield);
  let first=game.create_object_from_definition(&definition("First subject",2,false,true),A,Zone::Battlefield);
  let second=game.create_object_from_definition(&definition("Second subject",3,false,true),A,Zone::Battlefield);
  let snapshots=[first,second].map(|id|ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(id).unwrap(),&game).unwrap());
  let mut event=crate::events::ZoneChangeEvent::batch(vec![first,second],Zone::Battlefield,Zone::Graveyard,crate::events::EventCause::effect());
  event.snapshot=Some(snapshots[0].clone());event.snapshots=snapshots.to_vec();
  // The explicit synthetic batch below owns these simultaneous observations.
  // Individual state-construction moves must not additionally publish deaths.
  game.with_suppressed_action_observations(|game| {
   game.move_object_by_effect(first,Zone::Graveyard).unwrap();
   game.move_object_by_effect(second,Zone::Graveyard).unwrap();
  });
  let event=TriggerEvent::new(event,Default::default());
  let mut captured=super::super::TriggerQueue::new();captured.class_event_choices=capture_event_choices(&game,&event).unwrap();
  assert_eq!(captured.class_event_choices.len(),2);
  assert!(captured.class_event_choices.iter().all(|choice|choice.candidates.len()==3),
   "both simultaneous origin programs remain eligible for either death choice");
    let provider=*game.battlefield.iter().find(|id|game.object(**id).unwrap().name.as_ref()=="Rule provider").unwrap();
  game.object_mut(provider).unwrap().abilities_mut().clear();
  captured.defer_into_game(&mut game);assert!(captured.is_fully_empty());
  let mut dm=Pick{source:Some(provider),offers:0,option_counts:Vec::new()};let mut queue=super::super::TriggerQueue::new();
  crate::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  assert!(queue.class_event_choices.is_empty(),"real decision drain completes the event-time choice before continuation");
  assert_eq!(dm.offers,2);assert_eq!(queue.entries.len(),2,"chosen bodies are retained before stack placement");
  assert!(game.stack.is_empty());
  crate::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  assert_eq!(dm.offers,2);assert_eq!(dm.option_counts,vec![3,3]);assert_eq!(game.stack.len(),2);
  for _ in 0..2 {crate::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  assert_eq!(game.player(A).unwrap().life,22,"event-time programs survive ability removal before stacking");
 }
 #[test]
 fn exact_live_event_snapshot_keeps_ability_presence_and_controller_but_old_departed_lookbacks_do_not() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let provider=game.create_object_from_definition(&definition("Captured provider",1,true,false),A,Zone::Battlefield);
  let before=ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(provider).unwrap(),&game).unwrap();
  let vanilla=CardDefinitionBuilder::new(CardId::new(),"Unlabelled victim").card_types(vec![CardType::Creature])
   .power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
  let victim=game.create_object_from_definition(&vanilla,A,Zone::Battlefield);
  let event=departure(&mut game,victim,Zone::Graveyard).with_lookback_source_snapshots(vec![before.clone()]);
  game.set_current_controller(provider,B).unwrap();
  assert_eq!(game.current_controller(provider),Some(B));
  game.object_mut(provider).unwrap().abilities_mut().clear();
  let choices=capture_event_choices(&game,&event).unwrap();assert_eq!(choices.len(),1);assert_eq!(choices[0].controller,A);
  let mut queue=super::super::TriggerQueue::new();queue.class_event_choices=choices;
  let mut dm=Pick{source:Some(provider),offers:0,option_counts:Vec::new()};
  crate::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  crate::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(A).unwrap().life,21);assert_eq!(game.player(B).unwrap().life,20);
  game.move_object_by_effect(provider,Zone::Graveyard).unwrap();
  let next=game.create_object_from_definition(&vanilla,A,Zone::Battlefield);
  let unrelated=departure(&mut game,next,Zone::Graveyard).with_lookback_source_snapshots(vec![before]);
  assert!(capture_event_choices(&game,&unrelated).unwrap().is_empty(),"an unrelated departed source cannot be revived from an old lookback");
 }

 #[test]
 fn selection_and_real_trigger_notifications_finish_before_a_later_instruction_removes_abilities() {
  struct BeforeRemoval {provider:ObjectId,calls:usize}
  impl DecisionMaker for BeforeRemoval {
   fn decide_options(&mut self,game:&GameState,context:&crate::decisions::context::SelectOptionsContext)->Vec<usize> {
    if context.description.contains("echo bell triggered program") {
     self.calls+=1;
     assert!(game.current_characteristics(self.provider).unwrap().abilities.iter().any(|ability|matches!(ability.kind,AbilityKind::Triggered(_))),
      "the later ability-removal instruction must not run before the as-event choice");
     return vec![context.options.iter().find(|option|option.object_id==Some(self.provider)).unwrap().index];
    }
    SelectFirstDecisionMaker.decide_options(game,context)
   }
  }
  for remove_observer in [false,true] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let provider=game.create_object_from_definition(&definition("Chosen provider",1,true,false),A,Zone::Battlefield);
   let filter=ObjectFilter {specific:Some(provider),..Default::default()};
   let observer=CardDefinitionBuilder::new(CardId::new(),"Trigger observer").card_types(vec![CardType::Artifact])
    .with_ability(Ability::triggered(Trigger::ability_triggered_qualified(false,Some(filter),false),vec![Effect::gain_life(10)])).build();
   game.create_object_from_definition(&observer,A,Zone::Battlefield);
   let victim_definition=CardDefinitionBuilder::new(CardId::new(),"Unlabelled victim").card_types(vec![CardType::Creature])
    .power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
   let victim=game.create_object_from_definition(&victim_definition,A,Zone::Battlefield);
   let remove=crate::effects::ApplyContinuousEffect::new(
    if remove_observer {crate::continuous::EffectTarget::AllPermanents} else {crate::continuous::EffectTarget::Specific(provider)},
    crate::continuous::Modification::RemoveAllAbilities,crate::effect::Until::EndOfTurn);
   let program=Effect::new(crate::effects::SequenceEffect::new(vec![
    Effect::new(crate::effects::MoveToZoneEffect::new(crate::target::ChooseSpec::SpecificObject(victim),Zone::Graveyard,false)),Effect::new(remove)]));
   let mut dm=BeforeRemoval{provider,calls:0};
   crate::effects::execute_effect(&mut game,&program,&mut crate::effects::ExecutionContext::new(provider,A,&mut dm)).unwrap();
   assert_eq!(dm.calls,1);assert!(!game.current_characteristics(provider).unwrap().abilities.iter().any(|ability|matches!(ability.kind,AbilityKind::Triggered(_))));
   assert_eq!(game.effect_store.pending_trigger_entries.len(),2,"selected program and actual trigger observer are retained before continuation");
   let mut queue=super::super::TriggerQueue::new();
   for step in 0..10 {
    crate::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
    if game.stack.is_empty(){break;}
    assert!(step<9);crate::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   }
   assert_eq!(game.player(A).unwrap().life,31,"notification is captured before observer loss, and never emitted twice after queue transfer");
  }
 }

 #[test]
 fn official_class_uses_owned_program_identity_not_presentation_or_body_similarity() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let actual=CardDefinitionBuilder::new(CardId::new(),"Official keyword donor").card_types(vec![CardType::Creature])
   .power_toughness(crate::card::PowerToughness::fixed(2,2)).prowess().build();
  let source=game.create_object_from_definition(&actual,A,Zone::Battlefield);
  let mut snapshot=ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(source).unwrap(),&game).unwrap();
  let event=TriggerEvent::new(crate::events::NamedActionEvent::new("gather",A,source,snapshot.clone(),Vec::new(),1,crate::events::EventCause::effect()),Default::default());
  assert_eq!(entries_from_snapshot(&game,&snapshot,"prowess",true,&event).len(),1);
  let mut abilities=snapshot.abilities.to_vec();
  for ability in &mut abilities {
   if let AbilityKind::Triggered(triggered)=&mut ability.kind {triggered.effects.keyword=None;}
  }
  snapshot.abilities=abilities.into();
  assert!(entries_from_snapshot(&game,&snapshot,"prowess",true,&event).is_empty(),"presentation and identical pump body cannot invent official identity");
  assert!(entries_from_snapshot(&game,&snapshot,"prowess",false,&event).is_empty(),"official keyword is not an external class homonym");
 }

}
