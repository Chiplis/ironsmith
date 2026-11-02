//! Complete source inventory and preparation-price scenarios; parent owns execution.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::special_actions::{can_perform_check,perform};
use ironsmith::mana::ManaSymbol;
const A:PlayerId=PlayerId(0);
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-orchestrate-full-19.json")).unwrap()}
fn card(name:&str)->serde_json::Value {fixture()["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]==name).unwrap().clone()}
#[test]
fn all_nineteen_actual_source_cards_and_twenty_faces_compile_both_independent_routes() {
 let x=fixture();let cards=x["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),19);let mut faces=0;
 for card in cards {for face in card["executable_faces"].as_array().unwrap(){let mut source=card.clone();source.as_object_mut().unwrap().extend(face.as_object().unwrap().clone());let _=source_routes::definitions(&source).unwrap_or_else(|error|panic!("{}: {error}",face["name"]));faces+=1;}}
 assert_eq!(faces,20,"includes both complete Reytali faces");
}
fn ready()->GameState {let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g}
fn offered(game:&GameState,source:ironsmith::ObjectId)->Option<ironsmith::special_actions::SpecialAction> {
 compute_legal_actions(game,A).unwrap().into_iter().find_map(|action|match action {LegalAction::SpecialAction(action @ ironsmith::special_actions::SpecialAction::PrepareExile{card_id,..}) if card_id==source=>Some(action),_=>None})
}
#[test]
fn exact_nonmana_preparation_prices_pay_all_components_and_use_no_activation_stack() {
 for name in ["Crystal Conjuration","Malign Growth","Blessing of Valor"] {for definition in source_routes::definitions(&card(name)).unwrap(){for separate in [false,true] {
  let mut game=ready();let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
  for symbol in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless]{game.player_mut(A).unwrap().mana_pool.add(symbol,10);}
  assert!(offered(&game,source).is_none(),"{name}: mana alone cannot pay the preparation price");
  let texts=if name=="Blessing of Valor" {if separate {vec!["Mana cost: {R}\nType: Creature\nPower/Toughness: 1/1","Mana cost: {W}\nType: Creature\nPower/Toughness: 1/1"]}else{vec!["Mana cost: {R}{W}\nType: Creature\nPower/Toughness: 1/1"]}}else{vec!["Type: Land"]};
  let materials=texts.into_iter().map(|text|{let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Preparation material",text,false).unwrap();game.create_object_from_definition(&definition,A,Zone::Battlefield)}).collect::<Vec<_>>();
  let action=offered(&game,source).expect("exact authored price becomes affordable");assert!(can_perform_check(&action,&game,A).is_ok());perform(action,&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
  assert!(game.stack.is_empty());assert_eq!(game.effect_store.delayed_triggers.len(),1);
  let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);
  if name=="Blessing of Valor"{assert!(materials.iter().all(|id|game.is_tapped(*id)),"both distinct material roles are paid");}else{assert!(materials.iter().all(|id|game.object(*id).is_none()),"sacrifice is real zone movement, not a selected-object label");}
 }}}
}
