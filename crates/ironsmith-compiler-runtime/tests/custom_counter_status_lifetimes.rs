//! Generic designation lifetime bound to the first absence of a counter kind.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(text:&str)->[CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Counter designation",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(restored,artifact);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn apply(game:&mut GameState,definition:&CardDefinition,source:ironsmith::ObjectId){
 let effects=definition.abilities.iter().find_map(|a|if let ironsmith::ability::AbilityKind::Activated(a)=&a.kind{Some(a.effects.all_effects_owned())}else{None}).unwrap();
 execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
}
#[test]
fn designation_expires_at_first_counter_absence_and_never_revives_from_later_counters(){
 for definition in definitions("Type: Creature\nPower/Toughness: 2/2\n{0}: Give this creature the sovereign status until it has no +1/+1 counters."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  apply(&mut game,&definition,source);assert!(!game.has_custom_status(source,"sovereign"));
  game.add_counters(source,CounterType::PlusOnePlusOne,2).unwrap();apply(&mut game,&definition,source);assert!(game.has_custom_status(source,"sovereign"));
  game.remove_counters(source,CounterType::PlusOnePlusOne,1,None,None).unwrap();assert!(game.has_custom_status(source,"sovereign"));
  game.remove_counters(source,CounterType::PlusOnePlusOne,1,None,None).unwrap();assert!(!game.has_custom_status(source,"sovereign"));
  game.add_counters(source,CounterType::PlusOnePlusOne,1).unwrap();assert!(!game.has_custom_status(source,"sovereign"));
  apply(&mut game,&definition,source);assert!(game.has_custom_status(source,"sovereign"));
  let hand=game.move_object(source,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();let returned=game.move_object(hand,Zone::Battlefield,ironsmith::events::EventCause::effect()).unwrap();assert!(!game.has_custom_status(returned,"sovereign"));
 }
}
#[test]
fn permanent_application_outlives_a_separate_counter_duration_and_prevention_leaves_no_marker(){
 use ironsmith::target::ObjectFilter;
 for definition in definitions("Type: Creature\nPower/Toughness: 2/2\n{0}: Put two +1/+1 counters on this creature. Give this creature the sovereign status until it has no +1/+1 counters."){
  for prevent in [false,true]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   if prevent{execute_effect(&mut game,&Effect::new(ironsmith::effects::CantEffect::new(ironsmith::effect::Restriction::have_counters_placed(ObjectFilter::specific(source)),ironsmith::effect::Until::EndOfTurn)),&mut EffectContext::new_default(source,A)).unwrap();game.update_cant_effects();}
   apply(&mut game,&definition,source);assert_eq!(game.has_custom_status(source,"sovereign"),!prevent);
   if !prevent{game.apply_custom_status(source,"sovereign",false);game.remove_counters(source,CounterType::PlusOnePlusOne,2,None,None).unwrap();assert!(game.has_custom_status(source,"sovereign"),"a distinct permanent application still holds");}
  }
 }
}
