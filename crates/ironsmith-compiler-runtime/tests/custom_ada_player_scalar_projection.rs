//! Exact Ada plus player scalar families through native and artifact routes.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Target,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices{extra:bool}
impl DecisionMaker for Choices {
 fn decide_number(&mut self,_:&GameState,c:&ironsmith::decisions::context::NumberContext)->u32{if self.extra{c.max.min(2).max(c.min)}else{c.min}}
 fn decide_targets(&mut self,_:&GameState,c:&ironsmith::decisions::context::TargetsContext)->Vec<Target>{let n=c.requirements.iter().map(|r|r.min_targets).sum::<usize>();if n==2{vec![Target::Player(A),Target::Player(B)]}else{vec![Target::Player(B)]}}
 fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(g,c)}
}
fn cast(game:&mut GameState,card:ObjectId,dm:&mut Choices){
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==card)).expect("native chooser budget admits this scalar role");
 let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(c)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&c,dm).unwrap();}panic!("casting did not finish");
}
fn providers()->[CardDefinition;2]{
 let f:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();let card=f["mechanics"].as_array().unwrap().iter().flat_map(|m|m["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]=="Ada, the Free_OPH").unwrap();source_routes::definitions(card).unwrap()
}
#[test]
fn draw_mill_and_discard_retain_each_announced_player_and_execute_sibling_once(){
 let bodies=source_routes::definitions(&serde_json::json!({"name":"Player resource","executable_text":"Type: Artifact"})).unwrap();
 for provider in providers(){for (kind,instruction) in [("draw","Target player draws a card."),("mill","Target player mills two cards."),("discard","Target player discards a card."),("random","Target player discards a card at random."),("filtered","Target player discards an artifact card."),("loot","Target player draws a card, then discards a card.")]{
 let spells=source_routes::definitions(&serde_json::json!({"name":"Player role witness","executable_text":format!("Mana cost: {{0}}\nType: Instant\n{instruction}\nYou gain 1 life.")})).unwrap();
 for (index,spell) in spells.into_iter().enumerate(){for extra in [false,true]{for copy in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  let source=game.create_object_from_definition(&provider,A,Zone::Battlefield);
  for player in [A,B]{for _ in 0..3{game.create_object_from_definition(&bodies[index],player,Zone::Hand);}for _ in 0..6{game.create_object_from_definition(&bodies[index],player,Zone::Library);}}
  let card=game.create_object_from_definition(&spell,A,Zone::Hand);let mut dm=Choices{extra};cast(&mut game,card,&mut dm);assert_eq!(game.stack.last().unwrap().target_assignments[0].range.len(),if extra{2}else{1});
  game.move_object_by_effect(source,Zone::Graveyard).unwrap();
  if copy{let original=game.stack.last().unwrap().object_id;ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(original))),&mut ironsmith::effects::EffectContext::new_default(original,A)).unwrap();assert_eq!(game.stack.last().unwrap().target_assignments[0].range.len(),if extra{2}else{1});}
  while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  let n=if copy{2}else{1};assert_eq!(game.player(A).unwrap().life,20+n,"untargeted sibling is outside the projected scalar leaf");assert_eq!(game.player(B).unwrap().life,20);
  for player in [A,B]{let selected=player==B||extra;let repetitions=if selected{n as usize}else{0};let hand=game.player(player).unwrap().hand.len();let library=game.player(player).unwrap().library.len();
   match kind{"draw"=>{assert_eq!(hand,3+repetitions);assert_eq!(library,6-repetitions)},"mill"=>{assert_eq!(hand,3);assert_eq!(library,6-2*repetitions)},"loot"=>{assert_eq!(hand,3);assert_eq!(library,6-repetitions)},_=>{assert_eq!(hand,3-repetitions);assert_eq!(library,6)}}
  }
 }}}
 }}
}
