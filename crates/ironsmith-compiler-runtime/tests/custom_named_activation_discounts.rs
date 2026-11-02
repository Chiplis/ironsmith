//! Exact named activation cost facts; complete Fallow Grave/Revived Colpah control.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition,GameState,Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{LegalAction,compute_legal_actions,ActivationCostAbility};
use ironsmith::mana::ManaSymbol;
fn definitions(name:&str,text:&str)->[CardDefinition;2] {
 source_routes::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
const FALLOW:&str="Mana cost: {1}{G}\nType: Enchantment\nActivated abilities named \"revive\" of cards in your graveyard cost {1} less to activate.\nSacrifice Fallow Grave: Create a 1/1 black Worm creature token.";
const COLPAH:&str="Mana cost: {1}{B}\nType: Creature — Cat Horror\nPower/Toughness: 2/1\nMenace\n{3}{B}, Exile this card from your graveyard: This is the revive activated ability. Create a token that's a copy of this card, except it has no mana cost. Perform the revive action with tokens created this way. Activate only as a sorcery.";
fn funding(game:&mut GameState) {play::give_mana(game,play::A,ManaSymbol::Colorless,2);play::give_mana(game,play::A,ManaSymbol::Black,1);}
#[test]
fn actual_fallow_grave_reduces_only_its_controllers_graveyard_revive_price() {
 for route in 0..2 {for own in [false,true] {
  let mut game=play::game();let provider=definitions("Fallow Grave",FALLOW);
  game.create_object_from_definition(&provider[route],if own{play::A}else{play::B},Zone::Battlefield);
  let source=source_routes::definitions(&serde_json::json!({
   "name":"Revived Colpah","source_card_key":"Revived Colpah_ORP",
   "original_text":"Mana cost: {1}{B}\nType: Creature — Cat Horror\nPower/Toughness: 2/1\nMenace\nRevive {3}{B} *({3}{B}, Exile this card from your graveyard: Create a token that's a copy of it except it has no mana cost. Revive only as a sorcery.)*",
   "executable_text":COLPAH,"custom_keyword_identities":["revive"]
  })).unwrap();
  let card=game.create_object_from_definition(&source[route],play::A,Zone::Graveyard);funding(&mut game);
  let offered=compute_legal_actions(&game,play::A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source,..} if *source==card));
  assert_eq!(offered,own,"three available mana pays only the correctly discounted 2B activation");
  if own {
   let stable=game.object(card).unwrap().stable_id;
   play::activate(&mut game,card,play::activated_at(&source[route],0),&mut play::Script::default());
   assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);
   let original=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(original).unwrap().zone,Zone::Exile);
   while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut play::Script::default()).unwrap();}
   let token=*game.battlefield.iter().find(|id|game.object(**id).unwrap().kind==ironsmith::object::ObjectKind::Token).unwrap();
   assert_eq!(game.object(token).unwrap().name.as_ref(),"Revived Colpah");assert!(game.object(token).unwrap().mana_cost.is_none());
  }
 }}
}
fn witness()->[CardDefinition;2] {
 source_routes::definitions(&serde_json::json!({"name":"Independent labeled activation witness","custom_keyword_identities":["revive"],
  "executable_text":"Mana cost: {1}{B}\nType: Creature — Cat Horror\nPower/Toughness: 2/1\n{3}{B}, Exile this card from your graveyard: This is the revive activated ability. You gain 1 life.\n{3}{B}, Exile this card from your graveyard: This is the revival activated ability. You gain 2 life.\n{3}{B}, Exile this card from your graveyard: You gain 3 life."})).unwrap()
}
#[test]
fn same_card_keyword_identity_does_not_discount_other_labeled_or_unlabeled_abilities() {
 for route in 0..2 {
  let mut game=play::game();let provider=definitions("Fallow Grave",FALLOW);game.create_object_from_definition(&provider[route],play::A,Zone::Battlefield);
  let witness=witness();let card=game.create_object_from_definition(&witness[route],play::A,Zone::Graveyard);funding(&mut game);
  let mut offered=compute_legal_actions(&game,play::A).unwrap().into_iter().filter_map(|action|match action {LegalAction::ActivateAbility{source,ability_index,..} if source==card=>Some(ability_index),_=>None}).collect::<Vec<_>>();offered.sort();
  assert_eq!(offered,vec![play::activated_at(&witness[route],0)]);
 }
}
#[test]
fn immutable_selected_label_survives_slot_program_changes_without_hash_or_prefix_matching() {
 for route in 0..2 {
  let mut game=play::game();let provider=definitions("Fallow Grave",FALLOW);game.create_object_from_definition(&provider[route],play::A,Zone::Battlefield);
  let witness=witness();let card=game.create_object_from_definition(&witness[route],play::A,Zone::Graveyard);
  let index=play::activated_at(&witness[route],0);let selected=game.current_ability(card,index).unwrap();
  let AbilityKind::Activated(selected)=&selected.kind else {panic!("activated witness")};
  let original_cost=selected.mana_cost.clone();let facts=ActivationCostAbility::of(&game,play::A,card,selected);
  assert_eq!(facts.activation_label.as_deref(),Some("revive"));
  let legacy=ironsmith::decision::calculate_effective_activation_total_cost_for_ability(&game,play::A,card,&original_cost,&[],None);
  assert_eq!(legacy.mana_cost().unwrap().mana_value(),4,"absent captured facts cannot infer a named discount from the card marker");
  for changed in ["revive craft".to_string(),format!("revive{}","x".repeat(256))] {
   {
    let object=game.object_mut(card).unwrap();
    let AbilityKind::Activated(current)=&mut object.abilities_mut()[index].kind else {panic!("mutable witness")};
    current.effects.activation_label=Some(changed.clone());
   }
   let price=ironsmith::decision::calculate_effective_activation_total_cost_for_ability(&game,play::A,card,&original_cost,&[],Some(&facts));
   assert_eq!(price.mana_cost().unwrap().mana_value(),3,"captured original revive label remains authoritative");
   let fresh=ActivationCostAbility::at(&game,play::A,card,index).unwrap();assert_eq!(fresh.activation_label.as_deref(),Some(changed.as_str()));
   let price=ironsmith::decision::calculate_effective_activation_total_cost_for_ability(&game,play::A,card,&original_cost,&[],Some(&fresh));
   assert_eq!(price.mana_cost().unwrap().mana_value(),4,"a distinct full name must not alias revive");
  }
 }
}
