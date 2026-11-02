//! Real Planechase transitions through independently compiled source routes.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
use ironsmith::{GameState,PlayerId,ObjectId,PlanarCardKind};
use ironsmith::events::{KeywordActionEvent,KeywordActionKind};
fn definitions(name:&str,observers:bool)->[ironsmith::CardDefinition;2] {
 let rules=if observers {"Type: Plane\nWhen you planeswalk to this plane, you gain 1 life.\nWhen you planeswalk away from this plane, you gain 2 life."}else{"Type: Plane"};
 source_routes::definitions(&serde_json::json!({"name":name,"executable_text":rules})).unwrap()
}
fn named(game:&GameState,name:&str)->ObjectId {
 game.planechase.as_ref().unwrap().card_kinds.keys().copied().find(|id|game.object(*id).is_some_and(|object|object.name==name)).unwrap()
}
fn top(game:&mut GameState,player:PlayerId,id:ObjectId) {
 let deck=game.planechase.as_mut().unwrap().decks.get_mut(&player).unwrap();let index=deck.iter().position(|object|*object==id).unwrap();deck.remove(index);deck.push(id);
}
#[test]
fn planeswalk_direction_observers_use_actual_transition_receipts_on_both_routes() {
 for route in 0..2 {
  let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let alice=PlayerId(0);let bob=PlayerId(1);
  let alice_deck=(0..10).map(|index|(definitions(&format!("Directional plane {index}"),true)[route].clone(),PlanarCardKind::Plane)).collect();
  let bob_deck=(0..10).map(|index|(definitions(&format!("Bob plane {index}"),false)[route].clone(),PlanarCardKind::Plane)).collect();
  game.enable_planechase(vec![(alice,alice_deck),(bob,bob_deck)]).unwrap();
  let old=named(&game,"Directional plane 0");let destination=named(&game,"Directional plane 1");top(&mut game,alice,old);
  let active=game.reveal_starting_plane().unwrap();assert!(game.take_pending_trigger_events().is_empty(),"initial revelation is not a planeswalk");
  top(&mut game,alice,destination);let actual_destination=game.planeswalk(alice,active).unwrap();
  let events=game.take_pending_trigger_events();let event=events.iter().find(|event|event.downcast::<KeywordActionEvent>().is_some_and(|action|action.action==KeywordActionKind::Planeswalk)).unwrap();let receipt=event.downcast::<KeywordActionEvent>().unwrap();
  assert_eq!(receipt.source,active,"initiating source is not rewritten to destination");
  assert_eq!(receipt.object_tags[&ironsmith::tag::TagKey::from(ironsmith::tag::PLANESWALK_DESTINATION_TAG)][0].object_id,actual_destination);
  assert_eq!(receipt.object_tags[&ironsmith::tag::TagKey::from(ironsmith::tag::PLANESWALK_ORIGINS_TAG)][0].object_id,active);
  let entries=ironsmith::triggers::check_triggers(&game,event);assert_eq!(entries.len(),2,"only old-plane away and actual-new-plane to observers trigger; dormant unrelated planes do not");
  assert!(entries.iter().any(|entry|entry.source_name=="Directional plane 0"));assert!(entries.iter().any(|entry|entry.source_name=="Directional plane 1"));
 }
}
