//! Successful optional resolution payments create source-bound named receipts.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices(bool);
impl DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
}
fn definitions()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-tithe-permanent-next.json")).unwrap();let card=&fixture["mechanics"][0]["representative_cards"][0];
 let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),card["name"].as_str().unwrap()).with_custom_keyword("tithe");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,card["executable_text"].as_str().unwrap(),false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn flush(game:&mut GameState,dm:&mut Choices)->usize{
 let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();let count=queue.entries.len();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();count
}
#[test]
fn optional_entry_payment_emits_only_after_exact_other_permanents_are_tapped(){
 for definition in definitions(){for accept in [false,true]{for enough in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let mut dm=Choices(accept);
  let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Payment artifact","Type: Artifact",false).unwrap();
  let first=game.create_object_from_definition(&artifact,A,Zone::Battlefield);let second=game.create_object_from_definition(&artifact,A,Zone::Battlefield);let third=if enough{Some(game.create_object_from_definition(&artifact,A,Zone::Battlefield))}else{None};
  let opponent=game.create_object_from_definition(&artifact,B,Zone::Battlefield);let tapped=game.create_object_from_definition(&artifact,A,Zone::Battlefield);game.tap(tapped);
  let other_watcher=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.tap(other_watcher);
  let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Graveyard spell","Type: Instant\nYou gain 1 life.",false).unwrap();let grave=game.create_object_from_definition(&spell,A,Zone::Graveyard);let grave_stable=game.object(grave).unwrap().stable_id;
  let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
  let result=execute_effect(&mut game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Battlefield,false)),&mut EffectContext::new_default(source,A)).unwrap();for event in result.events{game.queue_trigger_event(Default::default(),event);}
  assert_eq!(flush(&mut game,&mut dm),1);assert_eq!(game.stack.len(),1,"only entering source has an entry trigger");
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let paid=accept&&enough;let permanent=game.find_object_by_stable_id(stable).unwrap();assert!(!game.is_tapped(permanent),"the entering source cannot pay its own tithe");
  assert_eq!(game.is_tapped(first),paid);assert_eq!(game.is_tapped(second),paid);if let Some(third)=third{assert_eq!(game.is_tapped(third),paid);}assert!(!game.is_tapped(opponent));assert!(game.is_tapped(tapped));
  flush(&mut game,&mut dm);assert_eq!(game.stack.len(),usize::from(paid),"only successful exact payment emits one source-bound watcher trigger");
  if paid{ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  let result=game.find_object_by_stable_id(grave_stable).unwrap();assert_eq!(game.object(result).unwrap().zone,if paid{Zone::Hand}else{Zone::Graveyard});
 }}}
}
