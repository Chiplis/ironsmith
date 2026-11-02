//! Source-owned numeric selections gate ordinary static/granted abilities.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,compute_legal_actions};
#[path="custom_final_next_wave_support.rs"] mod source_routes;
const A:PlayerId=PlayerId(0);
fn definitions(id:&str)->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-mimic-numbered.json")).unwrap();
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();let card=&row["representative_cards"][0];
 source_routes::definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"]))
}
struct Number(u32);
impl DecisionMaker for Number{fn decide_number(&mut self,_:&GameState,_:&ironsmith::decisions::context::NumberContext)->u32{self.0}}
fn activation(game:&GameState,source:ObjectId)->Option<LegalAction>{compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source))}
fn choose(game:&mut GameState,source:ObjectId,number:u32){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with,put_triggers_on_stack_with_dm};
 for mana in [ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Red]{game.player_mut(A).unwrap().mana_pool.add(mana,1);}
 let action=activation(game,source).expect("paid numeric choice legal");let mut dm=Number(number);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
 assert!(!state.has_pending_action());
 for _ in 0..16{if game.stack.is_empty(){return;}resolve_stack_entry_with(game,&mut dm).unwrap();put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();}panic!("unsettled numeric choice");
}
fn has(game:&GameState,source:ObjectId,id:ironsmith::static_abilities::StaticAbilityId)->bool{game.calculated_characteristics_arc(source).unwrap().static_abilities.iter().any(|ability|ability.id()==id)}
#[test]fn all_three_exact_numbered_source_variants_compile_losslessly(){for id in ["msem-700","msem-701","msem-702"]{definitions(id);}}
#[test]fn repeated_choice_triggers_after_new_characteristics_and_unknown_number_restores_base(){
 use ironsmith::static_abilities::StaticAbilityId as K;
 for definition in definitions("msem-702"){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
 let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);assert!(!has(&game,source,K::Trample));
 choose(&mut game,source,1);assert!(has(&game,source,K::Trample));assert!(has(&game,source,K::Hexproof));assert_eq!(game.current_power(source),Some(4));
 choose(&mut game,source,2);assert!(!has(&game,source,K::Trample));assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),3);
 choose(&mut game,source,2);assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),6,"reselection is still a completed choice");
 let other=game.create_object_from_definition(&definition,A,Zone::Battlefield);choose(&mut game,other,2);assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),6,"other source cannot share this numeric acquisition or event");
 choose(&mut game,source,17);assert!(!has(&game,source,K::Trample));assert!(!has(&game,source,K::Hexproof));assert_eq!(game.current_power(source),Some(10));
 choose(&mut game,source,0);assert!(!has(&game,source,K::Trample));assert!(!has(&game,source,K::Hexproof));assert_eq!(game.current_power(source),Some(10),"zero remains a legal choice and has no printed section on this source");
 for mana in [ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Red]{game.player_mut(A).unwrap().mana_pool.add(mana,1);}
 assert!(activation(&game,source).is_some());game.turn.phase=ironsmith::Phase::Combat;assert!(activation(&game,source).is_none(),"source choice is sorcery-only");
 }}
