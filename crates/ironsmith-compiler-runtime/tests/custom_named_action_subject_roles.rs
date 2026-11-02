//! Actor pronouns and explicit affected-card demonstratives have separate roles.
use ironsmith::{GameState,PlayerId,ObjectId,Zone,CardDefinition};
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
struct Yes;impl DecisionMaker for Yes{fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}}
fn definitions(name:&str,text:&str)->[CardDefinition;2]{
 let (compiled,direct_loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(),name),text.to_string(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
 assert!(!direct_loss.is_lossy(),"direct: {}",direct_loss.reasons_text());
 let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.unwrap().definition).unwrap();
 let(result,artifact_loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
 assert!(!artifact_loss.is_lossy(),"artifact: {}",artifact_loss.reasons_text());let(artifact,_artifact_materialized)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn perform(game:&mut GameState,source:ObjectId,object:ObjectId,label:&str){
 perform_expected(game,source,object,label,1);
}
fn perform_expected(game:&mut GameState,source:ObjectId,object:ObjectId,label:&str,expected:usize){
 let effect=ironsmith::Effect::new(ironsmith::effects::PerformNamedActionEffect{label:label.into(),performer:None,objects:Some(ChooseSpec::SpecificObject(object)),prior_objects:None});
 let outcome=execute_effect(game,&effect,&mut EffectContext::new_default(source,A)).unwrap();
 let events=outcome.events.into_iter().filter(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).collect::<Vec<_>>();assert_eq!(events.len(),1);
 for event in events{game.queue_trigger_event(event.provenance(),event);}
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=Yes;
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();assert_eq!(queue.entries.len(),expected,"the independently emitted receipt must respect the affected-card filter");
 ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();}
}
fn object(game:&mut GameState,text:&str)->ObjectId{let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Affected card",text,false).unwrap();game.create_object_from_definition(&definition,A,Zone::Library)}
#[test]
fn exact_vessel_and_mechanomind_sources_keep_actor_and_affected_card_distinct(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/named_action_subject_roles.json.fixture")).unwrap();
 for card in fixture["cards"].as_array().unwrap(){for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  if card["source_card_key"]=="Vessel of Lost Souls_HI12"{
   let donor=object(&mut game,"Mana cost: {5}{U}\nType: Creature — Bird\nPower/Toughness: 7/9\nFlying");perform(&mut game,source,donor,"envision");
   let chars=game.calculated_characteristics(source).unwrap();assert_eq!((chars.power,chars.toughness),(Some(7),Some(9)),"it is the actor, that card is the distinct copy donor");assert_eq!(game.object(donor).unwrap().zone,Zone::Library);
  }else{
   let donor=object(&mut game,"Type: Artifact");perform(&mut game,source,donor,"envision");
   assert_eq!(game.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),1);assert_eq!(game.counter_count(donor,ironsmith::CounterType::PlusOnePlusOne),0);
  }
 }}
}
#[test]
fn player_subject_keeps_affected_object_it_reference(){
 for definition in definitions("Player watcher","Type: Artifact\nWhenever you perform the sample action with an artifact card, put a +1/+1 counter on it."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let donor=object(&mut game,"Type: Artifact");perform(&mut game,source,donor,"sample");
  assert_eq!(game.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(donor,ironsmith::CounterType::PlusOnePlusOne),1);
 }
}

#[test]
fn named_action_card_filters_match_retained_snapshots_across_zones() {
 for definition in definitions("Cross-zone watcher", "Type: Artifact\nWhenever you perform the sample action with an artifact card, put a +1/+1 counter on it.") {
  for zone in [Zone::Library, Zone::Hand, Zone::Graveyard, Zone::Exile] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   let donor_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Affected card","Type: Artifact",false).unwrap();
   let donor=game.create_object_from_definition(&donor_definition,A,zone);
   perform_expected(&mut game,source,donor,"sample",1);
   assert_eq!(game.counter_count(donor,ironsmith::CounterType::PlusOnePlusOne),1,"{zone:?}");
  }
 }
}
#[test]
fn named_action_card_filters_preserve_explicit_zone_and_type_restrictions() {
 for definition in definitions("Restricted watcher", "Type: Artifact\nWhenever you perform the sample action with an artifact card in your graveyard, put a +1/+1 counter on it.") {
  for (zone,card_type,expected) in [(Zone::Library,"Artifact",0),(Zone::Graveyard,"Artifact",1),(Zone::Graveyard,"Creature",0)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   let text=format!("Type: {card_type}");
   let donor_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Affected card",&text,false).unwrap();
   let donor=game.create_object_from_definition(&donor_definition,A,zone);
   perform_expected(&mut game,source,donor,"sample",expected);
   assert_eq!(game.counter_count(donor,ironsmith::CounterType::PlusOnePlusOne),expected as u32);
  }
 }
}
