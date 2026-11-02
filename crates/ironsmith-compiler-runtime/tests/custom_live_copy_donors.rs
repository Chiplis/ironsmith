//! Live static copy donors share a finite layer-1 effect universe.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::types::CardType;
const A:PlayerId=PlayerId(0);
fn routes(name:&str,power:u8)->[CardDefinition;2]{
 let text=format!("Mana cost: {{2}}{{U}}\nType: Enchantment — Aura\nPower/Toughness: {power}/{power}\nEnchant permanent\nEnchanted permanent is a copy of this permanent except for its card types, supertypes, and subtypes.\nFlying");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||
  ironsmith_compiler::word_claims::with_enforcement_options(true,true,||
   ironsmith_compiler::CompilerFacade::new().compile_definition(
    ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name),
    text.clone(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());
 let direct=ironsmith_compiler_runtime::into_runtime_definition(result.unwrap().definition).unwrap();
 let (artifact,_)=ironsmith_compiler_runtime::compile_to_artifact(name,&text,false).unwrap();
 let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn host()->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Host","Type: Creature — Goblin\nPower/Toughness: 1/2",false).unwrap()}
fn attach(game:&mut GameState,source:ironsmith::ObjectId,target:ironsmith::ObjectId){assert!(game.attach_object_to_target(source,ironsmith::object::AttachmentTarget::Object(target)));}
#[test]
fn static_copy_donor_chain_uses_other_static_copies_and_frozen_copies_remain_frozen(){
 for (middle,top) in routes("Middle",3).into_iter().zip(routes("Top",7)){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let host=game.create_object_from_definition(&host(),A,Zone::Battlefield);
  let middle=game.create_object_from_definition(&middle,A,Zone::Battlefield);
  let top=game.create_object_from_definition(&top,A,Zone::Battlefield);
  attach(&mut game,middle,host);attach(&mut game,top,middle);
  for id in [host,middle,host]{
   let values=game.calculated_copiable_values(id).unwrap();
   assert_eq!(values.name,"Top");assert_eq!(values.power,Some(7));
   let current=game.current_characteristics(id).unwrap();
   assert_eq!(current.power,if id==host {Some(7)} else {None},"only the creature recipient has current power; the Aura's printed/copied power remains in its copiable envelope");
  }
  assert!(game.current_characteristics(host).unwrap().card_types.contains(&CardType::Creature));
  let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
  ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(ironsmith::target::ChooseSpec::SpecificObject(host),1,ironsmith::target::PlayerFilter::You)),&mut ironsmith::effects::EffectContext::new_default(top,A).with_decision_maker(&mut dm)).unwrap();
  let token=*game.battlefield.iter().find(|id|game.object(**id).is_some_and(|object|object.kind==ironsmith::object::ObjectKind::Token)).unwrap();
  // Moving the top donor away removes its static contribution. The middle
  // and host revert, but the independently resolved copy remains frozen.
  game.move_object(top,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();
  assert_eq!(game.calculated_copiable_values(host).unwrap().name,"Middle");
  assert_eq!(game.calculated_copiable_values(host).unwrap().power,Some(3));
  assert_eq!(game.calculated_copiable_values(token).unwrap().name,"Top");
  assert_eq!(game.calculated_copiable_values(token).unwrap().power,Some(7));
 }
}
#[test]
fn cyclic_static_copy_queries_are_finite_and_do_not_depend_on_prior_query_order(){
 for first_left in [false,true]{
  let left_def=routes("Earlier",3).into_iter().next().unwrap();
  let right_def=routes("Later",7).into_iter().next().unwrap();
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let left=game.create_object_from_definition(&left_def,A,Zone::Battlefield);
  let right=game.create_object_from_definition(&right_def,A,Zone::Battlefield);
  attach(&mut game,left,right);attach(&mut game,right,left);
  let order=if first_left{[left,right]}else{[right,left]};
  game.prewarm_calculated_characteristics(&order);
  let mut expected=None;
  for id in if first_left{[left,right,left,right]}else{[right,left,right,left]}{
   let values=game.calculated_copiable_values(id).unwrap();
   let current=game.current_characteristics(id).unwrap();
   assert_eq!(values.name,"Earlier");assert_eq!(values.power,Some(3));
   assert_eq!(current.name.as_ref(),values.name);
   if let Some((name,power))=&expected{assert_eq!(&values.name,name);assert_eq!(&values.power,power);}
   else{expected=Some((values.name.clone(),values.power));}
  }
 }
}
