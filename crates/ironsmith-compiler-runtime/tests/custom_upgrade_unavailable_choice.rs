//! Source-defined counter-menu failure produces its own targeted reflexive.
#[path="custom_final_next_wave_support.rs"]mod source_support;
use ironsmith::{GameState,PlayerId,Target,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct ChooseOpponent { targets:usize }
impl DecisionMaker for ChooseOpponent {
 fn decide_targets(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<Target>{self.targets+=1;assert_eq!(ctx.requirements.len(),1);assert!(ctx.requirements[0].legal_targets.contains(&Target::Player(B)));vec![Target::Player(B)]}
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{SelectFirstDecisionMaker.decide_options(g,c)}
 fn decide_mana_payment(&mut self,g:&GameState,c:&ironsmith::decisions::context::ManaPaymentContext)->ironsmith::mana_payment::ManaPaymentResponse{SelectFirstDecisionMaker.decide_mana_payment(g,c)}
}
#[test]
fn actual_source_checks_choice_availability_not_counter_output_then_stacks_a_separate_targeted_trigger(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/upgrade-unavailable-source.json")).unwrap();
 assert_eq!(fixture["original_source_record"]["rulesText"],"Whenever you cast a noncreature spell, upgrade Indecisive Sorceress. When you can't, remove all counters from it and it deals damage to any target equal to the number of counters removed this way.\n• +1/+1\n• Flying\n• First strike");
 for definition in source_support::definitions(&fixture).unwrap(){for scenario in 0..3 {
  let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;
  let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);
  for kind in [CounterType::PlusOnePlusOne,CounterType::Flying]{g.add_counters(source,kind,1).unwrap();}
  if scenario==1 {g.add_counters(source,CounterType::FirstStrike,1).unwrap();g.add_counters(source,CounterType::Charge,2).unwrap();}
  if scenario==2 {let barrier=ironsmith_compiler_runtime::compile_to_runtime_definition("Placement prevention","Type: Artifact\nCounters can't be put on creatures you control.",false).unwrap();g.create_object_from_definition(&barrier,A,Zone::Battlefield);}
  let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Noncreature trigger witness","Mana cost: {0}\nType: Instant\nYou gain 1 life.",false).unwrap();let spell=g.create_object_from_definition(&ordinary,A,Zone::Hand);g.take_pending_trigger_events();
  let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).unwrap();
  use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
  let mut state=PriorityLoopState::new(2);let mut q=TriggerQueue::new();let mut dm=ChooseOpponent{targets:0};let mut progress=apply_priority_response_with_dm(&mut g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..64{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut g,&mut q,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut g,&mut q,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g,&mut q,&mut dm).unwrap();assert_eq!(g.stack.len(),2);assert_eq!(dm.targets,0,"the source cast watcher has no targets");
  ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();assert_eq!(dm.targets,0,"a queued reflexive does not choose its target during the parent resolution");assert_eq!(g.player(B).unwrap().life,20);
  let initial=if scenario==1{5}else if scenario==0{3}else{2};assert_eq!(g.object(source).unwrap().counters.values().copied().sum::<u32>(),initial);
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut g,&mut q,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g,&mut q,&mut dm).unwrap();
  if scenario==1 {assert_eq!(g.stack.len(),2,"unavailable menu creates a new reflexive above the original spell");assert_eq!(dm.targets,1);assert_eq!(g.object(source).unwrap().counters.values().copied().sum::<u32>(),5,"counter removal waits for the separately targeted ability");ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();assert_eq!(g.player(B).unwrap().life,15,"all five actually removed counters determine the damage");assert_eq!(g.object(source).unwrap().counters.values().copied().sum::<u32>(),0);}
  else {assert_eq!(g.stack.len(),1,"an available kind suppresses the fallback even when placement was prevented");assert_eq!(dm.targets,0);assert_eq!(g.player(B).unwrap().life,20);}
 }}
}
