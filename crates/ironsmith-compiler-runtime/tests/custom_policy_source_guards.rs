//! Generic matching-characteristic and frozen targeting-history gates.
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{compute_legal_actions, LegalAction};
use ironsmith::special_actions::SpecialAction;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(condition:&str)->[CardDefinition;2] {
 let text=format!("Type: Enchantment\nSpecial action \"Condition witness\" — Any time you could cast a sorcery, you may pay {{0}} to take a special action. If you do, you gain 1 life. Take this action only if {condition}. Actions of this permanent in group \"test witnesses\" share one use each turn.");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Conditional witness",&text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game()->GameState {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game
}
fn allowed(game:&GameState,source:ObjectId)->bool {
 compute_legal_actions(game,A).unwrap().iter().any(|action|matches!(action,LegalAction::SpecialAction(SpecialAction::PerformIntrinsicAction {source_id,..}) if *source_id==source))
}
fn object(game:&mut GameState,owner:PlayerId,types:&str,pt:&str)->ObjectId {
 let text=format!("Type: {types}\nPower/Toughness: {pt}");let def=ironsmith_compiler_runtime::compile_to_runtime_definition("Characteristic witness",&text,false).unwrap();game.create_object_from_definition(&def,owner,Zone::Battlefield)
}
#[test]
fn equal_characteristic_groups_use_current_values_and_distinct_controlled_objects() {
 for toughness in [false,true] {
  let condition=if toughness {"You control two or more creatures with the same toughness"} else {"You control two or more creatures with the same power"};
  for def in definitions(condition) {
   let mut game=game();let source=game.create_object_from_definition(&def,A,Zone::Battlefield);
   let first=object(&mut game,A,"Creature","2/3");assert!(!allowed(&game,source),"one creature is not a pair");
   let other=object(&mut game,B,"Creature",if toughness {"4/3"} else {"2/4"});assert!(!allowed(&game,source),"opponent's matching creature does not count");
   game.set_current_controller(other,A).unwrap();assert!(allowed(&game,source));
   game.add_counters(first,ironsmith::CounterType::PlusOnePlusOne,1).unwrap();assert!(!allowed(&game,source),"current characteristic changed");
   game.set_current_controller(other,B).unwrap();let equal=object(&mut game,A,"Creature",if toughness {"9/4"} else {"3/9"});assert!(allowed(&game,source));
   game.move_object_by_effect(equal,Zone::Graveyard).unwrap();assert!(!allowed(&game,source),"departed creature cannot complete the pair");
  }
 }
}
fn announce(game:&mut GameState,target:ObjectId,source:ObjectId,controller:PlayerId,ability:bool) {
 let event=ironsmith::events::BecomesTargetedEvent::new(target,source,controller,ability).with_participant_snapshots(game);
 let event=ironsmith::triggers::TriggerEvent::new_with_provenance(event,Default::default());game.turn_store.turn_history.record_event(&event,None,None);
}
#[test]
fn targeting_history_uses_event_actor_kind_and_target_characteristics() {
 for ability in [false,true] {
  let condition=if ability {"You controlled an ability that targeted a creature you controlled this turn"} else {"You controlled a spell that targeted a creature you controlled this turn"};
  for def in definitions(condition) {
   for scenario in 0..7 {
    let mut game=game();let gate=game.create_object_from_definition(&def,A,Zone::Battlefield);
    let target=object(&mut game,if scenario==2 {B}else{A},if scenario==3 {"Artifact"}else{"Creature"},"2/2");
    let source=object(&mut game,B,"Creature","1/1");assert!(!allowed(&game,gate));
    announce(&mut game,target,source,if scenario==1 {B}else{A},if scenario==4 {!ability}else{ability});
    if scenario==2 {game.set_current_controller(target,A).unwrap();}
    if scenario==5 {game.set_current_controller(target,B).unwrap();game.move_object_by_effect(source,Zone::Graveyard).unwrap();game.move_object_by_effect(target,Zone::Graveyard).unwrap();}
    if scenario==6 {game.turn_store.turn_history.clear_for_new_turn();}
    assert_eq!(allowed(&game,gate),matches!(scenario,0|5),"scenario {scenario}, ability {ability}");
   }
  }
 }
}
