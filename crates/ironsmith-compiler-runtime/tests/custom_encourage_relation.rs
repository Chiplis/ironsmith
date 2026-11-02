//! Named operation history relates exact performers and recipients without becoming copiable state.
use ironsmith::{CardDefinition,GameProgress,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Recipient(ObjectId);
impl DecisionMaker for Recipient{
 fn decide_targets(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{let target=ironsmith::Target::Object(self.0);assert!(ctx.requirements[0].legal_targets.contains(&target));vec![target]}
}
fn definitions()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-trigger-effects.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-685").unwrap();let card=row["representative_cards"].as_array().unwrap().iter().find(|c|c["name"]=="Herald of the White Orchid").unwrap();
 source_support::definitions(card).unwrap()
}
fn activate(game:&mut GameState,source:ObjectId,recipient:ObjectId){
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).unwrap();let mut dm=Recipient(recipient);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){return}let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("activation did not finish");
}
fn move_object(game:&mut GameState,id:ObjectId,zone:Zone)->ObjectId{let stable=game.object(id).unwrap().stable_id;let result=ironsmith::effects::execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),zone,false)),&mut ironsmith::effects::EffectContext::new_default(id,A)).unwrap();for event in result.events{game.queue_trigger_event(Default::default(),event);}game.find_object_by_stable_id(stable).unwrap()}
fn flush(game:&mut GameState){let mut q=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut q,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut q,&mut SelectFirstDecisionMaker).unwrap();}
#[test]
fn source_recipient_history_is_exact_performer_exact_incarnation_and_turn_scoped(){
 for definition in definitions(){for mode in ["normal","other-source","different-recipient","recipient-reentry","source-reentry","later-turn","source-departs-after-trigger","invalid-target","noncombat"]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient body","Type: Creature — Elf\nPower/Toughness: 2/2",false).unwrap();let mut recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);let other_recipient=game.create_object_from_definition(&body,A,Zone::Battlefield);let other_source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let actor=if mode=="other-source"{other_source}else{source};game.remove_summoning_sickness(actor);
  activate(&mut game,actor,recipient);let original_target=recipient;if mode=="invalid-target"{recipient=move_object(&mut game,recipient,Zone::Hand);}ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();flush(&mut game);assert!(game.stack.is_empty());
  if mode=="invalid-target"{assert!(!game.named_action_affected_this_turn(actor,"encourage",original_target));assert_eq!(game.named_action_count(actor,"encourage",false),0);continue;}
  assert!(game.named_action_affected_this_turn(actor,"encourage",recipient));assert!(!game.named_action_affected_this_turn(actor,"encourage",other_recipient));assert!(!game.named_action_affected_this_turn(actor,"encourage",actor));assert_eq!(game.named_action_count(actor,"encourage",false),1);
  if mode=="recipient-reentry"{recipient=move_object(&mut game,recipient,Zone::Hand);recipient=move_object(&mut game,recipient,Zone::Battlefield);}
  if mode=="source-reentry"{let new=move_object(&mut game,source,Zone::Hand);let new=move_object(&mut game,new,Zone::Battlefield);assert_eq!(game.named_action_count(new,"encourage",false),0);}
  if mode=="later-turn"{game.turn.turn_number+=1;}
  let dealer=if mode=="different-recipient"{other_recipient}else{recipient};let combat=mode!="noncombat";game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::DamageEvent::with_cause(dealer,ironsmith::events::DamageTarget::Player(B),1,combat,if combat{ironsmith::events::EventCause::combat_damage(dealer)}else{ironsmith::events::EventCause::effect()}),Default::default()));flush(&mut game);
  let expected=matches!(mode,"normal"|"other-source"|"source-departs-after-trigger");assert_eq!(game.stack.len(),usize::from(expected),"{mode}");if mode=="other-source"{assert!(game.stack.iter().all(|entry|entry.object_id==other_source));}
  if mode=="source-departs-after-trigger"{move_object(&mut game,source,Zone::Hand);assert!(game.named_action_affected_this_turn(source,"encourage",recipient),"queued intervening-if keeps the exact departed actor's current-turn relationship");}
  if expected{ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  let tokens=game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.kind==ironsmith::object::ObjectKind::Token&&object.has_subtype(ironsmith::Subtype::Soldier)).count();assert_eq!(tokens,usize::from(expected),"{mode}");
 }}
}
