//! Exact Reytali source grant uses an acquired grant receipt through payment.
use ironsmith::{CardDefinition,GameState,ObjectId,Zone};
use ironsmith::decision::{compute_legal_actions,LegalAction,SelectFirstDecisionMaker,DecisionMaker};
use ironsmith::special_actions::{SpecialAction,perform,can_perform_check};
use ironsmith::mana::ManaSymbol;
#[path="cf8_p08/support.rs"] mod support;
#[path="cf8_p08/play.rs"] mod play;
fn providers()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-orchestrate-next.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"].as_str().is_some_and(|key|key.contains("Reytali"))).unwrap();
 support::definitions(card["name"].as_str().unwrap(),card["executable_faces"][0]["executable_text"].as_str().unwrap())
}
fn spell()->CardDefinition{support::definitions("Preparation recipient","Mana cost: {2}{R}\nType: Instant\nYou gain 1 life.")[0].clone()}
fn granted_action(game:&GameState,card:ObjectId,provider:ObjectId)->SpecialAction{
 let grants=game.effect_store.grant_registry.granted_alternative_casts_for_card(game,card,Zone::Hand,play::A);
 let (index,grant)=grants.iter().enumerate().find(|(_,grant)|grant.source_id==provider&&grant.method.exile_preparation().is_some()).unwrap();
 SpecialAction::PrepareGrantedExile{card_id:card,grant:ironsmith::alternative_cast::GrantSelection{identity:grant.permission_identity.clone().unwrap(),source:provider,index},preparation:grant.method.exile_preparation().unwrap().clone()}
}
fn own_resolution(game:&mut GameState)->ironsmith::triggers::TriggerEvent{
 let def=support::definitions("Successful spell","Type: Instant\nYou gain 1 life.")[0].clone();let id=game.create_object_from_definition(&def,play::A,Zone::Stack);
 let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(),game);
 ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::spells::SpellResolvedEvent::new(id,play::A,snapshot),Default::default())
}
#[test]
fn actual_reytali_grants_only_eligible_owned_hand_cards_and_keeps_two_acquisitions_distinct(){
 for definition in providers(){
  let mut game=play::game();let first=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);let second=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);
  let card=game.create_object_from_definition(&spell(),play::A,Zone::Hand);let other=game.create_object_from_definition(&spell(),play::B,Zone::Hand);
  let creature=play::vanilla("Creature recipient","{0}","Soldier",1,1);let creature=game.create_object_from_definition(&creature,play::A,Zone::Hand);
  play::give_mana(&mut game,play::A,ManaSymbol::Red,1);
  let a=granted_action(&game,card,first);let b=granted_action(&game,card,second);assert_ne!(a,b);
  let actions=compute_legal_actions(&game,play::A).unwrap();assert!(actions.contains(&LegalAction::SpecialAction(a.clone())));assert!(actions.contains(&LegalAction::SpecialAction(b)));
  assert!(!actions.iter().any(|action|matches!(action,LegalAction::SpecialAction(SpecialAction::PrepareGrantedExile{card_id,..}) if *card_id==creature||*card_id==other)));
  game.turn.phase=ironsmith::game_state::Phase::Combat;assert!(can_perform_check(&a,&game,play::A).is_err());game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  game.move_object_by_effect(first,Zone::Graveyard).unwrap();assert!(can_perform_check(&a,&game,play::A).is_err(),"an old occurrence cannot retarget the equal grant from the other provider");
 }
}
#[test]
fn actual_grant_preparation_survives_provider_departure_and_uses_one_delayed_trigger(){
 for definition in providers(){
  let mut game=play::game();let provider=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);let card=game.create_object_from_definition(&spell(),play::A,Zone::Hand);play::give_mana(&mut game,play::A,ManaSymbol::Red,1);
  let action=granted_action(&game,card,provider);let expected=match &action{SpecialAction::PrepareGrantedExile{grant,..}=>grant.clone(),_=>unreachable!()};
  perform(action,&mut game,play::A,&mut SelectFirstDecisionMaker).unwrap();let exiled=*game.exile.last().unwrap();assert_eq!(game.exile_preparation_grant(exiled),Some(&expected));
  assert!(!game.is_plotted_by(exiled,play::A));assert!(game.stack.is_empty());assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);
  game.move_object_by_effect(provider,Zone::Graveyard).unwrap();let event=own_resolution(&mut game);let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&event);assert_eq!(entries.len(),1);
  let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries{queue.add(entry);}ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert!(game.stack.iter().any(|entry|!entry.is_ability&&game.object(entry.object_id).is_some_and(|card|card.name=="Preparation recipient")));
  assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&event).is_empty());
 }
}
struct SacrificeProvider{mana_source:ObjectId,used:bool,cancel:bool}
impl DecisionMaker for SacrificeProvider{
 fn decide_mana_payment(&mut self,_game:&GameState,ctx:&ironsmith::decisions::context::ManaPaymentContext)->ironsmith::mana_payment::ManaPaymentResponse{
  if !self.used {self.used=true;return ironsmith::mana_payment::ManaPaymentResponse::Activate{source:self.mana_source,ability_index:0};}
  if self.cancel {ironsmith::mana_payment::ManaPaymentResponse::Cancel}else{ironsmith::mana_payment::ManaPaymentResponse::Confirm{plan_id:ctx.plan.id,request_hash:ctx.plan.request_hash}}
 }
}
#[test]
fn selected_source_acquisition_is_retained_when_its_provider_is_sacrificed_during_payment(){
 for definition in providers(){for cancel in [false,true]{
  let mut game=play::game();let provider=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);let card=game.create_object_from_definition(&spell(),play::A,Zone::Hand);
  let altar=support::definitions("Payment source","Type: Artifact\nSacrifice a creature: Add {R}.")[0].clone();let altar=game.create_object_from_definition(&altar,play::A,Zone::Battlefield);
  let action=granted_action(&game,card,provider);let expected=match &action{SpecialAction::PrepareGrantedExile{grant,..}=>grant.clone(),_=>unreachable!()};
  let result=perform(action.clone(),&mut game,play::A,&mut SacrificeProvider{mana_source:altar,used:false,cancel});
  if cancel{assert!(result.is_err());assert_eq!(game.object(card).unwrap().zone,Zone::Hand);assert_eq!(game.object(provider).unwrap().zone,Zone::Battlefield);assert!(game.effect_store.delayed_triggers.is_empty());assert!(can_perform_check(&action,&game,play::A).is_ok());}
  else{result.unwrap();let exiled=*game.exile.last().unwrap();assert_eq!(game.exile_preparation_grant(exiled),Some(&expected));assert!(game.object(provider).is_none_or(|object|object.zone!=Zone::Battlefield));assert_eq!(game.effect_store.delayed_triggers.len(),1);}
 }}
}
