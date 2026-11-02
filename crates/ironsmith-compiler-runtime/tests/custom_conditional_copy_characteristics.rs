//! Full Imitator source, independent native/artifact routes and entering-object exceptions.
#[path="custom_final_next_wave_support.rs"] mod support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone,Effect};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext,BooleanContext};
use ironsmith::effects::{EffectContext,execute_effect};
const A:PlayerId=PlayerId(0);
fn sources()->[CardDefinition;2] {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-growth-actions.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Imitator of Success_MSC").unwrap();
 assert!(card["original_text"].as_str().unwrap().contains("if it's a token"));support::definitions(card).unwrap()
}
fn donor()->CardDefinition {support::definitions(&serde_json::json!({"name":"Chosen donor","executable_text":"Mana cost: {4}{R}\nType: Creature — Giant\nPower/Toughness: 6/7\nVigilance"})).unwrap()[0].clone()}
struct CopyChoice;
impl DecisionMaker for CopyChoice {
 fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{true}
 fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
  ctx.options.iter().find(|option|option.legal&&option.description.contains("Chosen donor")).map(|option|vec![option.index]).unwrap_or_else(||SelectFirstDecisionMaker.decide_options(game,ctx))
 }
}
fn game()->GameState {let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;game}
fn activate(game:&mut GameState,source:ObjectId,index:usize,dm:&mut CopyChoice) {
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let action=LegalAction::ActivateAbility{source,ability_index:index};assert!(compute_legal_actions(game,A).unwrap().contains(&action));
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..60 {if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();}
 assert!(!state.has_pending_action());assert_eq!(game.stack.len(),1);
 while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();}
}
#[test]
fn actual_imitator_card_copy_does_not_take_token_exception_even_when_donor_is_a_token() {
 for definition in sources(){for donor_is_token in [false,true] {
  let mut game=game();let donor_def=donor();let donor=if donor_is_token{game.create_token_object_from_definition(&donor_def,A,Zone::Battlefield)}else{game.create_object_from_definition(&donor_def,A,Zone::Battlefield)};
  let cost=game.object(donor).unwrap().mana_cost.clone();let colors=game.current_characteristics(donor).unwrap().colors;
  let entrant=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=CopyChoice;
  let entered=game.move_object_with_etb_processing_with_dm(entrant,Zone::Battlefield,&mut dm).unwrap().assert_without_additions().into_result().expect("actual copy entry").new_id;
  assert_eq!(game.object(entered).unwrap().name.as_ref(),"Chosen donor");assert_eq!(game.object(entered).unwrap().mana_cost,cost);
  assert_eq!(game.calculated_power(entered),Some(6));assert_eq!(game.calculated_toughness(entered),Some(7));assert_eq!(game.current_characteristics(entered).unwrap().colors,colors);
  assert!(game.current_has_static_ability_id(entered,ironsmith_core::StaticAbilityId::Vigilance));
 }}
}
#[test]
fn actual_successor_token_uses_entering_token_exception_and_its_copy_inherits_modified_values() {
 for definition in sources(){for donor_is_token in [false,true] {
  let mut game=game();let donor_def=donor();let donor=if donor_is_token{game.create_token_object_from_definition(&donor_def,A,Zone::Battlefield)}else{game.create_object_from_definition(&donor_def,A,Zone::Battlefield)};
  let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=game.object(source).unwrap().stable_id;
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,2);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  let index=definition.abilities.iter().position(|ability|matches!(ability.kind,ironsmith::ability::AbilityKind::Activated(_))).unwrap();let mut dm=CopyChoice;activate(&mut game,source,index,&mut dm);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
  let result=game.battlefield.iter().copied().find(|id|*id!=donor).unwrap();assert!(matches!(game.object(result).unwrap().kind,ironsmith::object::ObjectKind::Token));
  assert_eq!(game.object(result).unwrap().name.as_ref(),"Chosen donor");assert!(game.object(result).unwrap().mana_cost.is_none());assert_eq!(game.calculated_power(result),Some(1));assert_eq!(game.calculated_toughness(result),Some(1));
  assert!(game.current_has_static_ability_id(result,ironsmith_core::StaticAbilityId::Vigilance));
  let before=game.battlefield.clone();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(ironsmith::target::ChooseSpec::SpecificObject(result),1,ironsmith::target::PlayerFilter::You)),&mut EffectContext::new_default(result,A)).unwrap();
  let ids=game.battlefield.iter().copied().filter(|id|!before.contains(id)).collect::<Vec<_>>();assert_eq!(ids.len(),1);let next=ids[0];
  assert!(game.object(next).unwrap().mana_cost.is_none());assert_eq!(game.calculated_power(next),Some(1));assert_eq!(game.calculated_toughness(next),Some(1));
 }}
}
