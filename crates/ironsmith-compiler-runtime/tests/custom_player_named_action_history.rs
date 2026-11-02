//! Completed player-operation history and amount qualifiers.
#[path="custom_player_named_action_history_support.rs"] mod history;
use history::*;
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::PlayerFilter;
use ironsmith_core::value_model::PlayerActionWindow;
#[path="custom_final_next_wave_support.rs"] mod support;
#[test]
fn canonical_player_history_amount_and_contraction_survive_independent_native_and_artifact_routes() {
 for (clause,amount) in [("you have performed the sample forge action this turn",0),("you've performed the sample forge action with an amount of 1 or more this turn",1),("you have performed the sample forge action with an amount of 2 or more this turn",2)] {
  let text=format!("Type: Enchantment\nAt the beginning of your end step, if {clause}, you gain 1 life.");
  for definition in support::definitions(&serde_json::json!({"name":"Player history witness","executable_text":text})).unwrap() {
   let condition=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>trigger.intervening_if.as_ref(),_=>None}).unwrap();
   assert!(matches!(condition,ironsmith_core::Condition::PlayerNamedActionHistory{player:PlayerFilter::You,label,window:PlayerActionWindow::ThisTurn,minimum:1,minimum_amount}if label=="sample forge"&&*minimum_amount==amount));
   if amount>0 {assert!(ironsmith_text::compiled_text_lines(&definition).join("\n").contains(&format!("with an amount of {amount} or more this turn")));}
   else {
    let mut legacy=serde_json::to_value(condition).unwrap();legacy["PlayerNamedActionHistory"].as_object_mut().unwrap().remove("minimum_amount");
    let restored:ironsmith_core::Condition=serde_json::from_value(legacy).unwrap();assert_eq!(&restored,condition,"old unqualified history artifacts default to no amount bound");
   }
  }
 }
}
#[test]
fn exact_completed_receipts_separate_zero_amount_actor_label_turn_combat_and_departed_source() {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let creator=game.create_object_from_definition(&definition("Creator","Type: Artifact"),A,Zone::Battlefield);
 let object=game.create_object_from_definition(&definition("Actual card","Type: Sorcery\nYou gain 1 life."),A,Zone::Hand);
 game.mark_combat_phase_started();game.turn.phase=ironsmith::Phase::Combat;
 let actor=ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,creator).unwrap();let subject=ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,object).unwrap();
 let fake=ironsmith::events::NamedActionEvent::new("sample forge",A,creator,actor,vec![subject],1,ironsmith::events::cause::EventCause::effect());
 let fake=ironsmith::triggers::TriggerEvent::new_with_provenance(fake,Default::default());game.queue_trigger_event(fake.provenance(),fake);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),0,"event-shaped publication cannot fabricate an operation receipt");
 complete(&mut game,creator,A,"sample forge",&[]);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),0,"legal amount-zero completion is not creation of a card");
 complete(&mut game,creator,B,"sample forge",&[object]);complete(&mut game,creator,A,"other forge",&[object]);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),0,"actor is the actual player, not the creator object's controller");
 complete(&mut game,creator,A,"sample forge",&[object]);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),1);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,2),0);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisCombat,1),1);
 game.turn.phase=ironsmith::Phase::NextMain;
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisCombat,1),0,"postcombat main has no current combat");
 assert_eq!(game.player_named_action_count(A,"sample forge",PlayerActionWindow::ThisCombat),0);
 complete(&mut game,creator,A,"sample forge",&[object]);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisCombat,1),0,"a postcombat operation cannot enter the preceding combat");
 game.mark_combat_phase_started();game.turn.phase=ironsmith::Phase::Combat;
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisCombat,1),0,"an extra combat cannot inherit earlier or intervening main-phase actions");
 assert_eq!(game.player_named_action_count(A,"sample forge",PlayerActionWindow::ThisCombat),0);
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),2);
 game.move_object_by_effect(creator,Zone::Graveyard).unwrap();game.move_object_by_effect(object,Zone::Exile).unwrap();
 assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),2,"source and recipient departure cannot erase a player's completed action");
 game.next_turn();assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::ThisTurn,1),0);assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::LastOwnTurn,1),2);
 game.next_turn();assert_eq!(count(&game,A,"sample forge",PlayerActionWindow::LastOwnTurn,1),2);
}
#[test]
fn ordinary_effect_conditions_use_nonzero_completed_player_receipts() {
 let text="Type: Sorcery\nIf you've performed the sample forge action with an amount of 1 or more this turn, you gain 5 life.";
 for spell in support::definitions(&serde_json::json!({"name":"Conditional receipt","executable_text":text})).unwrap() {for actual in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let maker=game.create_object_from_definition(&definition("Creator","Type: Artifact"),A,Zone::Battlefield);
  let object=game.create_object_from_definition(&definition("Created card","Type: Artifact"),A,Zone::Hand);
  complete(&mut game,maker,A,"sample forge",&[]);complete(&mut game,maker,B,"sample forge",&[object]);
  if actual {complete(&mut game,maker,A,"sample forge",&[object]);}
  let source=game.create_object_from_definition(&spell,A,Zone::Stack);let mut context=EffectContext::new_default(source,A);
  for effect in spell.spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut game,effect,&mut context).unwrap();}
  assert_eq!(game.player(A).unwrap().life,if actual{25}else{20});
 }}
}
