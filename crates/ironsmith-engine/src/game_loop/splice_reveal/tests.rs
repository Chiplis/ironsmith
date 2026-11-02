use super::*;
use crate::{Ability,CardDefinition,CardDefinitionBuilder,CardId,CardType,Subtype};
use crate::decision::{SelectFirstDecisionMaker,compute_legal_actions};
use crate::static_abilities::{CompiledStaticAbility,StaticAbility,SpliceSpec,SpliceQuality};
use crate::game_state::Phase;
const A:PlayerId=PlayerId::from_index(0);
#[derive(Default)]
struct Choices {selected:Vec<ObjectId>,answer:bool,views:usize,splice_prompts:usize}
impl DecisionMaker for Choices {
 fn decide_objects(&mut self,game:&GameState,ctx:&crate::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
  if ctx.description.starts_with("Reveal cards to splice"){self.splice_prompts+=1;self.selected.clone()}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
 }
 fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{self.answer}
 fn view_cards(&mut self,_:&GameState,_:PlayerId,_:&[ObjectId],ctx:&crate::decisions::context::ViewCardsContext){if ctx.public&&ctx.description.contains("spliced"){self.views+=1;}}
}
// Engine unit tests exercise native identity and immutable-clone behavior.
// Actual artifact roundtrip scenarios live in compiler-runtime/custom_reprise.
fn route(def:CardDefinition,clone_definition:bool)->CardDefinition {
 if clone_definition {def.clone()} else {def}
}
fn setup(clone_definition:bool,count:usize,nested:bool,reprise:bool)->(GameState,TriggerQueue,PriorityLoopState,Choices,ObjectId,Vec<crate::ids::StableId>){
 let mut game=crate::tests::test_helpers::setup_two_player_game();game.turn.phase=Phase::FirstMain;game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);
 let main=CardDefinitionBuilder::new(CardId::new(),"Main Arcane Spell").mana_cost(crate::mana::ManaCost::new()).card_types(vec![CardType::Sorcery]).subtypes(vec![Subtype::Arcane]).with_spell_effect(vec![Effect::gain_life(1)]).build();
 let main=game.create_object_from_definition(&route(main,clone_definition),A,Zone::Hand);
 let mut selected=Vec::new();let mut stable=Vec::new();
 for _ in 0..count{
  let mut effects=vec![Effect::new(crate::effects::ExileEffect::with_spec(crate::target::ChooseSpec::Source))];
  if nested{effects.push(Effect::may(vec![Effect::gain_life(4)]));}
  let offer=StaticAbility::from_model(CompiledStaticAbility::reveal_action_offer("revealed splice offer",vec![Zone::Hand],Some(1),Zone::Exile,effects,"Exile this revealed card"));
  let splice=StaticAbility::splice(SpliceSpec{custom_keyword:reprise.then(||"Reprise".into()),source_zone:reprise.then_some(Zone::Graveyard),source_destination:reprise.then_some(Zone::Exile),max_per_spell:reprise.then_some(1),descriptive:false,quality:SpliceQuality::Arcane,cost:crate::cost::TotalCost::mana(crate::mana::ManaCost::new()),cost_surface:None});
  let card=CardDefinitionBuilder::new(CardId::new(),"Revealed splice donor").mana_cost(crate::mana::ManaCost::new()).card_types(vec![CardType::Instant]).with_ability(Ability::static_ability(splice).in_zones(vec![Zone::Hand,Zone::Graveyard])).with_ability(Ability::static_ability(offer)).with_spell_effect(vec![Effect::gain_life(4)]).build();
  let card=game.create_object_from_definition(&route(card,clone_definition),A,if reprise{Zone::Graveyard}else{Zone::Hand});
  selected.push(card);stable.push(game.object(card).unwrap().stable_id);
 }
 game.take_pending_trigger_events();
 (game,TriggerQueue::new(),PriorityLoopState::new(2),Choices{selected,..Default::default()},main,stable)
}
fn start(game:&mut GameState,queue:&mut TriggerQueue,state:&mut PriorityLoopState,dm:&mut Choices,main:ObjectId)->GameProgress{
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==main)).unwrap();
 let progress=apply_priority_response_with_dm(game,queue,state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 let GameProgress::NeedsDecisionCtx(ctx@DecisionContext::SelectObjects(_))=progress else{panic!("expected actual splice selection: {progress:?}")};
 apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap()
}
fn answer(game:&mut GameState,queue:&mut TriggerQueue,state:&mut PriorityLoopState,dm:&mut Choices,progress:GameProgress,value:bool)->GameProgress{
 let GameProgress::NeedsDecisionCtx(ctx@DecisionContext::Boolean(_))=progress else{panic!("expected actual Boolean response channel: {progress:?}")};
 dm.answer=value;apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap()
}
fn finish(game:&mut GameState,queue:&mut TriggerQueue,state:&mut PriorityLoopState,dm:&mut Choices,mut progress:GameProgress){
 for _ in 0..64{
  if !state.has_pending_action(){return}
  let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("pending cast lost its response context: {progress:?}")};
  progress=apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap();
 }
 panic!("splice continuation did not finish")
}
fn completed_offers(game:&GameState)->usize{game.turn_store.turn_history.projected_records().filter(|record|record.event.downcast::<crate::events::NamedActionEvent>().is_some_and(|event|event.label=="revealed splice offer")).count()}
fn reveal_count(game:&GameState)->usize{game.turn_store.turn_history.projected_records().filter(|record|record.event.downcast::<crate::events::CardRevealedEvent>().is_some()).count()}
#[test]
fn decline_resumes_original_splice_once_through_real_boolean_channel(){
 for clone_definition in [false,true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(clone_definition,1,false,false);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);
  assert_eq!(dm.splice_prompts,1);assert_eq!(dm.views,2);assert!(state.pending_cast.as_ref().unwrap().splice_reveal.is_some());
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,false);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(dm.splice_prompts,1);assert_eq!(dm.views,2);assert_eq!(completed_offers(&game),0);assert_eq!(reveal_count(&game),1);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Hand);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,25);
 }
}
#[test]
fn accepted_offer_and_nested_decision_replay_only_the_private_child(){
 for clone_definition in [false,true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(clone_definition,1,true,false);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);
  let GameProgress::NeedsDecisionCtx(original_prompt)=&progress else{panic!("expected outer offer")};
  let original_prompt=original_prompt.clone();
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,true);
  assert!(matches!(progress,GameProgress::NeedsDecisionCtx(DecisionContext::Boolean(_))));
  assert!(apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&original_prompt,&mut dm).is_err(),"an old outer Boolean cannot answer the child decision");
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Hand);
  assert_eq!(game.player(A).unwrap().life,20);assert_eq!(completed_offers(&game),0);assert_eq!(dm.views,2);
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,true);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Exile);
  assert_eq!(game.player(A).unwrap().life,24);assert_eq!(completed_offers(&game),1);assert_eq!(reveal_count(&game),1);assert_eq!(dm.views,2);assert_eq!(dm.splice_prompts,1);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,29);
 }
}
#[test]
fn quota_rechecks_next_frozen_donor_without_revealing_or_splicing_again(){
 for clone_definition in [false,true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(clone_definition,2,false,false);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);
  assert_eq!(state.pending_cast.as_ref().unwrap().splice_reveal.as_ref().unwrap().selected.len(),2);
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,true);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(completed_offers(&game),1);assert_eq!(reveal_count(&game),2);assert_eq!(dm.views,2);assert_eq!(dm.splice_prompts,1);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Exile);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[1]).unwrap()).unwrap().zone,Zone::Hand);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,29);
 }
}
#[test]
fn reprise_moving_source_cost_is_not_reinterpreted_as_a_hand_reveal_offer(){
 for clone_definition in [false,true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(clone_definition,1,false,true);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(completed_offers(&game),0);assert_eq!(reveal_count(&game),0);assert_eq!(dm.splice_prompts,1);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Exile);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,25);
 }
}
