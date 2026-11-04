//! Whole raw-source token/resource families; fixture availability is not support proof.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-token-resource-whole-family-source.json")).unwrap()}
fn card(key:&str)->serde_json::Value {fixture()["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]==key).unwrap().clone()}
fn definitions(key:&str)->[CardDefinition;2] {source_support::definitions(&card(key)).unwrap_or_else(|error|panic!("{key}: {error}"))}
fn collect(family:&str,expected:usize) {
 let fixture=fixture();assert_eq!(fixture["physical_source_keys"].as_array().unwrap().len(),162);
 let mut keys=std::collections::BTreeSet::new();let mut errors=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap().iter().filter(|row|row["keyword"]==family) {
  for card in row["representative_cards"].as_array().unwrap() {
   let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key.to_string()),"duplicate {family} source {key}");
   assert_eq!(card["source_export"]["key"],card["source_card_key"]);
   let selected=card["source_faces"].as_array().unwrap().iter().find(|face|face["name"]==card["name"]).unwrap();
   assert_eq!(selected["original_text"],card["original_text"]);
   let raw_field=if card["source_face"]=="back" {"rulesText2"}else{"rulesText"};
   assert!(card["original_text"].as_str().unwrap().ends_with(card["source_export"][raw_field].as_str().unwrap()),"raw full body changed: {key}");
   if card["rewrite_status"]!="frontend_materialized_unverified" {errors.push(format!("{key}: requires root frontend materialization: {}",card["rewrite_status"]));continue;}
   if family=="Recruit" {
    for face in card["executable_faces"].as_array().unwrap() {
     assert!(face["custom_keyword_identities"].as_array().unwrap().iter().all(|identity|!identity.as_str().unwrap().eq_ignore_ascii_case("recruit")),"official Recruit cannot become a custom identity");
    }
   }
   match source_support::linked_definitions(card) {
    Err(error)=>errors.push(format!("{key}: {error}")),
    Ok(faces)=> {
     if faces.len()!=card["source_faces"].as_array().unwrap().len() {errors.push(format!("{key}: linked face count changed"));}
     for [native,artifact] in faces {
      for definition in [&native,&artifact] {if ironsmith::cards::generated_definition_has_unimplemented_content(definition) {errors.push(format!("{key}: unimplemented program"));}}
      let typed=|definition:CardDefinition|->Result<serde_json::Value,String>{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("{error:?}"))?;serde_json::to_value(wire).map_err(|error|error.to_string())};
      match (typed(native),typed(artifact)) {
       (Ok(native),Ok(artifact))=>assert_eq!(native,artifact,"{key}: complete native/artifact typed transport changed"),
       (Err(error),_)|(_,Err(error))=>errors.push(format!("{key}: typed encoding: {error}")),
      }
     }
    }
   }
  }
 }
 assert_eq!(keys.len(),expected,"exact {family} source universe");
 assert!(errors.is_empty(),"{}",errors.join("\n"));
}
#[test] fn all_adjudicate_twenty_one_full_sources(){collect("Adjudicate",21);}
#[test] fn all_arm_seventeen_full_sources(){collect("Arm",17);}
#[test] fn all_forebode_twenty_one_full_sources_including_nonseed_actor(){collect("Forebode",21);}
#[test] fn all_nourish_eighteen_full_sources(){collect("Nourish",18);}
#[test] fn all_profit_two_full_sources(){collect("Profit",2);}
#[test] fn all_reappropriate_twenty_nine_full_sources(){collect("Reappropriate",29);}
#[test] fn all_germinate_twenty_three_linked_full_sources(){collect("Germinate",23);}
#[test] fn all_recruit_thirty_three_physical_sources_use_canonical_muster_alias(){collect("Recruit",33);}
struct Choices(bool);
impl DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,context:&ironsmith::decisions::context::BooleanContext)->bool {self.0&&context.can_accept}
 fn decide_objects(&mut self,_:&GameState,context:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
  context.candidates.iter().filter(|object|object.legal).map(|object|object.id).take(context.min.max(usize::from(self.0))).collect()
 }
 fn decide_options(&mut self,game:&GameState,context:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {SelectFirstDecisionMaker.decide_options(game,context)}
}
fn settle(game:&mut GameState,dm:&mut impl DecisionMaker) {
 let mut queue=ironsmith::triggers::TriggerQueue::new();
 for step in 0..50 {
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
  if game.stack_is_empty(){return;}
  assert!(step<49);ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
 }
}
fn body()->CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary material","Type: Creature\nPower/Toughness: 2/2",false).unwrap()}
fn enter(game:&mut GameState,definition:&CardDefinition,dm:&mut impl DecisionMaker)->ObjectId {
 let source=game.create_object_from_definition(definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
 let outcome=execute_effect(game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Battlefield,false),&mut EffectContext::new(source,A,dm)).unwrap();
 for event in outcome.events {game.queue_trigger_event(Default::default(),event);}
 settle(game,dm);game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn actual_curator_adjudicates_attached_aura_and_observes_its_real_entry() {
 for definition in definitions("Curator of Cures_MIS") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=enter(&mut game,&definition,&mut Choices(true));
  let auras=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.kind==ironsmith::object::ObjectKind::Token&&object.subtypes.contains(&ironsmith::Subtype::Aura)).collect::<Vec<_>>();
  assert_eq!(auras.len(),1);assert_eq!(auras[0].attached_to,Some(ironsmith::object::AttachmentTarget::Object(source)));
  assert!(!auras[0].abilities.is_empty());assert_eq!(game.player(A).unwrap().life,21);
 }
}
#[test]
fn actual_frostfield_swordsman_is_equipped_and_actual_blacksmith_can_nourish() {
 for definition in definitions("Frostfield Swordsman_GHQ") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=enter(&mut game,&definition,&mut Choices(true));
  let equipment=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.kind==ironsmith::object::ObjectKind::Token&&object.subtypes.contains(&ironsmith::Subtype::Equipment)).collect::<Vec<_>>();
  assert_eq!(equipment.len(),1);assert_eq!(equipment[0].attached_to,Some(ironsmith::object::AttachmentTarget::Object(source)));
  let printed=card("Frostfield Swordsman_GHQ")["power"].as_str().unwrap().parse::<i32>().unwrap();
  assert_eq!(game.current_power(source),Some(printed+1));assert!(!equipment[0].abilities.is_empty());
 }
 for definition in definitions("Crinster Blacksmith_HVF") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=enter(&mut game,&definition,&mut Choices(true));
  let counters=game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne);
  let citizens=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.kind==ironsmith::object::ObjectKind::Token&&object.subtypes.contains(&ironsmith::Subtype::Citizen)).count();
  assert!(counters==2&&citizens==0||counters==0&&citizens==2,"nourish chooses one complete program, not both");
 }
}
fn spell(game:&mut GameState,definition:&CardDefinition,targets:Vec<ResolvedTarget>,dm:&mut impl DecisionMaker) {
 let source=game.create_object_from_definition(definition,A,Zone::Stack);let mut context=EffectContext::new(source,A,dm).with_targets(targets);
 for effect in definition.spell_effect.as_ref().unwrap().all_effects() {execute_effect(game,effect,&mut context).unwrap();}
}
#[test]
fn actual_under_pressure_forebodes_for_discarding_opponent_and_profit_for_exiled_controller() {
 for definition in definitions("Under Pressure_MSY") {for accepted in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let material=game.create_object_from_definition(&body(),B,Zone::Hand);let stable=game.object(material).unwrap().stable_id;
  spell(&mut game,&definition,vec![ResolvedTarget::Player(B)],&mut Choices(accepted));
  let omens=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.subtypes.iter().any(|subtype|subtype.to_string()=="Omen")).collect::<Vec<_>>();
  assert_eq!(omens.len(),usize::from(accepted));
  if accepted {assert_eq!(game.controller_of(omens[0]),B,"elided then forebodes retains the discarding actor");assert!(game.is_tapped(omens[0].id));}
  assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,if accepted{Zone::Graveyard}else{Zone::Hand});assert_eq!(game.player(A).unwrap().life,19);
 }}
 for definition in definitions("Turn to Gold_MS2") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let target=game.create_object_from_definition(&body(),A,Zone::Battlefield);let stable=game.object(target).unwrap().stable_id;
  let controller_source=game.create_object_from_definition(&body(),B,Zone::Battlefield);
  execute_effect(&mut game,&ironsmith::Effect::gain_control_with_duration(ironsmith::target::ChooseSpec::SpecificObject(target),ironsmith::effect::Until::Forever),&mut EffectContext::new_default(controller_source,B)).unwrap();
  assert_eq!(game.current_controller(target),Some(B));assert_eq!(game.object(target).unwrap().owner,A);
  spell(&mut game,&definition,vec![ResolvedTarget::Object(target)],&mut Choices(true));
  let gold=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.subtypes.iter().any(|subtype|subtype.to_string()=="Gold")).collect::<Vec<_>>();
  assert_eq!(gold.len(),1);assert_eq!(game.controller_of(gold[0]),B);assert!(!gold[0].abilities.is_empty());assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
 }
}
#[test]
fn actual_forager_optional_mixed_zone_price_distinguishes_payment_from_decline() {
 for definition in definitions("Wilderness Forager_LAW") {for paid in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let one=game.create_object_from_definition(&body(),A,Zone::Hand);let two=game.create_object_from_definition(&body(),A,Zone::Graveyard);
  let stable=[game.object(one).unwrap().stable_id,game.object(two).unwrap().stable_id];
  enter(&mut game,&definition,&mut Choices(paid));assert_eq!(game.player(A).unwrap().life,if paid{26}else{23});
  for (index,id) in stable.into_iter().enumerate() {assert_eq!(game.object(game.find_object_by_stable_id(id).unwrap()).unwrap().zone,if paid{Zone::Exile}else if index==0{Zone::Hand}else{Zone::Graveyard});}
 }}
}

#[test]
fn actual_affluent_knight_repeats_complete_muster_and_keeps_enemy_armies_unchanged() {
 for definition in definitions("Affluent Knight_ALK") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let army=ironsmith_compiler_runtime::compile_to_runtime_definition("Other Army","Type: Creature — Army\nPower/Toughness: 2/2",false).unwrap();
  let enemy=game.create_object_from_definition(&army,B,Zone::Battlefield);
  enter(&mut game,&definition,&mut Choices(true));
  let mut allied=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.kind==ironsmith::object::ObjectKind::Token&&game.controller_of(object)==A&&object.subtypes.contains(&ironsmith::Subtype::Army)).map(|object|game.counter_count(object.id,ironsmith::object::CounterType::PlusOnePlusOne)).collect::<Vec<_>>();
  allied.sort();assert_eq!(allied,vec![1,2]);assert_eq!(game.counter_count(enemy,ironsmith::object::CounterType::PlusOnePlusOne),0);
 }
}

#[test]
fn actual_vagabond_replaces_germination_instead_of_also_running_original_body() {
 for (provider,entrant) in definitions("Ontarachie Vagabond_EAU").into_iter().zip(definitions("Woopip Sproutling_EAU")) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&provider,A,Zone::Battlefield);
  let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);
  enter(&mut game,&entrant,&mut Choices(true));
  let saprolings=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.kind==ironsmith::object::ObjectKind::Token&&object.subtypes.contains(&ironsmith::Subtype::Saproling)).count();
  let allied_counters=game.battlefield.iter().copied().filter(|id|game.controller_of(game.object(*id).unwrap())==A).map(|id|game.counter_count(id,ironsmith::object::CounterType::PlusOnePlusOne)).sum::<u32>();
  assert_eq!(saprolings,1);assert_eq!(allied_counters,1,"replacement creates once then counters once; original germination must not also run");
  assert_eq!(game.counter_count(enemy,ironsmith::object::CounterType::PlusOnePlusOne),0,"replacement says a creature you control");
 }
}
