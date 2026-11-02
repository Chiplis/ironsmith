//! Original source reminders supply the Weapon blueprint through ordinary metadata.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
const A:PlayerId=PlayerId(0);
fn settle(g:&mut GameState){
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 for _ in 0..16{ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut queue,&mut dm).unwrap();if g.stack.is_empty(){return}ironsmith::game_loop::resolve_stack_entry_with(g,&mut dm).unwrap();}panic!("source token program did not finish")
}
fn activate_equip(g:&mut GameState,token:ObjectId,index:usize){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let action=compute_legal_actions(g,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index}if *source==token&&*ability_index==index)).expect("the supplied equip price is payable");
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(g,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..32{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(g,&mut queue,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut queue,&mut dm).unwrap();settle(g);
}
#[test]
fn both_actual_foreshadow_sources_create_complete_weapon_and_its_equip_works(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
 for key in ["Cathedral Gargoyle_INM","Lambholt Tracker_INM"]{
  let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]==key).unwrap();
  assert!(card["original_text"].as_str().unwrap().contains("Equipment artifact with “Equipped creature gets +1/+1” and equip {2}"));
  for (route, definition) in source_support::definitions(card).unwrap().into_iter().enumerate(){
   let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;
   let hostdef=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent equipment recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let host=g.create_object_from_definition(&hostdef,A,Zone::Battlefield);
   let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);
   g.push_to_stack(ironsmith::game_state::StackEntry::ability(source,A,vec![ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,false)]));settle(&mut g);
   let weapons=g.battlefield.iter().copied().filter(|id|g.object(*id).unwrap().name.as_str()=="Weapon").collect::<Vec<_>>();assert_eq!(weapons.len(),1,"source={key} route={route}; battlefield={:?}; source_definition={:?}",g.battlefield.iter().filter_map(|id|g.object(*id)).map(|object|(object.id,object.name.to_string(),object.kind,object.zone)).collect::<Vec<_>>(),definition.abilities);let token=weapons[0];let object=g.object(token).unwrap();assert_eq!(object.kind,ironsmith::object::ObjectKind::Token);
   let chars=g.calculated_characteristics(token).unwrap();assert_eq!(chars.card_types,vec![ironsmith::CardType::Artifact]);assert!(chars.subtypes.contains(&ironsmith::Subtype::Equipment));assert_eq!(chars.colors,ironsmith::color::ColorSet::COLORLESS);
   let equip=chars.abilities.iter().enumerate().find_map(|(index,ability)|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)if matches!(ability.keyword,Some(ironsmith_core::ActivatedAbilityKeyword::Equip))=>{assert_eq!(ability.mana_cost.mana_cost().unwrap().mana_value(),2);Some(index)},_=>None}).expect("Weapon contains real equip ability");
   g.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);g.turn.priority_player=Some(A);activate_equip(&mut g,token,equip);
   assert_eq!(g.object(token).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));assert_eq!(g.calculated_power(host),Some(3));assert_eq!(g.calculated_toughness(host),Some(3));assert_eq!(g.player(A).unwrap().mana_pool.total(),0);
  }
 }
}
