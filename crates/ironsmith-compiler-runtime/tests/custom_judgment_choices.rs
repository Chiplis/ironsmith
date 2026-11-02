//! Generic self named choices preserve complete branch and object-loop scope.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
const B:PlayerId=PlayerId(1);
fn definitions(text:&str)->[CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Named choice",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());
 let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(restored,artifact);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
struct Choices{next:usize,decisions:usize}
impl DecisionMaker for Choices {
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
  self.decisions+=1;vec![ctx.options.iter().filter(|o|o.legal).nth(self.next).unwrap().index]
 }
}
fn body()->CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition("Body","Type: Creature\nPower/Toughness: 2/2",false).unwrap()}
const PROGRAM:&str="Choose target creature you control. You choose honor or condemn. If you chose honor, put a +1/+1 counter on that creature. If you chose condemn, sacrifice that creature, then draw a card.";
#[test]
fn self_named_choice_keeps_counter_and_sacrifice_draw_branches_exclusive_even_if_sacrifice_is_prevented(){
 use ironsmith::effects::ResolvedTarget;
 use ironsmith::object::CounterType;
 use ironsmith::target::ObjectFilter;
 for definition in definitions(&format!("Type: Sorcery\n{PROGRAM}")){for option in [0,1]{for prevent in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let recipient=game.create_object_from_definition(&body(),A,Zone::Battlefield);
  let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);
  game.create_object_from_definition(&body(),A,Zone::Library);
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  if prevent {execute_effect(&mut game,&Effect::new(ironsmith::effects::CantEffect::new(ironsmith::effect::Restriction::be_sacrificed(ObjectFilter::specific(recipient)),ironsmith::effect::Until::EndOfTurn)),&mut EffectContext::new_default(source,A)).unwrap();game.update_cant_effects();}
  let mut dm=Choices{next:option,decisions:0};
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(dm.decisions,1);
  assert_eq!(game.player(A).unwrap().hand.len(),usize::from(option==1));
  assert_eq!(game.battlefield.contains(&recipient),option==0||prevent);
  if option==0 {assert_eq!(game.counter_count(recipient,CounterType::PlusOnePlusOne),1);}
  assert!(game.battlefield.contains(&enemy));assert_eq!(game.counter_count(enemy,CounterType::PlusOnePlusOne),0);
 }}}
}
#[test]
fn named_choice_and_complete_branch_repeat_for_each_object_with_no_extra_player_choices(){
 use ironsmith::object::CounterType;
 let text="Type: Sorcery\nFor each creature you control, you choose honor or condemn. If you chose honor, put a +1/+1 counter on that creature. If you chose condemn, sacrifice that creature, then draw a card.";
 for definition in definitions(text){for option in [0,1]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let own=[game.create_object_from_definition(&body(),A,Zone::Battlefield),game.create_object_from_definition(&body(),A,Zone::Battlefield)];
  let hand=game.create_object_from_definition(&body(),A,Zone::Hand);let grave=game.create_object_from_definition(&body(),A,Zone::Graveyard);
  let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);
  for _ in 0..2 {game.create_object_from_definition(&body(),A,Zone::Library);}
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let mut dm=Choices{next:option,decisions:0};let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(dm.decisions,2);assert_eq!(game.player(A).unwrap().hand.len(),if option==1{3}else{1});
  for id in own {assert_eq!(game.battlefield.contains(&id),option==0);if option==0{assert_eq!(game.counter_count(id,CounterType::PlusOnePlusOne),1);}}
  assert!(game.battlefield.contains(&enemy));assert!(game.object(enemy).unwrap().counters.is_empty());
  assert!(game.player(A).unwrap().hand.contains(&hand));assert_eq!(game.counter_count(hand,CounterType::PlusOnePlusOne),0);assert_eq!(game.object(grave).unwrap().zone,Zone::Graveyard);assert_eq!(game.counter_count(grave,CounterType::PlusOnePlusOne),0);
 }}
}

#[test]
fn named_choice_designation_receipt_survives_prevented_sacrifice_and_only_emits_the_selected_option(){
 use ironsmith::effects::ResolvedTarget;
 use ironsmith::target::ObjectFilter;
 let text="Type: Sorcery\nChoose target creature you control. You choose honor or condemn. If you chose honor, perform the honor action with that creature, then put a +1/+1 counter on that creature. If you chose condemn, perform the condemn action with that creature, then sacrifice that creature, then draw a card.";
 let watcher=ironsmith_compiler_runtime::compile_to_runtime_definition("Witness","Type: Enchantment\nWhenever you perform the condemn action with a creature, each opponent loses 1 life.",false).unwrap();
 for definition in definitions(text){for option in [0,1]{for prevent in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&watcher,A,Zone::Battlefield);let recipient=game.create_object_from_definition(&body(),A,Zone::Battlefield);game.create_object_from_definition(&body(),A,Zone::Library);let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  if prevent{execute_effect(&mut game,&Effect::new(ironsmith::effects::CantEffect::new(ironsmith::effect::Restriction::be_sacrificed(ObjectFilter::specific(recipient)),ironsmith::effect::Until::EndOfTurn)),&mut EffectContext::new_default(source,A)).unwrap();game.update_cant_effects();}
  let mut dm=Choices{next:option,decisions:0};let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
  let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),usize::from(option==1));
 }}}
}
