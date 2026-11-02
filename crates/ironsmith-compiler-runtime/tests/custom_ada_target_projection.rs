//! Exact Ada source plus independent direct/artifact routes; parent owns execution.
#[path = "custom_final_next_wave_support.rs"] mod source_routes;
use ironsmith::{GameState,PlayerId,Zone,Target};
use ironsmith::decision::{DecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
struct Choices { extra:bool, targets:Option<Vec<Target>> }
impl DecisionMaker for Choices {
 fn decide_number(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::NumberContext)->u32 {
  if self.extra {ctx.max.min(2).max(ctx.min)} else {ctx.min}
 }
 fn decide_targets(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<Target> {
  let count=ctx.requirements.iter().map(|requirement|requirement.min_targets).sum::<usize>();
  if let Some(targets)=&self.targets{return targets.iter().take(count).copied().collect();}
  if count==2 {vec![Target::Player(A),Target::Player(B)]} else {vec![Target::Player(B)]}
 }
}
fn cast<D:DecisionMaker>(game:&mut GameState,card:ironsmith::ObjectId,dm:&mut D) {
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==card)).expect("spell has legal original targets");
 let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..64 {if !state.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,dm).unwrap();}
 panic!("target announcement did not finish: last progress={progress:?}, pending cast={:?}",state.pending_cast);
}
#[test]
fn exact_ada_expands_scalar_and_vector_recipients_without_repeating_untargeted_sibling() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
 let ada=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
 assert!(ada["original_text"].as_str().unwrap().contains("As you choose targets for an instant or sorcery spell, choose up to one additional target."));
 for provider in source_routes::definitions(ada).unwrap() {for scalar in [false,true] {for extra in [false,true] {for remove_provider in [false,true] {for copy in [false,true] {
  let text=if scalar {"Mana cost: {0}\nType: Instant\nTarget player gains 3 life.\nYou gain 1 life."}else{"Mana cost: {0}\nType: Instant\nThis spell deals 2 damage to any target.\nYou gain 1 life."};
  for spell in source_routes::definitions(&serde_json::json!({"name":"Independent recipient instruction","executable_text":text})).unwrap() {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
   let source=game.create_object_from_definition(&provider,A,Zone::Battlefield);let card=game.create_object_from_definition(&spell,A,Zone::Hand);let mut dm=Choices{extra,targets:None};cast(&mut game,card,&mut dm);
   let range=game.stack.last().unwrap().target_assignments[0].range.clone();assert_eq!(range.len(),if extra{2}else{1});
   if remove_provider {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
   if copy {
    let original=game.stack.last().unwrap().object_id;
    ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(original))),&mut ironsmith::effects::EffectContext::new_default(original,A)).unwrap();
    assert_eq!(game.stack.last().unwrap().target_assignments[0].range.len(),if extra{2}else{1},"copy retains announced groups without a live provider");
   }
   while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
   let copies=if copy{2}else{1};
   assert_eq!(game.player(A).unwrap().life,20+copies*(1+if extra{if scalar{3}else{-2}}else{0}),"untargeted sibling occurs once, independent of live provider");
   assert_eq!(game.player(B).unwrap().life,20+copies*if scalar{3}else{-2});
  }
 }}}}}
}

#[test]
fn one_or_both_added_group_targets_becoming_illegal_preserves_fizzle_boundary() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
 let ada=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
 let spell=serde_json::json!({"name":"Independent damage packet","executable_text":"Mana cost: {0}\nType: Instant\nThis spell deals 2 damage to target creature.\nYou gain 1 life."});
 for provider in source_routes::definitions(ada).unwrap() {for definition in source_routes::definitions(&spell).unwrap() {for removed in [0,1,2] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  game.create_object_from_definition(&provider,A,Zone::Battlefield);
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent body","Type: Creature\nPower/Toughness: 1/8",false).unwrap();
  let targets:Vec<_>=(0..2).map(|_|game.create_object_from_definition(&body,B,Zone::Battlefield)).collect();
  let card=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices{extra:true,targets:Some(targets.iter().copied().map(Target::Object).collect())};cast(&mut game,card,&mut dm);
  for id in targets.iter().take(removed){game.move_object_by_effect(*id,Zone::Graveyard).unwrap();}
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(A).unwrap().life,if removed==2{20}else{21},"all illegal targets stop the whole spell; one legal target preserves untargeted sibling");
  for id in targets.iter().skip(removed){assert_eq!(game.damage_on(*id),2);}
 }}}
}


struct XThenTargetCount { numbers:Vec<(bool,u32)> }
impl DecisionMaker for XThenTargetCount {
 fn decide_number(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::NumberContext)->u32 {
  let value=if ctx.is_x_value {2} else {1};assert!(value>=ctx.min&&value<=ctx.max);
  self.numbers.push((ctx.is_x_value,value));value
 }
 fn decide_targets(&mut self,_:&GameState,_:&ironsmith::decisions::context::TargetsContext)->Vec<Target>{vec![Target::Player(B)]}
}
#[test]
fn actual_ada_target_count_context_preserves_an_independently_announced_x_price() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
 let ada=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
 let spell=serde_json::json!({"name":"X announcement witness","executable_text":"Mana cost: {X}\nType: Instant\nTarget player gains X life.\nYou gain 1 life."});
 for provider in source_routes::definitions(ada).unwrap(){for definition in source_routes::definitions(&spell).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  game.create_object_from_definition(&provider,A,Zone::Battlefield);
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,3);
  let card=game.create_object_from_definition(&definition,A,Zone::Hand);
  let mut dm=XThenTargetCount{numbers:Vec::new()};cast(&mut game,card,&mut dm);
  assert_eq!(dm.numbers,vec![(true,2),(false,1)],"X and target count are distinct stage-owned decisions");
  assert_eq!(game.stack.last().unwrap().x_value,Some(2));
  assert_eq!(game.stack.last().unwrap().target_assignments[0].range.len(),1);
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(B).unwrap().life,22);assert_eq!(game.player(A).unwrap().life,21);
 }}
}
