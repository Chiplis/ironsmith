//! Actor-owned choices and real-card ownership remain aligned across routes.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
fn routes(text:&str)->[CardDefinition;2] {
 let pool=[("First design","Type: Sorcery\nDraw a card."),("Second design","Type: Sorcery\nGain 2 life.")].into_iter().map(|(name,text)|ironsmith_compiler::compile_supplied_card_definition(name,text,&[],&[],&[]).unwrap()).collect();
 let (artifact,direct)=ironsmith_compiler::with_supplied_card_definitions(pool,||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Actor source",text,false))).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()]
}
#[derive(Default)]struct OwnerChoices{accept:bool,booleans:Vec<PlayerId>,modes:Vec<PlayerId>}
impl ironsmith::decision::DecisionMaker for OwnerChoices {
 fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool{self.booleans.push(ctx.player);self.accept}
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{self.modes.push(ctx.player);vec![1]}
}
#[test]
fn noncontroller_owner_decides_optional_name_and_owns_created_card_on_both_routes(){
 let text="Type: Sorcery\nExile target creature. Its owner may do the following: [Choose one of the following: [Create a real card named \"First design\" in its owner's hand as part of the named making action] or [Create a real card named \"Second design\" in its owner's hand as part of the named making action]].";
 for definition in routes(text){for accept in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let target=ironsmith_compiler_runtime::compile_to_runtime_definition("Target","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
  let object=game.create_object_from_definition(&target,B,Zone::Battlefield);
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let mut dm=OwnerChoices{accept,..Default::default()};
  let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut dm);
  ctx.targets=vec![ironsmith::effects::ResolvedTarget::Object(object)];
  let mut events=Vec::new();for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){events.extend(execute_effect(&mut game,effect,&mut ctx).unwrap().events);}
  drop(ctx);
  assert_eq!(dm.booleans,vec![B]);assert!(game.player(A).unwrap().hand.is_empty());
  if accept {assert_eq!(dm.modes,vec![B]);let hand=&game.player(B).unwrap().hand;assert_eq!(hand.len(),1);assert_eq!(game.object(hand[0]).unwrap().name,"Second design");
   let actions=events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label=="making").collect::<Vec<_>>();assert_eq!(actions.len(),1);assert_eq!(actions[0].player,B);
  }else{assert!(dm.modes.is_empty());assert!(game.player(B).unwrap().hand.is_empty());assert!(!events.iter().any(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|event|event.label=="making")));}
 }}
}

#[test]
fn creation_count_binds_actual_prior_exiled_objects_on_both_routes(){
 for definition in routes("Type: Sorcery\nExile the top three cards of your library. Create the number of cards exiled this way real cards named \"First design\" in your hand as part of the named making action.") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Filler","Type: Sorcery\nGain 1 life.",false).unwrap();
  for _ in 0..3{game.create_object_from_definition(&filler,A,Zone::Library);}
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let mut ctx=EffectContext::new_default(source,A);let mut events=Vec::new();
  for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){events.extend(execute_effect(&mut game,effect,&mut ctx).unwrap().events);}
  assert_eq!(game.player(A).unwrap().hand.len(),3);
  assert_eq!(game.player(A).unwrap().library.len(),0);
  assert_eq!(events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).find(|event|event.label=="making").unwrap().amount,3);
 }
}
