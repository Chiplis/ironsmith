//! Exact whole-source contracts; candidate admission and gameplay are separate assertions.
#[path = "custom_final_next_wave_support.rs"] mod source_routes;
#[path = "cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use std::collections::{BTreeSet, VecDeque};
const A: PlayerId = PlayerId(0); const B: PlayerId = PlayerId(1);
fn fixture()->serde_json::Value { serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-standard-actions.json")).unwrap() }
fn expected()->serde_json::Value { serde_json::from_str(include_str!("data/msem_standard_action_scope.json")).unwrap() }
fn full_source_contract(keyword:&str, count:usize) {
 let f=if keyword=="Consume" {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-consume-actions.json")).unwrap()} else {fixture()};let inventory=expected();
 let expected:BTreeSet<String>=inventory[keyword].as_array().unwrap().iter().map(|v|v.as_str().unwrap().to_owned()).collect();
 assert_eq!(expected.len(),count,"authoritative whole physical source set");
 let cards:Vec<_>=f["mechanics"].as_array().unwrap().iter().filter(|row|row["keyword"]==keyword).flat_map(|row|row["representative_cards"].as_array().unwrap()).collect();
 let actual:BTreeSet<String>=cards.iter().map(|c|c["source_card_key"].as_str().unwrap().to_owned()).collect();
 assert_eq!(cards.len(),count,"one candidate per physical source, no silent duplicate");assert_eq!(actual,expected);
 let mut physical_faces=0usize;
 let mut admission_failures=Vec::new();
 for card in cards {
  assert_eq!(card["source_raw_record"]["key"],card["source_card_key"],"exact raw provenance");
  assert!(card["original_text"].as_str().is_some_and(|s|!s.is_empty()));
  let faces=match source_routes::linked_definitions(card) {
   Ok(faces)=>faces,
   Err(error)=>{admission_failures.push(format!("{keyword} full source {}: {error}",card["source_card_key"]));continue;}
  };
  assert!(!faces.is_empty());
  assert_eq!(faces.len(),card["source_faces"].as_array().unwrap().len(),"no linked face dropped");
  physical_faces+=faces.len();
  for [native,artifact] in faces {
   assert_eq!(native.card.name,artifact.card.name);assert_eq!(native.abilities.len(),artifact.abilities.len());
   // Effect equality compares executor Arc identity. The complete typed wire
   // graph retains all IDs, costs, filters, labels and linked-source metadata.
   let native=serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(native).expect("native typed payload encoding")).unwrap();
   let artifact=serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(artifact).expect("artifact typed payload encoding")).unwrap();
   assert_eq!(native,artifact,"full native/artifact semantic graph: {}",card["source_card_key"]);
  }
 }
 assert!(admission_failures.is_empty(),"all whole-source admission failures:\n{}",admission_failures.join("\n"));
 assert_eq!(physical_faces,if keyword=="Crystallize"{22}else{count},"whole face count includes Remi front and back");
}
#[test] fn all_augur_sources_strictly_compile_on_independent_routes(){full_source_contract("Augur",1)}
#[test] fn all_invigorate_sources_strictly_compile_on_independent_routes(){full_source_contract("Invigorate",18)}
#[test] fn all_permutate_sources_including_spirit_transfusion_compile_on_independent_routes(){full_source_contract("Permutate",12)}
#[test] fn all_purge_sources_strictly_compile_on_independent_routes(){full_source_contract("Purge",17)}
#[test] fn all_rouse_sources_strictly_compile_on_independent_routes(){full_source_contract("Rouse",22)}
#[test] fn all_consume_sources_strictly_compile_on_independent_routes(){full_source_contract("Consume",12)}
#[test] fn all_crystallize_sources_including_all_remi_faces_compile_on_independent_routes(){full_source_contract("Crystallize",21)}
fn actual(name:&str)->[CardDefinition;2]{
 let f=fixture();let card=f["mechanics"].as_array().unwrap().iter().flat_map(|m|m["representative_cards"].as_array().unwrap()).find(|c|c["name"]==name).expect("actual full physical source");
 source_routes::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))
}
fn body(power:i32)->CardDefinition {play::vanilla("Resource body","{0}","Fox",power,8)}
#[derive(Default)]
struct Choices { answers:VecDeque<bool>, wanted:Vec<ObjectId>, observed:Vec<Vec<ObjectId>> }
impl DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool {self.answers.pop_front().unwrap_or(true)}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
  self.observed.push(ctx.candidates.iter().filter(|c|c.legal).map(|c|c.id).collect());
  if self.wanted.is_empty(){SelectFirstDecisionMaker.decide_objects(game,ctx)}else{ctx.candidates.iter().filter(|c|c.legal&&self.wanted.contains(&c.id)).map(|c|c.id).collect()}
 }
 fn decide_counters(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::CountersContext)->Vec<(CounterType,u32)>{ctx.available_counters.iter().find(|(_,n)|*n>0).map(|(kind,_)|vec![(*kind,1)]).unwrap_or_default()}
 fn decide_distribute(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::DistributeContext)->Vec<(Target,u32)>{
  assert_eq!(ctx.total,4,"actual Rousing Roots amount");let choices:Vec<_>=ctx.targets.iter().filter(|t|matches!(t.target,Target::Object(id)if self.wanted.contains(&id))).collect();assert_eq!(choices.len(),2);choices.into_iter().map(|t|(t.target,2)).collect()
 }
 fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<Target>{SelectFirstDecisionMaker.decide_targets(game,ctx)}
}
fn resolve_program(game:&mut GameState,source:ObjectId,effects:Vec<Effect>,dm:&mut impl DecisionMaker){execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_decision_maker(dm)).unwrap();}
fn resolve_spell(game:&mut GameState,definition:&CardDefinition,dm:&mut impl DecisionMaker){let source=game.create_object_from_definition(definition,A,Zone::Stack);resolve_program(game,source,definition.spell_effect.as_ref().expect("whole actual spell program").all_effects_owned(),dm);}
#[test]
fn actual_achieve_greatness_chooses_a_tied_least_power_ally_and_places_five_counters(){
 for definition in actual("Achieve Greatness") {
  let mut game=play::game();let strong=game.create_object_from_definition(&body(4),A,Zone::Battlefield);let weak=game.create_object_from_definition(&body(1),A,Zone::Battlefield);let tied=game.create_object_from_definition(&body(1),A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body(0),B,Zone::Battlefield);
  let mut dm=Choices::default();resolve_spell(&mut game,&definition,&mut dm);
  assert!(dm.observed.iter().any(|ids|ids.len()==2&&ids.contains(&weak)&&ids.contains(&tied)),"least-power tie is a choice, not a target: {:?}",dm.observed);
  assert_eq!(game.counter_count(strong,CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(enemy,CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(weak,CounterType::PlusOnePlusOne)+game.counter_count(tied,CounterType::PlusOnePlusOne),5);
 }
}
#[test]
fn actual_resounding_relief_requires_a_same_type_pair_across_graveyards_and_keeps_its_sibling_life(){
 for definition in actual("Resounding Relief") {for matching in [false,true] {
  let mut game=play::game();let first=game.create_object_from_definition(&body(1),A,Zone::Graveyard);let other=if matching{body(2)}else{ironsmith_compiler_runtime::compile_to_runtime_definition("Unmatched instant","Type: Instant\nYou gain 1 life.",false).unwrap()};let second=game.create_object_from_definition(&other,B,Zone::Graveyard);
  let draw=game.create_object_from_definition(&body(1),A,Zone::Library);let draw_stable=game.object(draw).unwrap().stable_id;let first_stable=game.object(first).unwrap().stable_id;let second_stable=game.object(second).unwrap().stable_id;
  resolve_spell(&mut game,&definition,&mut Choices::default());assert_eq!(game.player(A).unwrap().life,23,"independent following life gain");
  for stable in [first_stable,second_stable]{let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,if matching{Zone::Exile}else{Zone::Graveyard});}
  let draw=game.find_object_by_stable_id(draw_stable).unwrap();assert_eq!(game.object(draw).unwrap().zone,if matching{Zone::Hand}else{Zone::Library});
 }}
}
#[test]
fn actual_rousing_roots_distributes_four_and_only_animates_selected_noncreatures(){
 for definition in actual("Rousing Roots") {
  let mut game=play::game();let creature=game.create_object_from_definition(&body(3),A,Zone::Battlefield);let land_def=ironsmith_compiler_runtime::compile_to_runtime_definition("Animation witness","Type: Land",false).unwrap();let land=game.create_object_from_definition(&land_def,A,Zone::Battlefield);let untouched=game.create_object_from_definition(&land_def,A,Zone::Battlefield);let enemy=game.create_object_from_definition(&land_def,B,Zone::Battlefield);
  let mut dm=Choices{wanted:vec![creature,land],..Choices::default()};resolve_spell(&mut game,&definition,&mut dm);
  assert_eq!(game.counter_count(creature,CounterType::PlusOnePlusOne),2);assert_eq!(game.calculated_power(creature),Some(5));assert!(!game.is_tapped(creature));assert_eq!(game.counter_count(land,CounterType::PlusOnePlusOne),2);assert_eq!(game.calculated_power(land),Some(2));assert!(game.is_tapped(land));assert!(game.calculated_card_types(land).contains(&ironsmith::CardType::Land));assert!(game.calculated_card_types(land).contains(&ironsmith::CardType::Creature));
  for id in [untouched,enemy]{assert_eq!(game.counter_count(id,CounterType::PlusOnePlusOne),0);assert!(!game.is_tapped(id));assert!(!game.calculated_card_types(id).contains(&ironsmith::CardType::Creature));}
 }
}
fn activate(game:&mut GameState,source:ObjectId,index:usize,dm:&mut impl DecisionMaker){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let action=ironsmith::decision::LegalAction::ActivateAbility{source,ability_index:index};assert!(ironsmith::decision::compute_legal_actions(game,A).unwrap().contains(&action));let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(game.players_in_game());let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..60 {if state.pending_activation.is_none(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();}assert!(state.pending_activation.is_none());
}
#[test]
fn actual_anomalion_activated_permutate_removes_a_nonplus_counter_and_excludes_the_donor(){
 for definition in actual("Anomalion") {for accepted in [false,true] {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let initial_source_counters=game.counter_count(source,CounterType::PlusOnePlusOne);let donor=game.create_object_from_definition(&body(1),A,Zone::Battlefield);let recipient=game.create_object_from_definition(&body(2),A,Zone::Battlefield);game.add_counters(donor,CounterType::Flying,1);
  play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::Green,3);play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::White,3);
  let index=play::activated_at(&definition,0);let mut dm=Choices{answers:VecDeque::from(vec![accepted]),wanted:vec![donor,recipient],..Choices::default()};activate(&mut game,source,index,&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.counter_count(donor,CounterType::Flying),u32::from(!accepted));assert_eq!(game.counter_count(donor,CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(recipient,CounterType::PlusOnePlusOne),if accepted{2}else{0});assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),initial_source_counters);
 }}
}
#[test]
fn actual_vetas_loyalty_augur_preserves_the_viewed_card_across_all_optional_outcomes(){
 for definition in actual("Vetas of Distant Groves") {for (sacrifice,graveyard) in [(true,false),(false,false),(false,true)] {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let donor=game.create_object_from_definition(&body(1),A,Zone::Battlefield);let viewed=game.create_object_from_definition(&body(2),A,Zone::Library);let viewed_stable=game.object(viewed).unwrap().stable_id;let donor_stable=game.object(donor).unwrap().stable_id;
  let mut dm=Choices{answers:VecDeque::from(if sacrifice{vec![true]}else{vec![false,graveyard]}),..Choices::default()};activate(&mut game,source,play::activated_at(&definition,0),&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let viewed=game.find_object_by_stable_id(viewed_stable).unwrap();assert_eq!(game.object(viewed).unwrap().zone,if sacrifice{Zone::Hand}else if graveyard{Zone::Graveyard}else{Zone::Library});let donor=game.find_object_by_stable_id(donor_stable).unwrap();assert_eq!(game.object(donor).unwrap().zone,if sacrifice{Zone::Graveyard}else{Zone::Battlefield});assert_eq!(game.counter_count(source,CounterType::Loyalty),4,"real +1 loyalty cost still paid");
 }}
}

#[test]
fn actual_muses_whims_draws_two_before_the_counter_or_treasure_choice() {
 for definition in actual("Muse's Whims") {for accepted in [false,true] {for available in [false,true] {
  let mut game=play::game();let recipient=available.then(||game.create_object_from_definition(&body(1),A,Zone::Battlefield));
  game.create_object_from_definition(&body(1),A,Zone::Library);game.create_object_from_definition(&body(2),A,Zone::Library);
  let mut dm=Choices{answers:VecDeque::from(vec![accepted]),..Choices::default()};resolve_spell(&mut game,&definition,&mut dm);
  assert_eq!(game.player(A).unwrap().hand.len(),2,"whole printed draw sibling is neither lost nor repeated");
  assert_eq!(recipient.map(|id|game.counter_count(id,CounterType::PlusOnePlusOne)).unwrap_or(0),u32::from(accepted&&available));
  let treasures=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.subtypes.contains(&ironsmith::Subtype::Treasure)).count();
  assert_eq!(treasures,usize::from(!accepted||!available),"exactly one native fallback treasure");
 }}}
}

#[test]
fn actual_battle_acumen_recomputes_the_least_power_after_each_of_three_instructions(){
 for definition in actual("Battle Acumen") {
  let mut game=play::game();let first=game.create_object_from_definition(&body(0),A,Zone::Battlefield);let second=game.create_object_from_definition(&body(1),A,Zone::Battlefield);let strong=game.create_object_from_definition(&body(10),A,Zone::Battlefield);
  resolve_spell(&mut game,&definition,&mut Choices::default());
  assert_eq!(game.counter_count(first,CounterType::PlusOnePlusOne),2);assert_eq!(game.counter_count(second,CounterType::PlusOnePlusOne),1);assert_eq!(game.counter_count(strong,CounterType::PlusOnePlusOne),0);
 }
}
#[test]
fn actual_blades_of_the_many_binds_its_local_x_to_the_current_friendly_creature_count(){
 for definition in actual("Blades of the Many") {
  let mut game=play::game();let least=game.create_object_from_definition(&body(0),A,Zone::Battlefield);let other=game.create_object_from_definition(&body(3),A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body(0),B,Zone::Battlefield);
  resolve_spell(&mut game,&definition,&mut Choices::default());assert_eq!(game.counter_count(least,CounterType::PlusOnePlusOne),2);assert_eq!(game.counter_count(other,CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(enemy,CounterType::PlusOnePlusOne),0);
 }
}
#[test]
fn actual_so_much_to_do_keeps_each_crystallize_choice_independent_of_the_draw_sibling(){
 for definition in actual("So Much to Do") {
  let mut game=play::game();let recipient=game.create_object_from_definition(&body(1),A,Zone::Battlefield);game.create_object_from_definition(&body(1),A,Zone::Library);game.create_object_from_definition(&body(2),A,Zone::Library);
  let mut dm=Choices{answers:VecDeque::from(vec![true,false]),..Choices::default()};resolve_spell(&mut game,&definition,&mut dm);
  assert_eq!(game.player(A).unwrap().hand.len(),2,"the twice suffix repeats crystallize, not drawing");assert_eq!(game.counter_count(recipient,CounterType::PlusOnePlusOne),1);let treasures=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.subtypes.contains(&ironsmith::Subtype::Treasure)).count();assert_eq!(treasures,1);assert!(dm.answers.is_empty(),"one independent choice per repetition");
 }
}
