//! Generic real-card creation: source-owned definitions, typed receipts, and artifact transport.
use ironsmith::{GameState,PlayerId,Zone,CardDefinition,ObjectId};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
const B:PlayerId=PlayerId(1);
fn pool()->Vec<ironsmith_compiler::CardDefinition>{
 [("Fatigue","Mana cost: {2}\nType: Sorcery\nDraw a card."),("Mountain","Type: Basic Land — Mountain\n{T}: Add {R}."),("Plains","Type: Basic Land — Plains\n{T}: Add {W}."),("Swamp","Type: Basic Land — Swamp\n{T}: Add {B}.")].into_iter().map(|(name,text)|ironsmith_compiler::compile_supplied_card_definition(name,text,&[],&[],&[]).unwrap()).collect()
}
fn routes(text:&str)->[CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::with_supplied_card_definitions(pool(),||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Generic supplied creator",text,false))));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn resolve(game:&mut GameState,definition:&CardDefinition)->Vec<ironsmith::triggers::TriggerEvent>{
 let source=game.create_object_from_definition(definition,A,Zone::Stack);let mut context=EffectContext::new_default(source,A);let mut events=Vec::new();
 for effect in definition.spell_effect.as_ref().expect("spell program").flattened_default_effects(){let result=execute_effect(game,effect,&mut context).unwrap();events.extend(result.events);}
 events
}
fn names(game:&GameState,player:PlayerId)->Vec<String>{game.player(player).unwrap().hand.iter().map(|id|game.object(*id).unwrap().name.clone()).collect()}
#[test]
fn named_creation_uses_supplied_custom_fatigue_as_real_cards_and_one_exact_batch_receipt(){
 for definition in routes("Type: Sorcery\nCreate three real cards named \"Fatigue\" in your hand as part of the named call action."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let events=resolve(&mut game,&definition);assert_eq!(names(&game,A),vec!["Fatigue";3]);
  for id in &game.player(A).unwrap().hand {let card=game.object(*id).unwrap();assert_eq!(card.kind,ironsmith::object::ObjectKind::Card);assert_eq!(card.owner,A);let program=format!("{:?}",card.spell_effect);assert!(program.contains("DrawCardsEffect"),"custom Fatigue draws instead of official same-named skip-draw spell");}
  let actions=events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label=="call").collect::<Vec<_>>();assert_eq!(actions.len(),1);assert_eq!(actions[0].amount,3);assert_eq!(actions[0].objects.len(),3);
 }
}
#[test]
fn distinct_named_children_are_one_operation_with_union_of_created_identities(){
 for definition in routes("Type: Sorcery\nCreate a real card for each name \"Mountain\", \"Plains\", \"Swamp\" in your hand as part of the named call action."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let events=resolve(&mut game,&definition);let mut actual=names(&game,A);actual.sort();assert_eq!(actual,vec!["Mountain","Plains","Swamp"]);
  let actions=events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label=="call").collect::<Vec<_>>();assert_eq!(actions.len(),1);assert_eq!(actions[0].amount,3);assert_eq!(actions[0].objects.len(),3);
 }
}
#[test]
fn each_actor_owns_their_created_cards_and_their_separate_receipt(){
 for definition in routes("Type: Sorcery\nFor each player, do the following: [Create two real cards named \"Fatigue\" in that player's hand as part of the named call action]."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let events=resolve(&mut game,&definition);assert_eq!(names(&game,A).len(),2);assert_eq!(names(&game,B).len(),2);
  let actions=events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label=="call").collect::<Vec<_>>();assert_eq!(actions.len(),2);
  for actor in [A,B]{let event=actions.iter().find(|event|event.player==actor).unwrap();assert_eq!(event.amount,2);assert!(event.objects.iter().all(|snapshot|snapshot.owner==actor));}
 }
}
#[test]
fn absent_supplied_named_definition_is_a_compile_error_even_for_an_official_name(){
 let error=ironsmith_compiler::with_supplied_card_definitions(Vec::new(),||ironsmith_compiler_runtime::compile_to_artifact("No fallback","Type: Sorcery\nCreate a real card named \"Fatigue\" in your hand.",false)).unwrap_err();assert!(error.to_string().contains("supplied"));
}
#[test]
fn chosen_name_creation_requires_an_actual_name_choice_producer(){
 let text="Type: Sorcery\nCreate a real card with the chosen name in your hand.";
 let error=ironsmith_compiler::with_supplied_card_definitions(pool(),||ironsmith_compiler_runtime::compile_to_artifact("No fabricated name",text,false)).unwrap_err();assert!(error.to_string().contains("card-name choice producer"));
}
#[path="custom_final_next_wave_support.rs"]mod source_support;
fn source_fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-conjure-source-complete.json")).unwrap()}
fn source_routes(name:&str)->[CardDefinition;2]{let data=source_fixture();let card=data["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]==name).unwrap();source_support::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))}
#[test]
fn all_103_actual_sources_and_all_38_target_definitions_compile_on_both_routes(){
 let data=source_fixture();let cards=data["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),103);let records=cards[0]["supplied_card_definitions"].as_array().unwrap();assert_eq!(records.len(),38);
 let mut failures=Vec::new();for card in cards{if let Err(error)=source_support::definitions(card){failures.push(format!("{}: {error}",card["name"]));}}assert!(failures.is_empty(),"{}",failures.join("\n"));
}
#[test]
fn actual_moonlit_lantern_source_creates_all_three_real_lands_with_one_conjure_event(){
 for definition in source_routes("Moonlit Lantern Festival"){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let events=resolve(&mut game,&definition);let mut actual=names(&game,A);actual.sort();assert_eq!(actual,vec!["Mountain","Plains","Swamp"]);
  let actions=events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label=="conjure").collect::<Vec<_>>();assert_eq!(actions.len(),1);assert_eq!(actions[0].amount,3);
 }
}

#[test]
fn actual_batch_observers_fire_once_and_use_created_card_count_on_both_routes(){
 let creators=source_routes("Moonlit Lantern Festival");let highrollers=source_routes("Enigmatic Highroller");let compositions=source_routes("Heartfelt Composition");
 for route in 0..2 {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&highrollers[route],A,Zone::Battlefield);game.create_object_from_definition(&compositions[route],A,Zone::Battlefield);
  for event in resolve(&mut game,&creators[route]){game.queue_trigger_event(Default::default(),event);}
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(queue.entries.len(),2,"one completion, one trigger peractual observer");
  ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  let treasures=game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.kind==ironsmith::object::ObjectKind::Token)&&game.current_subtypes(**id).is_some_and(|types|types.contains(&ironsmith::types::Subtype::Treasure))).count();assert_eq!(treasures,3,"amount is number of real cards, not number of child creation nodes");
  let elementals=game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.kind==ironsmith::object::ObjectKind::Token)&&game.current_subtypes(**id).is_some_and(|types|types.contains(&ironsmith::types::Subtype::Elemental))).count();assert_eq!(elementals,1,"multi-identity batch is one Conjure operation");
 }
}
