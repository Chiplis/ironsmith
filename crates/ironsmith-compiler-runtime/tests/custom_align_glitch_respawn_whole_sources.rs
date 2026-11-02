//! Whole raw corpus including grants, counter choices, consumers and persistent fragments.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,PlayerId,ObjectId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-align-glitch-respawn-whole-source.json")).unwrap()}
fn definitions(key:&str)->[CardDefinition;2]{let f=fixture();let c=f["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]==key).unwrap();source_support::definitions(c).unwrap()}
#[test]
fn complete_forty_eight_physical_sources_compile_losslessly_on_independent_routes(){
 let f=fixture();let mut failures=Vec::new();let mut count=0;
 for row in f["mechanics"].as_array().unwrap(){let expected=match row["keyword"].as_str().unwrap(){"Align"=>6,"Glitch"=>26,"Respawn"=>16,_=>panic!("unexpectedfamily")};assert_eq!(row["representative_cards"].as_array().unwrap().len(),expected);
 for card in row["representative_cards"].as_array().unwrap(){count+=1;assert_eq!(card["source_card_key"],card["original_source_record"]["key"]);if card["rewrite_status"]!="frontend_materialized_unverified"{failures.push(format!("{}: {}",card["source_card_key"],card["rewrite_status"]));continue}match source_support::linked_definitions(card){Ok(pairs)=>{assert_eq!(pairs.len(),card["source_faces"].as_array().unwrap().len());for [native,wire] in pairs{assert_eq!(ironsmith_text::compiled_text_lines(&native),ironsmith_text::compiled_text_lines(&wire));}},Err(e)=>failures.push(format!("{}: {e}",card["source_card_key"]))}
 }}assert_eq!(count,48);assert!(failures.is_empty(),"{}",failures.join("\n"));
}
#[test]
fn actual_respawn_source_returns_only_after_real_attack_and_only_while_exact_card_remains_in_graveyard(){
 use ironsmith::decision::AttackerDeclaration;use ironsmith::combat_state::{CombatState,AttackTarget};use ironsmith::triggers::TriggerQueue;
 for d in definitions("Mukal Butcher_AKD"){for attacked in [false,true]{for removed in [false,true]{
 let mut g=GameState::new(vec!["A".into(),"B".into()],20);let id=g.create_object_from_definition(&d,A,Zone::Battlefield);let stable=g.object(id).unwrap().stable_id;g.take_pending_trigger_events();
 if attacked{g.remove_summoning_sickness(id);g.turn.active_player=A;g.turn.phase=ironsmith::game_state::Phase::Combat;g.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);g.mark_combat_phase_started();let mut combat=CombatState::default();let mut q=TriggerQueue::new();ironsmith::game_loop::apply_attacker_declarations(&mut g,&mut combat,&mut q,&[AttackerDeclaration{creature:id,target:AttackTarget::Player(B)}]).unwrap();}
 let dead=g.move_object(id,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();if removed{g.move_object(dead,Zone::Exile,ironsmith::events::EventCause::effect()).unwrap();}
 let mut q=TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut g,&mut q,&mut dm).unwrap();assert_eq!(q.entries.len(),usize::from(attacked));ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g,&mut q,&mut dm).unwrap();while !g.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();}
 let now=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(now).unwrap().zone,if removed{Zone::Exile}else if attacked{Zone::Hand}else{Zone::Graveyard});
 }}}
}
struct PaidFragment{target:ObjectId,payment_seen:bool,fragment_seen:bool}
impl DecisionMaker for PaidFragment{
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{if c.description.starts_with("Choose optional costs"){self.payment_seen=true;let o=c.options.iter().find(|o|o.legal).unwrap();assert!(o.repeatable);return vec![o.index]}if c.description=="Choose bracketed text fragments to remove"{self.fragment_seen=true;assert_eq!((c.min,c.max),(1,1));return vec![0]}SelectFirstDecisionMaker.decide_options(g,c)}
 fn decide_targets(&mut self,_:&GameState,c:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{assert!(c.requirements[0].legal_targets.contains(&ironsmith::Target::Object(self.target)));vec![ironsmith::Target::Object(self.target)]}
 fn decide_mana_payment(&mut self,g:&GameState,c:&ironsmith::decisions::context::ManaPaymentContext)->ironsmith::mana_payment::ManaPaymentResponse{SelectFirstDecisionMaker.decide_mana_payment(g,c)}
}
#[test]
fn actual_glitch_removes_attacking_restriction_before_targets_and_pays_the_repeatable_price(){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};use ironsmith::mana::ManaSymbol;
 for d in definitions("Blessing of Immunity_AKD"){let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Nonattacking witness","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=g.create_object_from_definition(&body,A,Zone::Battlefield);let spell=g.create_object_from_definition(&d,A,Zone::Hand);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White,1);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).expect("paid variant has a legal nonattacking target");let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=PaidFragment{target,payment_seen:false,fragment_seen:false};let mut progress=apply_priority_response_with_dm(&mut g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();for _ in 0..64{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut g,&mut q,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());assert!(dm.payment_seen&&dm.fragment_seen);assert_eq!(g.player(A).unwrap().mana_pool.total(),0);ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut dm).unwrap();assert_eq!(g.calculated_power(target),Some(4));assert_eq!(g.calculated_toughness(target),Some(4));assert!(g.current_has_static_ability_id(target,ironsmith_core::StaticAbilityId::Indestructible));}
}
