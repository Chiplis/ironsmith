//! Source-owned externally invoked programs keep selection, stack and LKI semantics.
use ironsmith::{GameState,PlayerId,Zone,ObjectId};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
#[path="custom_final_next_wave_support.rs"] mod source_support;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-death-knell-complete-source.json")).unwrap()}
fn card<'a>(fixture:&'a serde_json::Value,name:&str)->&'a serde_json::Value {fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]==name).unwrap()}
#[test]
fn all_twenty_seven_exact_source_records_compile_through_independent_routes() {
 let fixture=fixture();let cards=fixture["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),27);
 let mut keys=std::collections::BTreeSet::new();let mut errors=Vec::new();
 for card in cards {
  assert!(keys.insert(card["source_card_key"].as_str().unwrap()));assert_eq!(card["original_text"],card["source_faces"][0]["original_text"]);
  match source_support::linked_definitions(card) {
   Ok(faces)=>for routes in faces {for definition in routes {
    if ironsmith::cards::generated_definition_has_unimplemented_content(&definition) {errors.push(format!("{}: unimplemented content",card["source_card_key"]));}
   }},Err(error)=>errors.push(format!("{}: {error}",card["source_card_key"])),
  }
 }
 assert!(keys.contains("Agent O Reminder_KXP"));assert!(keys.contains("Inconspicuous Otter_KXP"));
 assert!(errors.is_empty(),"{}",errors.join("\n"));
}
struct Pick {wanted:Option<ObjectId>,offers:Vec<(PlayerId,usize)>}
impl DecisionMaker for Pick {
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
  if c.description.contains("death knell triggered program") {
   self.offers.push((c.player,c.options.len()));
   return self.wanted.and_then(|id|c.options.iter().find(|option|option.object_id==Some(id)).map(|option|vec![option.index])).unwrap_or_default();
  }
  SelectFirstDecisionMaker.decide_options(g,c)
 }
}
fn settle(game:&mut GameState,dm:&mut impl DecisionMaker) {
 let mut queue=ironsmith::triggers::TriggerQueue::new();
 for step in 0..60 {
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
  if game.stack_is_empty(){return;}
  assert!(step<59,"source work did not settle");ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
 }
}
fn move_card(game:&mut GameState,id:ObjectId,to:Zone,dm:&mut impl DecisionMaker) {
 let outcome=ironsmith::effects::execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
  ironsmith::target::ChooseSpec::SpecificObject(id),to,false)),&mut ironsmith::effects::EffectContext::new(id,A,dm)).unwrap();
 for event in outcome.events {game.queue_trigger_event(Default::default(),event);}
 settle(game,dm);
}
fn vanilla()->ironsmith::CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition("Vanilla subject","Type: Creature\nPower/Toughness: 2/2",false).unwrap()}
#[test]
fn siphon_student_and_multiple_live_programs_offer_one_choice_with_exact_dying_source_lki() {
 let fixture=fixture();
 for (student,wraith) in source_support::definitions(card(&fixture,"Siphon Student")).unwrap().into_iter()
  .zip(source_support::definitions(card(&fixture,"Guiltless Wraith")).unwrap()) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let student=game.create_object_from_definition(&student,A,Zone::Battlefield);
  game.create_object_from_definition(&wraith,A,Zone::Battlefield);game.create_object_from_definition(&wraith,B,Zone::Battlefield);
  let victim=game.create_object_from_definition(&vanilla(),A,Zone::Battlefield);
  let mut dm=Pick{wanted:Some(student),offers:Vec::new()};move_card(&mut game,victim,Zone::Graveyard,&mut dm);
  assert_eq!(dm.offers,vec![(A,2)],"two source declarations share one event selection; opponent has none");
  assert_eq!(game.player(A).unwrap().life,22);
  dm.offers.clear();move_card(&mut game,student,Zone::Graveyard,&mut dm);
  assert_eq!(dm.offers,vec![(A,2)],"the dead student's original ability remains selectable alongside a live permanent");
  assert_eq!(game.player(A).unwrap().life,24);assert_eq!(game.player(B).unwrap().life,20);
 }
}
#[test]
fn font_of_sorrows_projects_only_the_actual_entrant_program_without_an_extra_choice_or_trigger() {
 let fixture=fixture();
 for (font,student) in source_support::definitions(card(&fixture,"Font of Sorrows")).unwrap().into_iter()
  .zip(source_support::definitions(card(&fixture,"Siphon Student")).unwrap()) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  for _ in 0..3 {game.create_object_from_definition(&vanilla(),A,Zone::Library);}
  let font=game.create_object_from_definition(&font,A,Zone::Hand);
  let mut dm=Pick{wanted:None,offers:Vec::new()};move_card(&mut game,font,Zone::Battlefield,&mut dm);
  assert_eq!(game.player(A).unwrap().hand.len(),1,"Font's own entry triggers its exact draw program once");
  let student=game.create_object_from_definition(&student,A,Zone::Hand);move_card(&mut game,student,Zone::Battlefield,&mut dm);
  assert_eq!(game.player(A).unwrap().life,22,"the other entrant's gain-life program triggers");
  assert_eq!(game.player(A).unwrap().hand.len(),1,"entering another program does not trigger Font's draw program");
  assert!(dm.offers.is_empty(),"these source-authored automatic projections are not optional death selections");
 }
}

#[test]
fn vengeful_sorrow_observes_each_opponent_card_only_during_its_controllers_turn() {
 let fixture=fixture();
 for (sorrow,student) in source_support::definitions(card(&fixture,"Vengeful Sorrow")).unwrap().into_iter()
  .zip(source_support::definitions(card(&fixture,"Siphon Student")).unwrap()) {
  for during_your_turn in [false,true] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=if during_your_turn {A} else {B};
   let sorrow=game.create_object_from_definition(&sorrow,A,Zone::Battlefield);
   let student=game.create_object_from_definition(&student,A,Zone::Battlefield);
   for _ in 0..2 {game.create_object_from_definition(&vanilla(),B,Zone::Library);}
   let mut dm=Pick{wanted:Some(student),offers:Vec::new()};
   let outcome=ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::target_draws(2,ironsmith::target::PlayerFilter::Specific(B)),
    &mut ironsmith::effects::EffectContext::new(sorrow,A,&mut dm)).unwrap();
   for event in outcome.events {game.queue_trigger_event(Default::default(),event);}
   settle(&mut game,&mut dm);
   assert_eq!(dm.offers.len(),if during_your_turn {2} else {0},"a two-card event supplies two independent source-authored program choices");
   assert!(dm.offers.iter().all(|(player,_)|*player==A));
   assert_eq!(game.player(A).unwrap().life,if during_your_turn {24} else {20});
   assert_eq!(game.player(B).unwrap().hand.len(),2);
  }
 }
}
