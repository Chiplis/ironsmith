//! Exact source Sightsee uses independently chosen real supplied cards.
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
#[path="custom_final_next_wave_support.rs"] mod source_support;
const A: PlayerId = PlayerId(0);
fn fixture() -> serde_json::Value { serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-sightsee-complete-source.json")).unwrap() }
fn card<'a>(fixture: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
 fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card| card["name"]==name).unwrap()
}
#[test]
fn all_eighteen_preserved_sources_compile_independently_and_roundtrip() {
 let fixture=fixture();let cards=fixture["mechanics"][0]["representative_cards"].as_array().unwrap();
 assert_eq!(cards.len(),18);
 let mut keys=std::collections::BTreeSet::new();let mut errors=Vec::new();
 for card in cards {
  assert!(keys.insert(card["source_card_key"].as_str().unwrap()));
  assert!(card["source_export"]["rulesText"].as_str().unwrap().to_lowercase().contains("sightsee"));
  assert_eq!(card["source_faces"][0]["original_text"],card["original_text"]);
  assert_eq!(card["supplied_card_definitions"].as_array().unwrap().len(),12);
  match source_support::linked_definitions(card) {
   Ok(faces)=>for definitions in faces {for definition in definitions {
    if ironsmith::cards::generated_definition_has_unimplemented_content(&definition) {errors.push(format!("{}: unimplemented content",card["name"]));}
   }},
   Err(error)=>errors.push(format!("{}: {error}",card["name"])),
  }
 }
 assert!(errors.is_empty(),"{}",errors.join("\n"));
}
struct Names {wanted:Vec<&'static str>,next:usize}
impl DecisionMaker for Names {
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
  if c.description=="Choose a card name" {
   let name=self.wanted[self.next];self.next+=1;
   return vec![c.options.iter().find(|option|option.description.eq_ignore_ascii_case(name)).expect("complete basic-land menu").index];
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
  assert!(step<59,"source work did not settle");
  ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
 }
}
#[test]
fn sky_scraper_three_independent_choices_allow_repetition_snow_colorless_and_conjure_observers() {
 let fixture=fixture();
 for (sky,storyteller) in source_support::definitions(card(&fixture,"Sky Scraper")).unwrap().into_iter()
  .zip(source_support::definitions(card(&fixture,"Campfire Storyteller")).unwrap()) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&storyteller,A,Zone::Battlefield);
  let sky=game.create_object_from_definition(&sky,A,Zone::Hand);
  let mut dm=Names{wanted:vec!["Snow-Covered Wastes","Island","Snow-Covered Wastes"],next:0};
  let outcome=ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::put_onto_battlefield(
   ironsmith::target::ChooseSpec::SpecificObject(sky),false,ironsmith::target::PlayerFilter::You),
   &mut ironsmith::effects::EffectContext::new(sky,A,&mut dm)).unwrap();
  for event in outcome.events {game.queue_trigger_event(Default::default(),event);}
  settle(&mut game,&mut dm);
  assert_eq!(dm.next,3,"each sightsee is a separate basic-land choice");
  let hand=game.player(A).unwrap().hand.iter().map(|id|game.object(*id).unwrap()).collect::<Vec<_>>();
  assert_eq!(hand.len(),3);assert_eq!(hand.iter().filter(|object|object.name.as_ref()=="Snow-Covered Wastes").count(),2);
  assert_eq!(hand.iter().filter(|object|object.name.as_ref()=="Island").count(),1);
  assert!(hand.iter().all(|object|object.kind==ironsmith::object::ObjectKind::Card && object.owner==A));
  for object in hand.iter().filter(|object|object.name.as_ref()=="Snow-Covered Wastes") {
   assert!(object.supertypes.contains(&ironsmith::types::Supertype::Snow));
   assert!(object.supertypes.contains(&ironsmith::types::Supertype::Basic));
   assert!(object.colors.is_empty());
  }
  assert_eq!(game.battlefield.iter().filter(|id|game.object(**id).unwrap().kind==ironsmith::object::ObjectKind::Token).count(),3,
   "each independent land-conjure action triggers the exact Campfire source once");
 }
}
#[test]
fn finite_card_name_operands_preserve_quoted_commas_and_allow_a_prior_name_again() {
 let source=serde_json::json!({"name":"Finite name choice", "executable_text":"Type: Sorcery\nChoose \"First, the Named\", or \"Second Name\" as a card name."});
 for definition in source_support::definitions(&source).unwrap() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let id=game.create_object_from_definition(&definition,A,Zone::Stack);
  let mut dm=Names{wanted:vec!["First, the Named","Second Name","First, the Named"],next:0};
  for _ in 0..3 {
   let mut ctx=ironsmith::effects::EffectContext::new(id,A,&mut dm);
   for effect in definition.spell_effect.as_ref().unwrap().all_effects_owned() {ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();}
  }
  assert_eq!(dm.next,3);
 }
}
