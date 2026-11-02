//! Standard card-type transition observations across completed characteristic changes.
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[path="cf8_p08/play.rs"] mod play;
const A:PlayerId=PlayerId(0);
fn definitions(name:&str,text:&str)->[ironsmith::cards::CardDefinition;2]{
 let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap_or_else(|e|panic!("{name}: {e:?}"));
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn drain(game:&mut GameState,expected:usize){
 let mut dm=SelectFirstDecisionMaker;let mut queue=ironsmith::triggers::TriggerQueue::new();
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();
 assert_eq!(queue.entries.len(),expected);
 ironsmith::game_loop::put_triggers_on_stack(game,&mut queue).unwrap();
 while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();}
}
const HOMESTEAD:&str="Mana cost: {3}\nType: Artifact — Vehicle\nPower/Toughness: 1/1\nWhenever Rolling Homestead becomes a creature, you may put a +1/+1 counter on this creature. If you don't, create a 1/1 white Citizen creature token.\nCrew 2";
#[test]
fn rolling_homestead_fullsource_crew_observes_only_actual_noncreature_to_creature_changes(){
 for definition in definitions("Rolling Homestead",HOMESTEAD){
  let mut game=play::game();
  let vehicle=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let mate=definitions("Crewmate","Type: Creature\nPower/Toughness: 2/2").into_iter().next().unwrap();
  let crew=game.create_object_from_definition(&mate,A,Zone::Battlefield);
  game.refresh_continuous_state().unwrap();drain(&mut game,0);
  let index=play::activated_at(&definition,0);
  for expected_counter in [1,1,2]{
   if expected_counter==2{
    ironsmith::turn::execute_cleanup_step(&mut game);game.refresh_continuous_state().unwrap();
    assert!(!game.current_is_creature(vehicle));drain(&mut game,0);
   }
   game.untap(crew);game.untap(vehicle);
   play::activate(&mut game,vehicle,index,&mut play::Script::default());
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
   assert!(game.current_is_creature(vehicle));
   let old=game.counter_count(vehicle,CounterType::PlusOnePlusOne);
   drain(&mut game,usize::from(old<expected_counter));
   assert_eq!(game.counter_count(vehicle,CounterType::PlusOnePlusOne),expected_counter);
  }
 }
}
#[test]
fn entering_as_a_creature_and_repeated_refreshes_are_not_type_gain_events(){
 for definition in definitions("Already animate","Type: Artifact Creature\nPower/Toughness: 1/1\nWhenever this permanent becomes a creature, you gain 1 life."){
  let mut game=play::game();game.create_object_from_definition(&definition,A,Zone::Battlefield);
  for _ in 0..3{game.refresh_continuous_state().unwrap();drain(&mut game,0);}
  assert_eq!(game.player(A).unwrap().life,20);
 }
}
#[test]
fn an_independent_static_type_gain_uses_the_same_completed_transition_owner(){
 for definition in definitions("Rolling Homestead",HOMESTEAD){
  let mut game=play::game();let vehicle=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  game.refresh_continuous_state().unwrap();drain(&mut game,0);
  let animator=definitions("Artifact animator","Type: Enchantment\nArtifacts you control are creatures in addition to their other types.").into_iter().next().unwrap();
  game.create_object_from_definition(&animator,A,Zone::Battlefield);
  game.refresh_continuous_state().unwrap();drain(&mut game,1);
  assert!(game.current_is_creature(vehicle));assert_eq!(game.counter_count(vehicle,CounterType::PlusOnePlusOne),1);
  game.refresh_continuous_state().unwrap();drain(&mut game,0);
 }
}
#[test]
fn the_transition_reader_is_generic_over_card_types(){
 for definition in definitions("Type observer","Type: Artifact\nWhenever this permanent becomes an enchantment, you gain 1 life.\n{0}: This permanent becomes an enchantment in addition to its other types."){
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  game.refresh_continuous_state().unwrap();drain(&mut game,0);
  let index=play::activated_at(&definition,0);
  play::activate(&mut game,source,index,&mut play::Script::default());
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();drain(&mut game,1);
  assert_eq!(game.player(A).unwrap().life,21);
 }
}
