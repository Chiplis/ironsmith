//! Actual complete sources, with a retained original Role menu at the operation boundary.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect,NamedActionProgramEffect};
#[path="custom_final_next_wave_support.rs"] mod source_support;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixtures()->Vec<serde_json::Value>{vec![
 serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-audition-choices.json")).unwrap(),
 serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-audition-original-menu-modifier.json")).unwrap(),
]}
fn source(name:&str)->[CardDefinition;2]{
 let fixtures=fixtures();let card=fixtures.iter().flat_map(|fixture|fixture["mechanics"].as_array().unwrap()).flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
 source_support::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))
}
#[test]
fn all_thirty_operative_sources_are_strict_on_independent_native_and_artifact_routes(){
 let mut keys=std::collections::BTreeSet::new();
 for fixture in fixtures(){for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
  assert!(keys.insert(card["source_card_key"].as_str().unwrap().to_owned()));
  for pair in source_support::linked_definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"])){
   for definition in pair{assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));}
  }
 }}}
 assert_eq!(keys.len(),30);
}
fn collect(effect:&Effect,output:&mut Vec<Effect>){
 if effect.downcast_ref::<NamedActionProgramEffect>().is_some_and(|program|program.label=="audition"){output.push(effect.clone());return;}
 effect.0.visit_child_effects(&mut |child|collect(child,output));
}
fn audition(definition:&CardDefinition)->Effect{
 let mut found=Vec::new();
 if let Some(spell)=&definition.spell_effect{for effect in spell.all_effects(){collect(effect,&mut found);}}
 for ability in &definition.abilities{match &ability.kind{
  ironsmith::ability::AbilityKind::Triggered(trigger)=>for effect in trigger.effects.all_effects(){collect(effect,&mut found);},
  ironsmith::ability::AbilityKind::Activated(activation)=>for effect in activation.effects.all_effects(){collect(effect,&mut found);},_=>{}
 }}
 found.into_iter().next().expect("actual source retains named Audition program")
}
struct Choices{role:usize,menus:Vec<Vec<String>>}
impl DecisionMaker for Choices{
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
  let role_menu=ctx.options.iter().any(|option|option.description.contains("Role"));
  if role_menu{self.menus.push(ctx.options.iter().map(|option|option.description.clone()).collect());}
  vec![ctx.options.iter().filter(|option|option.legal).nth(if role_menu{self.role}else{0}).expect("original legal choice available").index]
 }
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{ctx.candidates.iter().find(|candidate|candidate.legal&&game.object(candidate.id).is_some_and(|object|object.kind==ironsmith::object::ObjectKind::Token&&!object.subtypes.contains(&ironsmith::Subtype::Role))).map(|candidate|vec![candidate.id]).unwrap_or_else(||SelectFirstDecisionMaker.decide_objects(game,ctx))}
}
fn tokens(game:&GameState,player:PlayerId)->Vec<ObjectId>{game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|object|object.controller==player&&object.kind==ironsmith::object::ObjectKind::Token)).collect()}
#[test]
fn actual_modifier_preserves_all_six_original_role_menus_and_does_not_repeat_with_two_providers(){
 let cases=[("Hope Spot",vec!["Cleric","Scout"]),("Newswright",vec!["Warlock","Wizard"]),("Backdrop Artisan",vec!["Warrior","Wizard"]),("Prop Work",vec!["Scout","Warrior"]),("Sculpt the Scene",vec!["Cleric","Warlock"]),("Theatric Tandem",vec!["Warlock","Warrior","Wizard"])];
 let providers=source("Verdant Curtain Prodigy");
 for (name,roles) in cases{let programs=source(name);for route in 0..2{for provider_count in [1,2]{for selected in 0..roles.len(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  for _ in 0..provider_count{game.create_object_from_definition(&providers[route],A,Zone::Battlefield);}
  let source=game.create_object_from_definition(&programs[route],A,Zone::Stack);
  let mut dm=Choices{role:selected,menus:Vec::new()};
  execute_effect(&mut game,&audition(&programs[route]),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  let created=tokens(&game,A);assert_eq!(created.len(),2,"{name} providers={provider_count}");
  assert_eq!(created.iter().filter(|id|game.object(**id).unwrap().subtypes.contains(&ironsmith::Subtype::Role)).count(),1);
  let role=created.iter().copied().find(|id|game.object(*id).unwrap().subtypes.contains(&ironsmith::Subtype::Role)).unwrap();
  let actor=created.iter().copied().find(|id|*id!=role).unwrap();
  assert_eq!(game.object(role).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(actor)),"Actor created before Role host selection");
  let labels=dm.menus.last().expect("Role choice after Actor");assert_eq!(labels.len(),roles.len());
  for role in &roles{assert!(labels.iter().any(|label|label.contains(&format!("{role} Role"))),"{name}: {labels:?}");}
  assert!(labels.iter().all(|label|!label.contains("Construct Actor")));
 }}}}
}
#[test]
fn modifier_applies_only_to_its_controllers_operation(){
 let providers=source("Verdant Curtain Prodigy");let programs=source("Hope Spot");
 for route in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&providers[route],A,Zone::Battlefield);
  let source=game.create_object_from_definition(&programs[route],B,Zone::Stack);let mut dm=Choices{role:0,menus:Vec::new()};
  execute_effect(&mut game,&audition(&programs[route]),&mut EffectContext::new_default(source,B).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(tokens(&game,B).len(),1);assert!(tokens(&game,A).is_empty());
 }
}

#[test]
fn accepted_mode_plan_survives_provider_departure_before_its_original_menu_executes(){
 let providers=source("Verdant Curtain Prodigy");let programs=source("Hope Spot");
 for route in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let provider=game.create_object_from_definition(&providers[route],A,Zone::Battlefield);
  let source=game.create_object_from_definition(&programs[route],A,Zone::Stack);
  let effect=audition(&programs[route]);let mut operation=effect.downcast_ref::<NamedActionProgramEffect>().unwrap().clone();
  // A generic earlier instruction removes the source after proposal/CR616,
  // before either original mode. The accepted plan is not reacquired later.
  operation.effects.insert(0,Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(provider),Zone::Graveyard,false));
  let mut dm=Choices{role:0,menus:Vec::new()};
  execute_effect(&mut game,&Effect::new(operation),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  let created=tokens(&game,A);assert_eq!(created.len(),2);
  assert!(!game.battlefield.contains(&provider));
  let role=created.iter().copied().find(|id|game.object(*id).unwrap().subtypes.contains(&ironsmith::Subtype::Role)).unwrap();
  let actor=created.into_iter().find(|id|*id!=role).unwrap();
  assert_eq!(game.object(role).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(actor)));
 }
}

#[test]
fn generic_original_mode_policy_is_typed_and_rejects_unread_or_invalid_mode_surfaces(){
 for definition in source_support::definitions(&serde_json::json!({"name":"Independent mode modifier","executable_text":"Type: Enchantment\nIf you would perform the assess action, perform its mode numbered 2, then choose one of its other modes instead."})).unwrap(){
  let policy=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Static(ability)=>ability.compiled_model(),_=>None}).unwrap();
  assert!(matches!(&policy.payload,ironsmith_core::StaticAbilityPayload::NamedActionFirstThenOtherMode{label,player:ironsmith::target::PlayerFilter::You,first_mode:1,..}if label=="assess"));
 }
 for phrase in ["perform its mode numbered 0, then choose one of its other modes","perform its mode numbered 1, then choose two of its other modes","perform its mode numbered 1, then choose one of its other modes and draw a card"]{
  assert!(source_support::definitions(&serde_json::json!({"name":"Invalid mode modifier","executable_text":format!("Type: Enchantment\nIf you would perform the assess action, {phrase} instead.")})).is_err());
 }
}

#[test]
fn actual_hope_spot_retains_role_creation_receipt_for_its_following_untap(){
 struct HostChoice{host:ObjectId,choices:Choices}
 impl DecisionMaker for HostChoice{
  fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{self.choices.decide_options(game,ctx)}
  fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
   if ctx.candidates.iter().any(|candidate|candidate.legal&&candidate.id==self.host){vec![self.host]}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
  }
 }
 let providers=source("Verdant Curtain Prodigy");let spells=source("Hope Spot");
 for route in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let provider=game.create_object_from_definition(&providers[route],A,Zone::Battlefield);game.tap(provider);
  let source=game.create_object_from_definition(&spells[route],A,Zone::Stack);
  let effect=Effect::new(ironsmith::effects::SequenceEffect::new(spells[route].spell_effect.as_ref().unwrap().all_effects_owned()));
  let mut dm=HostChoice{host:provider,choices:Choices{role:0,menus:Vec::new()}};
  execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(tokens(&game,A).len(),2);
  assert!(!game.is_tapped(provider),"the actual source's Role-this-way followup must read the retained original chosen Role result");
 }
}

#[test]
fn actual_optional_prop_custodian_decline_never_proposes_or_completes_a_modified_audition(){
 struct MayChoices{accept:bool,calls:usize,choices:Choices}
 impl DecisionMaker for MayChoices{
  fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.calls+=1;self.accept}
  fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{self.choices.decide_options(game,ctx)}
  fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{self.choices.decide_objects(game,ctx)}
 }
 let providers=source("Verdant Curtain Prodigy");let sources=source("Prop Custodian");
 for route in 0..2{for provider_count in [1,2]{for accepted in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  for _ in 0..provider_count{game.create_object_from_definition(&providers[route],A,Zone::Battlefield);}
  let source=game.create_object_from_definition(&sources[route],A,Zone::Battlefield);
  let trigger=sources[route].abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).expect("actual complete Prop Custodian trigger");
  let effect=Effect::new(ironsmith::effects::SequenceEffect::new(trigger.effects.all_effects_owned()));
  let mut dm=MayChoices{accept:accepted,calls:0,choices:Choices{role:0,menus:Vec::new()}};
  let outcome=execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(dm.calls,1,"one actual outer permission owner");
  assert_eq!(tokens(&game,A).len(),if accepted{2}else{0});
  assert_eq!(dm.choices.menus.len(),usize::from(accepted),"decline must not reach a Role menu or original operation proposal");
  assert_eq!(outcome.events.iter().filter(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|receipt|receipt.label=="audition")).count(),usize::from(accepted));
 }}}
}
