//! A source-zone designation follows its immediate spell, regardless of cast permission.
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(text:&str)->[CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Zone status",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn mark(game:&mut GameState,definition:&CardDefinition,source:ironsmith::ObjectId){
 let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
 execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
}
#[test]
fn exact_exile_status_follows_only_immediate_spell_successor_on_both_routes(){
 for name in ["cached","archived"]{
  for definition in definitions(&format!("Type: Sorcery\nGive this card the {name} status in exile and on the spell it becomes.")){
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let origin=game.create_object_from_definition(&definition,A,Zone::Exile);mark(&mut game,&definition,origin);
   assert!(game.has_custom_status(origin,name));
   let spell=game.move_object(origin,Zone::Stack,ironsmith::events::EventCause::effect()).unwrap();
   assert!(game.has_custom_status(spell,name),"independent of a selected play permission");
   assert!(!game.has_custom_status(origin,name),"the departed incarnation has no live designation");
   let graveyard=game.move_object(spell,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();
   assert!(!game.has_custom_status(graveyard,name));
   let returned=game.move_object(graveyard,Zone::Exile,ironsmith::events::EventCause::effect()).unwrap();
   let recast=game.move_object(returned,Zone::Stack,ironsmith::events::EventCause::effect()).unwrap();
   assert!(!game.has_custom_status(recast,name),"a later incarnation must not revive the receipt");
  }
 }
}
#[test]
fn other_departures_and_ordinary_zone_statuses_never_lend_spell_designations(){
 for carry in [false,true]{
  let suffix=if carry{" and on the spell it becomes"}else{""};
  for definition in definitions(&format!("Type: Sorcery\nGive this card the archived status in exile{suffix}.")){
   for destination in [Zone::Hand,Zone::Graveyard]{
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let origin=game.create_object_from_definition(&definition,A,Zone::Exile);mark(&mut game,&definition,origin);
    let departed=game.move_object(origin,destination,ironsmith::events::EventCause::effect()).unwrap();
    let returned=game.move_object(departed,Zone::Exile,ironsmith::events::EventCause::effect()).unwrap();
    let spell=game.move_object(returned,Zone::Stack,ironsmith::events::EventCause::effect()).unwrap();
    assert!(!game.has_custom_status(spell,"archived"));
   }
   if !carry{
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);let origin=game.create_object_from_definition(&definition,A,Zone::Exile);mark(&mut game,&definition,origin);
    let spell=game.move_object(origin,Zone::Stack,ironsmith::events::EventCause::effect()).unwrap();assert!(!game.has_custom_status(spell,"archived"));
   }
  }
 }
}
