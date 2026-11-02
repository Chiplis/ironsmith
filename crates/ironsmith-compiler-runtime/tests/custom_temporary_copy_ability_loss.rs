//! Timed layer-six copy modifiers apply before entry without erasing the copied text.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
const A:PlayerId=PlayerId(0);
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g}
fn donor(g:&mut GameState)->ObjectId{let d=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent copied text","Type: Creature\nPower/Toughness: 2/2\nFlying\nWhen this creature enters, you gain 7 life.",false).unwrap();g.create_object_from_definition(&d,A,Zone::Battlefield)}
fn settle(g:&mut GameState){let mut q=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;for _ in 0..16{ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut q,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut q,&mut dm).unwrap();if g.stack.is_empty(){return}ironsmith::game_loop::resolve_stack_entry_with(g,&mut dm).unwrap();}panic!("entry did not settle")}
fn copy(g:&mut GameState,source:ObjectId,temporary:bool)->ObjectId{let mut effect=ironsmith::effects::CreateTokenCopyEffect::one(ironsmith::target::ChooseSpec::SpecificObject(source));if temporary{effect.ability_loss_until=Some(ironsmith::effect::Until::EndOfTurn);}let before=g.battlefield.clone();g.push_to_stack(ironsmith::game_state::StackEntry::ability(source,A,vec![ironsmith::Effect::new(effect)]));settle(g);*g.battlefield.iter().find(|id|!before.contains(id)).unwrap()}
#[test]
fn temporary_copy_loss_precedes_etb_but_is_not_copiable_and_expires(){
 let mut g=game();let original=donor(&mut g);let temporary=copy(&mut g,original,true);
 assert_eq!(g.player(A).unwrap().life,20,"copied ETB ability is absent as the token enters");
 assert!(g.current_abilities(temporary).unwrap().is_empty());
 let printed=g.calculated_copiable_values(original).unwrap();let copied=g.calculated_copiable_values(temporary).unwrap();assert_eq!(copied.abilities.len(),printed.abilities.len());assert_eq!(copied.ability_labels,printed.ability_labels);
 let independent=copy(&mut g,temporary,false);assert!(g.object_has_static_ability_id(independent,ironsmith::static_abilities::StaticAbilityId::Flying));assert_eq!(g.player(A).unwrap().life,27,"another copy sees the preserved copiable ETB text");
 ironsmith::turn::execute_cleanup_step(&mut g);assert!(g.object_has_static_ability_id(temporary,ironsmith::static_abilities::StaticAbilityId::Flying));assert_eq!(g.player(A).unwrap().life,27,"expiration does not retroactively trigger an entry");
}
#[test]
fn complete_lamech_source_uses_real_loyalty_activation_for_timed_copy_loss(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Lamech, Seeking Apocolypse_EOW").unwrap();
 assert!(card["original_text"].as_str().unwrap().contains("except it loses all abilities until end of turn"));
 for definition in source_support::definitions(card).unwrap(){
  let mut g=game();let original=donor(&mut g);let walker=g.create_object_from_definition(&definition,A,Zone::Battlefield);let before=g.battlefield.clone();
  let index=g.current_abilities(walker).unwrap().iter().enumerate().find_map(|(index,ability)|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)if ability.effects.iter().any(|effect|effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>().is_some())=>Some(index),_=>None}).expect("the exact source has a copy-token loyalty activation");
  let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index}if *source==walker&&*ability_index==index)).unwrap();
  let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut g,&mut q,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..32{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut g,&mut q,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g,&mut q,&mut dm).unwrap();settle(&mut g);
  let token=*g.battlefield.iter().find(|id|!before.contains(id)).unwrap();assert!(g.current_abilities(token).unwrap().is_empty());assert_eq!(g.player(A).unwrap().life,20);assert_eq!(g.calculated_copiable_values(token).unwrap().abilities.len(),g.calculated_copiable_values(original).unwrap().abilities.len());ironsmith::turn::execute_cleanup_step(&mut g);assert!(g.object_has_static_ability_id(token,ironsmith::static_abilities::StaticAbilityId::Flying));
 }
}
