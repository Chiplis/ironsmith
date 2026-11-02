//! C747 work selection resolves into noncopiable named permanent designations.
//! Full source corpus is a separate strict contract below, never a prose score.
use ironsmith::decision::{compute_legal_actions,LegalAction};
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::game_loop::resolve_stack_entry_with;
#[path="cf8_p08/support.rs"] mod support;
#[path="cf8_p08/play.rs"] mod play;
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-gallery.json")).unwrap()}
fn definitions(card:&serde_json::Value)->[CardDefinition;2] {
 strict_routes::definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"]))
}
fn legal(game:&GameState,player:PlayerId,source:ObjectId,index:usize)->bool {
 compute_legal_actions(game,player).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}|LegalAction::ActivateManaAbility{source:id,ability_index} if *id==source&&*ability_index==index))
}
fn control()->Vec<CardDefinition>{definitions(&fixture()["behavioral_controls"][0]).to_vec()}
#[test]
fn every_complete_source_gallery_and_its_work_sections_compile_strictly() {
 for card in fixture()["mechanics"][0]["representative_cards"].as_array().unwrap(){let _=definitions(card);}
}
#[test]
fn unlocking_is_targetless_and_locks_on_resolution_instead_of_announcement() {
 for definition in control(){
  let mut game=play::game();let source=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);
  let first=play::activated_at(&definition,1);let second=play::activated_at(&definition,2);
  assert!(legal(&game,play::A,source,first),"granting a target-requiring ability does not target anything while unlocking");
  let mut dm=play::Script::default();play::activate(&mut game,source,first,&mut dm);
  assert!(!game.has_custom_status(source,"unlocked work"));
  assert!(!game.has_custom_status(source,"unlocked work named portrait: dawn"));
  game.stack.pop(); // Countering the ability prevents the unlock designation.
  assert!(legal(&game,play::A,source,second));
  play::activate(&mut game,source,second,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert!(game.has_custom_status(source,"unlocked work"));
  assert!(game.has_custom_status(source,"unlocked work named canvas, dusk"),"actual statuses: {:?}",game.custom_status_names(source));
  assert!(!game.has_custom_status(source,"unlocked work named portrait: dawn"));
  assert!(!legal(&game,play::A,source,first));assert!(!legal(&game,play::A,source,second));
  let base=play::activated_at(&definition,0);assert!(legal(&game,play::A,source,base),"top-box ability remains normal");
  play::activate(&mut game,source,base,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(play::A).unwrap().life,23);
 }
}
#[test]
fn unlocked_work_grants_survive_type_control_changes_but_do_not_copy() {
 for definition in control(){
  let mut game=play::game();let source=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);
  let creature=play::vanilla("Witness","{0}","Soldier",2,2);
  let a=game.create_object_from_definition(&creature,play::A,Zone::Battlefield);let b=game.create_object_from_definition(&creature,play::B,Zone::Battlefield);
  assert_eq!(game.current_characteristics(a).unwrap().power,Some(2));
  let second=play::activated_at(&definition,2);let mut dm=play::Script::default();play::activate(&mut game,source,second,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.current_characteristics(a).unwrap().power,Some(3));assert_eq!(game.current_characteristics(b).unwrap().power,Some(2));
  let snapshot=game.object(source).unwrap().clone();let copy=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);game.object_mut(copy).unwrap().copy_copiable_values_from(&snapshot);
  assert!(!game.has_custom_status(copy,"unlocked work"));assert!(legal(&game,play::A,copy,second));
  assert_eq!(game.current_characteristics(a).unwrap().power,Some(3),"copying a Gallery must not copy its unlocked work or grant a second anthem");
  {let object=game.object_mut(source).unwrap();object.card_types=vec![ironsmith::CardType::Artifact].into();object.subtypes=Vec::new().into();}
  assert!(game.has_custom_status(source,"unlocked work named canvas, dusk"),"actual statuses: {:?}",game.custom_status_names(source));assert_eq!(game.current_characteristics(a).unwrap().power,Some(3));
  game.set_current_controller(source,play::B).unwrap();
  assert_eq!(game.current_characteristics(a).unwrap().power,Some(2));assert_eq!(game.current_characteristics(b).unwrap().power,Some(3));
  game.turn.active_player=play::B;game.turn.priority_player=Some(play::B);
  assert!(!legal(&game,play::B,source,second));
 }
}
#[test]
fn a_new_battlefield_incarnation_has_no_unlocked_work_and_sorcery_timing_is_enforced() {
 for definition in control(){
  let mut game=play::game();let source=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);
  let first=play::activated_at(&definition,1);game.turn.phase=ironsmith::game_state::Phase::Combat;
  assert!(!legal(&game,play::A,source,first));game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  let mut dm=play::Script::default();play::activate(&mut game,source,first,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert!(game.has_custom_status(source,"unlocked work"));
  let exiled=game.move_object_by_effect(source,Zone::Exile).unwrap();let returned=game.move_object_by_effect(exiled,Zone::Battlefield).unwrap();
  assert_ne!(returned,source);assert!(!game.has_custom_status(returned,"unlocked work"));assert!(legal(&game,play::A,returned,first));
 }
}

#[test]
fn quoted_work_labels_preserve_punctuation_in_typed_status_instructions(){
 for definition in control(){
  let names:Vec<_>=support::find_all::<ironsmith_core::SetPermanentDesignationEffect>(&definition).into_iter().filter_map(|effect|match effect.designation{ironsmith_core::PermanentDesignation::CustomStatus{name,..}=>Some(name),_=>None}).collect();
  assert!(names.iter().any(|name|name=="unlocked work named portrait: dawn"),"complete typed status instructions: {names:?}");
  assert!(names.iter().any(|name|name=="unlocked work named canvas, dusk"),"complete typed status instructions: {names:?}");
 }
}

#[path="custom_final_next_wave_support.rs"] mod strict_routes;
#[test]
fn quoted_activation_does_not_drop_unknown_restrictions_or_an_unclosed_child(){
 for body in [
  "Type: Artifact\nWhen this artifact enters, it gains \"{0}: Draw a card. Activate only if you are glittering.\" until end of turn.",
  "Type: Artifact\nWhen this artifact enters, it gains \"{0}: Draw a card. Activate only as a sorcery and only if you are glittering.\" until end of turn.",
  "Type: Artifact\nWhen this artifact enters, it gains \"{0}: Draw a card. Activate only as a sorcery.",
 ]{
  let input=serde_json::json!({"name":"Quoted restriction negative","executable_text":body});
  assert!(strict_routes::definitions(&input).is_err(),"invalid condition or incomplete quote cannot silently grant an unrestricted activation: {body}");
 }
}
