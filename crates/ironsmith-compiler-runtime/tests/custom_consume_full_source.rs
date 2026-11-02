//! Consume uses one atomic optional resource program with the authored payer.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
const A: PlayerId = PlayerId(0); const B: PlayerId = PlayerId(1);
fn fixture() -> serde_json::Value { serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-consume-actions.json")).unwrap() }
fn card(name: &str) -> (String, String) {
 let f=fixture(); let c=f["mechanics"].as_array().unwrap().iter().flat_map(|m|m["representative_cards"].as_array().unwrap()).find(|c|c["name"]==name).unwrap();
 (name.into(),c["executable_text"].as_str().unwrap().into())
}
fn definitions(name:&str,text:&str)->[CardDefinition;2]{
 let f=fixture();
 let card=f["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap())
  .find(|card|card["name"]==name && card["executable_text"]==text);
 let authored=serde_json::json!({"name":name,"executable_text":text});
 source_support::definitions(card.unwrap_or(&authored))
  .unwrap_or_else(|error|panic!("{name} independent native/artifact: {error}"))
}

fn body()->CardDefinition { ironsmith_compiler_runtime::compile_to_runtime_definition("Resource body","Type: Creature — Fox\nPower/Toughness: 1/1",false).unwrap() }
fn triggers(def:&CardDefinition)->Vec<Vec<Effect>> {def.abilities.iter().filter_map(|a|match &a.kind {ironsmith::ability::AbilityKind::Triggered(t)=>Some(t.effects.all_effects_owned()),_=>None}).collect()}
struct Choices { accept:bool, expected_payer:PlayerId, offers:Vec<bool> }
impl DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,c:&ironsmith::decisions::context::BooleanContext)->bool {assert_eq!(c.player,self.expected_payer);self.offers.push(c.can_accept);self.accept}
 fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{assert_eq!(c.player,self.expected_payer);let mut fallback=ironsmith::decision::SelectFirstDecisionMaker;fallback.decide_objects(g,c)}
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{assert_eq!(c.player,self.expected_payer);let mut fallback=ironsmith::decision::SelectFirstDecisionMaker;fallback.decide_options(g,c)}
}
fn execute_program(game:&mut GameState,source:ObjectId,effects:Vec<Effect>,actor:PlayerId,dm:&mut dyn DecisionMaker){
 let spell=game.create_object_from_definition(&body(),actor,Zone::Stack);
 let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new(spell,actor,Zone::Hand),Default::default());
 let mut ctx=EffectContext::new_default(source,A).with_decision_maker(dm);
 // Only the opposing cast trigger needs an event-bound player. Other source
 // programs use the permanent's own source operand rather than this spell.
 if actor == B { ctx = ctx.with_triggering_event(event); }
 execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap();
}
fn resolve_actual_trigger(game:&mut GameState,source:ObjectId,event:ironsmith::triggers::TriggerEvent,dm:&mut impl DecisionMaker){
 let entries:Vec<_>=ironsmith::triggers::check_triggers(game,&event).into_iter().filter(|entry|entry.source==source).collect();
 assert_eq!(entries.len(),1,"one actual source trigger for the typed event");
 let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries{queue.add(entry);}
 ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();assert_eq!(game.stack.len(),1);
 ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
}

#[test]
fn all_twelve_exact_consume_sources_and_both_remi_faces_compile_without_loss(){
 let f=fixture();let mut count=0;
 for m in f["mechanics"].as_array().unwrap(){for c in m["representative_cards"].as_array().unwrap(){source_support::definitions(c).unwrap();count+=1;}}
 assert_eq!(count,12);
 let f:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-standard-actions.json")).unwrap();
 let remi=f["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-383").unwrap()["representative_cards"].as_array().unwrap().iter().find(|c|c["name"]=="Remi Amber, Slyten Specialist").unwrap();
 let faces=remi["executable_faces"].as_array().unwrap();assert_eq!(faces.len(),2);
 let linked=source_support::linked_definitions(remi).unwrap();assert_eq!(linked.len(),2);
 // Compile the complete linked physical card on both routes, retaining
 // per-face subtype vocabulary, identities, layout and actual sibling IDs.
 for index in 0..2 {
  let front=&linked[0][index];let back=&linked[1][index];
  assert_eq!(front.card.other_face,Some(back.card.id));assert_eq!(back.card.other_face,Some(front.card.id));
  assert_eq!(front.card.other_face_name.as_deref(),Some(back.card.name.as_str()));assert_eq!(back.card.other_face_name.as_deref(),Some(front.card.name.as_str()));
 }

}
#[test]
fn opposing_offer_owns_choices_and_resources_and_infeasible_modes_cannot_accept(){
 let(name,text)=card("Thunderous Avarice");
 for definition in definitions(&name,&text){for route in ["sacrifice","exile","impossible","decline"]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let own=game.create_object_from_definition(&body(),A,Zone::Battlefield);
  let resource=if route=="impossible"{None}else{Some(game.create_object_from_definition(&body(),B,if route=="exile"{Zone::Graveyard}else{Zone::Battlefield}))};let stable=resource.map(|id|game.object(id).unwrap().stable_id);
  let mut dm=Choices{accept:route!="decline",expected_payer:B,offers:vec![]};
  let spell=game.create_object_from_definition(&body(),B,Zone::Stack);
  let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new(spell,B,Zone::Hand),Default::default());
  resolve_actual_trigger(&mut game,source,event,&mut dm);
  assert_eq!(game.object(own).unwrap().zone,Zone::Battlefield,"source controller's resources never pay the opposing offer");assert_eq!(game.object(source).unwrap().zone,Zone::Battlefield,"the consuming creature cannot sacrifice itself");
  let paid=matches!(route,"sacrifice"|"exile");assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),if paid{0}else{1},"If they don't observes the actual payment result");
  if let Some(stable)=stable{let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,match route{"sacrifice"=>Zone::Graveyard,"exile"=>Zone::Exile,_=>Zone::Battlefield},"actual payer route {route}, offers={:?}",dm.offers);}
  if route=="impossible" {assert!(dm.offers.iter().all(|can_accept|!*can_accept));}
 }}
}
#[test]
fn sacrifice_receipt_alone_installs_hellkite_reflexive_followup(){
 let(name,text)=card("Hellkite Omnivore");
 for definition in definitions(&name,&text){for sacrifice in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.create_object_from_definition(&body(),A,if sacrifice{Zone::Battlefield}else{Zone::Graveyard});
  let mut dm=Choices{accept:true,expected_payer:A,offers:vec![]};
  let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(source,ironsmith::triggers::AttackEventTarget::Player(B)),Default::default());
  resolve_actual_trigger(&mut game,source,event,&mut dm);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),1);
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut fallback=ironsmith::decision::SelectFirstDecisionMaker;ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut fallback).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut fallback).unwrap();assert_eq!(game.stack.len(),usize::from(sacrifice),"the exile alternative must not satisfy sacrificed-this-way");
 }}
}
#[test]
fn exiled_resource_stays_linked_to_ooze_for_its_real_death_trigger(){
 let(name,text)=card("Regurgitator Ooze");
 for definition in definitions(&name,&text){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let resource=game.create_object_from_definition(&body(),A,Zone::Graveyard);let stable=game.object(resource).unwrap().stable_id;
  let mut dm=Choices{accept:true,expected_payer:A,offers:vec![]};execute_program(&mut game,source,triggers(&definition)[0].clone(),A,&mut dm);let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);assert_eq!(game.calculated_power(source),Some(5),"following it refers to the consuming Ooze");
  execute_effect(&mut game,&Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,false),&mut EffectContext::new_default(source,A)).unwrap();
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut fallback=ironsmith::decision::SelectFirstDecisionMaker;ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut fallback).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut fallback).unwrap();assert_eq!(game.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut fallback).unwrap();let returned=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(returned).unwrap().zone,Zone::Hand);
 }
}
