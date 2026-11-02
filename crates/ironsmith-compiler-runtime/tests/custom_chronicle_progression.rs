//! Native chapter-clock declarations preserve ordinary Chronicle identity.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,PlayerId,ObjectId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::triggers::TriggerQueue;
use ironsmith::game_loop::{add_saga_lore_counters,drain_pending_trigger_events_with_dm,put_triggers_on_stack_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn source()->serde_json::Value {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-chronicle.json")).unwrap();fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["source_card_key"]=="Astral Capricine_TTR").unwrap().clone()
}
fn definitions()->[CardDefinition;2] {let card=source();assert!(card["custom_keyword_identities"].as_array().unwrap().iter().any(|name|name=="chronicle"));let definitions=source_support::definitions(&card).unwrap();for definition in &definitions {assert!(definition.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.custom_keyword_name()==Some("chronicle"))));}definitions}
fn settle(game:&mut GameState,queue:&mut TriggerQueue) {
 drain_pending_trigger_events_with_dm(game,queue,&mut SelectFirstDecisionMaker).unwrap();put_triggers_on_stack_with_dm(game,queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}
}
fn enter(game:&mut GameState,definition:&CardDefinition)->ObjectId {
 let card=game.create_object_from_definition(definition,A,Zone::Hand);let receipt=game.move_object_with_etb_processing_with_dm(card,Zone::Battlefield,&mut SelectFirstDecisionMaker).unwrap();assert!(!receipt.pending);receipt.original.into_result().unwrap().new_id
}
#[test]
fn exact_chronicle_entry_and_native_main_progression_reset_after_final_chapter_without_saga_sacrifice() {
 for definition in definitions() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;let source=enter(&mut game,&definition);let mut queue=TriggerQueue::new();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),1);assert!(game.stack.is_empty());settle(&mut game,&mut queue);
  assert!(!game.current_subtypes(source).unwrap().contains(&ironsmith::Subtype::Saga));
  game.turn.phase=ironsmith::game_state::Phase::FirstMain;add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),2);assert!(game.stack.is_empty(),"the lore action itself does not use the stack");assert_eq!(queue.entries.len(),1,"only the chapter is pending");settle(&mut game,&mut queue);
  game.turn.active_player=B;add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),2);game.turn.active_player=A;
  add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),3);let pending=queue.entries.len();add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),3);assert_eq!(queue.entries.len(),pending);
  assert!(!ironsmith::rules::state_based::check_state_based_actions(&game).iter().any(|action|matches!(action,ironsmith::rules::state_based::StateBasedAction::SagaSacrifice(id) if *id==source)));
  settle(&mut game,&mut queue);assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),0);assert_eq!(game.battlefield.len(),3,"the full final chapter creates its two Goat tokens");assert!(game.object(source).is_some());
  add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),1);
 }
}
#[test]
fn native_clock_reads_live_permission_current_controller_and_stops_when_final_chapter_is_countered() {
 for definition in definitions() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=enter(&mut game,&definition);let mut queue=TriggerQueue::new();settle(&mut game,&mut queue);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  let original=game.object(source).unwrap().abilities.clone();std::sync::Arc::make_mut(&mut game.object_mut(source).unwrap().abilities).retain(|ability|!matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if matches!(ability.keyword_marker_mechanic(),Some(ironsmith_core::KeywordMarkerMechanic::PrecombatLoreProgression{..}))));game.refresh_continuous_state().unwrap();add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),1);
  game.object_mut(source).unwrap().abilities=original;game.set_current_controller(source,B).unwrap();game.refresh_continuous_state().unwrap();game.turn.active_player=A;add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),1);
  game.turn.active_player=B;add_saga_lore_counters(&mut game,&mut queue).unwrap();settle(&mut game,&mut queue);add_saga_lore_counters(&mut game,&mut queue).unwrap();put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert!(!game.stack.is_empty());game.stack.clear();add_saga_lore_counters(&mut game,&mut queue).unwrap();assert_eq!(game.counter_count(source,ironsmith::CounterType::Lore),3);assert!(queue.entries.is_empty());assert!(game.object(source).is_some());
 }
}
#[test]
fn every_complete_chronicle_source_compiles_losslessly_through_independent_direct_and_artifact_routes() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-chronicle.json")).unwrap();let cards=fixture["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),19);let failures:Vec<_>=cards.iter().filter_map(|card|source_support::definitions(card).err().map(|error|format!("{}: {error}",card["source_card_key"]))).collect();assert!(failures.is_empty(),"{}",failures.join("\n"));
}
