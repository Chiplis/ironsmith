//! Every original Harmony/Harmonize record is admitted independently, without representative-only success.
#[path = "custom_final_next_wave_support.rs"] mod source_support;
#[test]
fn every_harmony_harmonize_source_record_strictly_round_trips() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-harmony-external-abilities.json")).unwrap();
 let mut records=0; let mut faces=0; let mut keys=std::collections::BTreeSet::new(); let mut failures=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap() { for card in row["representative_cards"].as_array().unwrap() {
  records+=1;let key=card["source_card_key"].as_str().expect("exact source identity");keys.insert(key);
  let label=format!("{} / {key}",row["id"]);
  if card["rewrite_status"]!="frontend_materialized_unverified" { failures.push(format!("{label}: source was not materialized: {}",card["rewrite_status"])); }
  let Some(inputs)=card["executable_faces"].as_array() else {failures.push(format!("{label}: missing executable faces"));continue};
  if inputs.len()!=card["source_faces"].as_array().unwrap().len() {failures.push(format!("{label}: incomplete physical face union"));}
  for face in inputs { faces+=1; match source_support::definitions(face) {
   Ok([direct,indirect])=>{
    assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect),"{label}");
    let expected=card["source_faces"].as_array().unwrap().iter().find(|source|source["name"]==face["name"])
      .and_then(|source|source["original_text"].as_str()).expect("compiled face must retain exact source association")
      .lines().filter(|line|line.trim_start().starts_with("Harmony —")||line.trim_start().starts_with("Harmony –")).count();
    for definition in [&direct,&indirect] {
     let actual=definition.abilities.iter().filter(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Triggered(triggered)
       if triggered.trigger.downcast_ref::<ironsmith::triggers::special::CustomTrigger>().is_some_and(|marker|marker.id=="externally-triggered:harmony"))).count();
     if actual!=expected {failures.push(format!("{label} / {}: expected {expected} independently retained Harmony programs, found {actual}",face["name"]));}
    }
   },
   Err(error)=>failures.push(format!("{label} / {}: {error}",face["name"]))
  }}
 }}
 assert_eq!(records,69,"original record count must remain unchanged");assert_eq!(keys.len(),68,"exact physical union must remain unchanged");
 assert!(failures.is_empty(),"{} whole-source failures across {records} records / {faces} supplied faces:\n{}",failures.len(),failures.join("\n"));
 assert_eq!(faces,69);
}

use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn actual_source(key:&str)->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-harmony-external-abilities.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]==key).expect("exact original source present");
 assert_eq!(card["rewrite_status"],"frontend_materialized_unverified");
 source_support::definitions(&card["executable_faces"][0]).unwrap()
}
fn harmony_program()->[CardDefinition;2]{source_support::definitions(&serde_json::json!({"name":"Independent harmonize instruction","executable_text":"Type: Sorcery\nPerform the tune ensemble action as follows: Trigger up to one harmony triggered ability of permanents you control as the tune ensemble action."})).unwrap()}
fn harmonize(game:&mut GameState,program:&CardDefinition,dm:&mut dyn DecisionMaker)->usize{
 let source=game.create_object_from_definition(program,A,Zone::Stack);let mut context=EffectContext::new(source,A,dm);let mut completed=0;
 for effect in program.spell_effect.as_ref().unwrap().all_effects(){let outcome=execute_effect(game,effect,&mut context).unwrap();completed+=outcome.events.iter().filter(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|action|action.label=="tune ensemble")).count();}
 completed
}
fn settle(game:&mut GameState){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}}
#[test]
fn actual_fiddler_harmony_queues_normally_and_checks_current_controller_and_zone(){
 for (index,donor) in actual_source("New Alores Fiddler_EAU").into_iter().enumerate(){let program=harmony_program()[index].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&donor,B,Zone::Battlefield);game.create_object_from_definition(&donor,A,Zone::Graveyard);
  assert_eq!(harmonize(&mut game,&program,&mut SelectFirstDecisionMaker),1,"zero candidates still completes harmonize");settle(&mut game);assert_eq!(game.player(A).unwrap().life,20);
  game.create_object_from_definition(&donor,A,Zone::Battlefield);
  assert_eq!(harmonize(&mut game,&program,&mut SelectFirstDecisionMaker),1);assert_eq!(game.player(A).unwrap().life,20,"harmony is queued rather than executed inline");settle(&mut game);assert_eq!(game.player(A).unwrap().life,22);
 }
}
#[test]
fn actual_herald_graveyard_replacement_returns_only_its_source_instead_of_harmonizing(){
 for (index,herald) in actual_source("Herald of Festivities_EAU").into_iter().enumerate(){let program=harmony_program()[index].clone();let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let id=game.create_object_from_definition(&herald,A,Zone::Graveyard);
  assert_eq!(harmonize(&mut game,&program,&mut SelectFirstDecisionMaker),0,"replaced action has no original completion");
  assert!(!game.player(A).unwrap().graveyard.contains(&id));assert_eq!(game.player(A).unwrap().hand.len(),1);
 }
}

struct HarmonyChoices { accept:bool, booleans:usize }
impl DecisionMaker for HarmonyChoices {
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{(0..ctx.max.min(ctx.options.len())).collect()}
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.booleans+=1;self.accept}
}
#[test]
fn actual_sending_stones_replaces_with_a_graveyard_occurrence_and_respects_its_own_zone(){
 for (index,stones) in actual_source("Sending Stones_ACA").into_iter().enumerate(){
  let donor=actual_source("New Alores Fiddler_EAU")[index].clone();let program=harmony_program()[index].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&stones,A,Zone::Battlefield);
  let own=game.create_object_from_definition(&donor,A,Zone::Graveyard);
  game.create_object_from_definition(&donor,B,Zone::Graveyard);
  let mut choices=HarmonyChoices{accept:true,booleans:0};
  assert_eq!(harmonize(&mut game,&program,&mut choices),0,"choosing the replacement does not complete original harmonize");
  assert_eq!(game.player(A).unwrap().life,20,"selected external program awaits normal trigger resolution");
  settle(&mut game);assert_eq!(game.player(A).unwrap().life,22);assert!(game.player(A).unwrap().graveyard.contains(&own),"external invocation doesn't move the selected card");
  game.remove_object(source);
  assert_eq!(harmonize(&mut game,&program,&mut choices),1,"without Sending Stones the ordinary battlefield domain applies");settle(&mut game);assert_eq!(game.player(A).unwrap().life,22);
 }
}
#[test]
fn actual_lightning_replacement_is_stack_only_and_puts_counters_on_that_exact_spell(){
 for (index,lightning) in actual_source("Lightning and Thunder_ACA").into_iter().enumerate(){
  let program=harmony_program()[index].clone();
  for zone in [Zone::Stack,Zone::Graveyard,Zone::Battlefield] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&lightning,A,zone);
   let other=game.create_object_from_definition(&lightning,B,Zone::Stack);
   let mut choices=HarmonyChoices{accept:true,booleans:0};
   assert_eq!(harmonize(&mut game,&program,&mut choices),usize::from(zone!=Zone::Stack));
   assert_eq!(game.counter_count(source,ironsmith::CounterType::Named("thunder".into())),if zone==Zone::Stack{3}else{0});
   assert_eq!(game.counter_count(other,ironsmith::CounterType::Named("thunder".into())),0,"another player's spell cannot replace your operation");
   if zone==Zone::Stack {
    assert_eq!(harmonize(&mut game,&program,&mut choices),0);
    assert_eq!(game.counter_count(source,ironsmith::CounterType::Named("thunder".into())),6,"separate harmonize proposals each offer the replacement");
   }
  }
 }
}
#[test]
fn actual_lilia_optional_multiplier_decline_preserves_the_once_per_turn_offer(){
 for (index,lilia) in actual_source("Lilia of the Soul_OTH").into_iter().enumerate(){
  let donor=actual_source("New Alores Fiddler_EAU")[index].clone();let program=harmony_program()[index].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&lilia,A,Zone::Battlefield);game.create_object_from_definition(&donor,A,Zone::Battlefield);
  let mut choices=HarmonyChoices{accept:false,booleans:0};
  assert_eq!(harmonize(&mut game,&program,&mut choices),1);settle(&mut game);assert_eq!(game.player(A).unwrap().life,22);assert_eq!(choices.booleans,1);
  choices.accept=true;
  assert_eq!(harmonize(&mut game,&program,&mut choices),1);settle(&mut game);assert_eq!(game.player(A).unwrap().life,26);assert_eq!(choices.booleans,2);
  assert_eq!(harmonize(&mut game,&program,&mut choices),1);settle(&mut game);assert_eq!(game.player(A).unwrap().life,28);assert_eq!(choices.booleans,2,"accepted use consumes only this turn's doubling offer");
 }
}
#[test]
fn actual_conductor_can_choose_distinct_occurrences_but_never_duplicate_the_only_one(){
 for (index,conductor) in actual_source("Gilded Conductor_ACA").into_iter().enumerate(){
  let donor=actual_source("New Alores Fiddler_EAU")[index].clone();let program=harmony_program()[index].clone();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&conductor,A,Zone::Battlefield);
  let mut choices=HarmonyChoices{accept:true,booleans:0};
  assert_eq!(harmonize(&mut game,&program,&mut choices),1);
  let mut queue=ironsmith::triggers::TriggerQueue::new();
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.stack.len(),1,"one selectable Harmony occurrence cannot fill both choice positions");
  while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  game.create_object_from_definition(&donor,A,Zone::Battlefield);
  assert_eq!(harmonize(&mut game,&program,&mut choices),1);
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.stack.len(),2,"Conductor's own Scry and Fiddler's life program are distinct occurrences");
  while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  assert_eq!(game.player(A).unwrap().life,22);
 }
}
