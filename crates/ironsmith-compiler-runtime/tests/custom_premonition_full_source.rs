//! Exact source through independent direct/artifact routes; parent owns execution.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::special_actions::{SpecialAction,can_perform_check,perform};
use ironsmith::ability::AbilityKind;
use ironsmith::mana::ManaSymbol;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn source()->serde_json::Value {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-premonition-full-source.json")).unwrap();
 let card=fixture["mechanics"][0]["representative_cards"][0].clone();
 assert_eq!(card["source_card_key"],"Cultish Sorcerer_MSY");card
}
fn ready()->GameState {let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game}
fn settle(game:&mut GameState) {
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 for _ in 0..32 {ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();if game.stack.is_empty(){return;}ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();}
 panic!("entry trigger resolution did not settle");
}
fn can_cast(game:&GameState,id:ironsmith::ObjectId)->bool {compute_legal_actions(game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id))}
#[test]
fn exact_source_special_action_copies_with_entry_counter_and_delays_only_original_exile_cast() {
 for definition in source_routes::definitions(&source()).unwrap() {
  let index=definition.abilities.iter().position(|ability|matches!(&ability.kind,AbilityKind::Static(rule)if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{..})))).unwrap();
  assert!(definition.abilities[index].functions_in(&Zone::Hand), "outer hand action retains its declared functional zone");
  assert!(!definition.abilities[index].functions_in(&Zone::Exile), "nested exile casting permission does not relocate the outer action");
  let mut game=ready();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;
  let action=SpecialAction::PerformIntrinsicAction{source_id:card,ability_index:index};
  assert!(can_perform_check(&action,&game,A).is_err(),"printed preparation cost is unaffordable");
  game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);
  game.turn.active_player=B;assert!(can_perform_check(&action,&game,A).is_err(),"sorcery timing is retained");game.turn.active_player=A;
  assert!(can_perform_check(&action,&game,A).is_ok(), "funded outer hand action is available at sorcery timing");
  let before_turn=game.turn.turn_number;
  perform(action,&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
  assert!(game.stack.is_empty(),"special action does not itself use the stack");
  settle(&mut game);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);
  let tokens=game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|object|object.name=="Cultish Sorcerer")).collect::<Vec<_>>();assert_eq!(tokens.len(),1);let token=tokens[0];
  assert_ne!(game.object(token).unwrap().stable_id,stable);assert_eq!(game.counter_count(token,ironsmith::object::CounterType::Decayed),1);
  assert!(game.current_has_static_ability_id(token,ironsmith_core::StaticAbilityId::CantBlock));
  assert_eq!(game.counter_count(exiled,ironsmith::object::CounterType::Decayed),0);
  game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);assert!(!can_cast(&game,exiled),"permission not active in the preparation turn");
  game.turn.phase=ironsmith::game_state::Phase::Ending;assert!(!can_cast(&game,exiled),"end step is still current turn");
  game.next_turn();assert!(game.turn.turn_number>before_turn);game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);assert!(!can_cast(&game,exiled),"normal creature timing still forbids opponent turn");
  game.next_turn();game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);
  assert!(can_cast(&game,exiled),"ordinary original mana cost and timing now authorized");
  let grave=game.move_object_by_effect(exiled,Zone::Graveyard).unwrap();let returned=game.move_object_by_effect(grave,Zone::Exile).unwrap();assert!(!can_cast(&game,returned),"old permission never follows a new exile incarnation");
 }
}
