//! Recipient-local turn prevention budgets are distinct from per-event absorb.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone,Value};
use ironsmith::effects::{execute_effect,EffectContext,DealDamageEffect,DealDamageBySourcesEffect};
use ironsmith::target::ChooseSpec;
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(all_but:bool)->[CardDefinition;2]{
 let prefix=if all_but{"all but the first"}else{"the first"};let text=format!("Type: Creature\nPower/Toughness: 2/20\nPrevent {prefix} 1 damage that would be dealt to this creature each turn.");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Selo",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn setup(definition:&CardDefinition)->(GameState,ObjectId,ObjectId,ObjectId){let mut game=GameState::new(vec!["A".into(),"B".into()],20);let protected=game.create_object_from_definition(definition,A,Zone::Battlefield);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Damage witness","Type: Creature\nPower/Toughness: 3/20",false).unwrap();let x=game.create_object_from_definition(&body,B,Zone::Battlefield);let y=game.create_object_from_definition(&body,B,Zone::Battlefield);(game,protected,x,y)}
fn hit(game:&mut GameState,target:ObjectId,source:ObjectId,unpreventable:bool,dm:&mut dyn DecisionMaker){let effect=Effect::new(DealDamageEffect::new(3,ChooseSpec::SpecificObject(target)).with_unpreventable(unpreventable));execute_effect(game,&effect,&mut EffectContext::new_default(source,B).with_decision_maker(dm)).unwrap();}
#[test]
fn first_damage_budget_is_shared_by_simultaneous_sources_and_resets_each_turn(){
 for definition in definitions(false){let(mut game,target,x,y)=setup(&definition);let effect=Effect::new(DealDamageBySourcesEffect::new(vec![ChooseSpec::SpecificObject(x),ChooseSpec::SpecificObject(y)],Value::Fixed(3),ChooseSpec::SpecificObject(target)));execute_effect(&mut game,&effect,&mut EffectContext::new_default(x,B).with_decision_maker(&mut SelectFirstDecisionMaker)).unwrap();assert_eq!(game.damage_on(target),5);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),8);game.next_turn();let before=game.damage_on(target);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target)-before,2);}
}
#[test]
fn dealt_damage_cap_covers_all_simultaneous_sources_and_later_events(){
 for definition in definitions(true){let(mut game,target,x,y)=setup(&definition);let effect=Effect::new(DealDamageBySourcesEffect::new(vec![ChooseSpec::SpecificObject(x),ChooseSpec::SpecificObject(y)],Value::Fixed(3),ChooseSpec::SpecificObject(target)));execute_effect(&mut game,&effect,&mut EffectContext::new_default(x,B).with_decision_maker(&mut SelectFirstDecisionMaker)).unwrap();assert_eq!(game.damage_on(target),1);hit(&mut game,target,y,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),1);game.next_turn();let before=game.damage_on(target);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target)-before,1);}
}
struct ExternalFirst;
impl DecisionMaker for ExternalFirst{fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{ctx.options.iter().find(|o|o.legal&&o.description.contains("External prevention witness")).or_else(||ctx.options.iter().find(|o|o.legal)).map(|o|vec![o.index]).unwrap_or_default()}}
#[test]
fn other_prevention_does_not_spend_dealt_damage_cap_and_unpreventable_damage_counts(){
 for definition in definitions(true){let(mut game,target,x,_)=setup(&definition);let source=ironsmith_compiler_runtime::compile_to_runtime_definition("External prevention witness","Type: Enchantment",false).unwrap();let source=game.create_object_from_definition(&source,A,Zone::Battlefield);execute_effect(&mut game,&Effect::new(ironsmith::effects::PreventDamageEffect::new(3,ChooseSpec::SpecificObject(target),ironsmith::effect::Until::EndOfTurn)),&mut EffectContext::new_default(source,A)).unwrap();hit(&mut game,target,x,false,&mut ExternalFirst);assert_eq!(game.damage_on(target),0);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),1);
 let(mut game,target,x,_)=setup(&definition);hit(&mut game,target,x,true,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),3);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),3);}
}
#[test]
fn damage_cap_belongs_to_current_recipient_incarnation(){for definition in definitions(true){let(mut game,target,x,_)=setup(&definition);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);let stable=game.object(target).unwrap().stable_id;execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(target),Zone::Graveyard,false),&mut EffectContext::new_default(x,B)).unwrap();let grave=game.find_object_by_stable_id(stable).unwrap();execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(grave),Zone::Battlefield,false),&mut EffectContext::new_default(x,B)).unwrap();let new=game.find_object_by_stable_id(stable).unwrap();assert_ne!(new,target);hit(&mut game,new,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(new),1);}}
#[test]
fn exact_copperhide_level_band_and_back_face_preserve_different_turn_budgets(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-turn-prevention-source.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-411").unwrap();let faces=row["representative_cards"][0]["executable_faces"].as_array().unwrap();assert_eq!(faces.len(),2);
 for (index,face) in faces.iter().enumerate(){let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(face["name"].as_str().unwrap(),face["executable_text"].as_str().unwrap(),false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]{for level in [0,1]{let(mut game,target,x,_)=setup(&definition);if level>0{game.add_counters(target,ironsmith::object::CounterType::Level,level).unwrap();}hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),if index==1{1}else if level==1{2}else{3});if index==0&&level==0 {continue;}let previous=game.damage_on(target);hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target)-previous,if index==1{0}else{3});}}}
}
#[test]
fn first_single_point_is_not_recreated_by_static_refresh_control_change_or_reacquisition() {
 for definition in definitions(false) {
  let(mut game,target,x,_)=setup(&definition);
  let point=Effect::new(DealDamageEffect::new(1,ChooseSpec::SpecificObject(target)));
  execute_effect(&mut game,&point,&mut EffectContext::new_default(x,B)).unwrap();assert_eq!(game.damage_on(target),0);
  let abilities=game.object(target).unwrap().abilities.clone();std::sync::Arc::make_mut(&mut game.object_mut(target).unwrap().abilities).clear();game.refresh_continuous_state().unwrap();game.object_mut(target).unwrap().abilities=abilities;
  game.set_current_controller(target,B).unwrap();game.refresh_continuous_state().unwrap();
  execute_effect(&mut game,&point,&mut EffectContext::new_default(x,B)).unwrap();assert_eq!(game.damage_on(target),1,"one already prevented point spends the same occurrence's first-damage budget");
 }
}
#[test]
fn unpreventable_earlier_damage_passes_the_first_damage_window() {
 for definition in definitions(false) {
  let(mut game,target,x,_)=setup(&definition);hit(&mut game,target,x,true,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),3);
  hit(&mut game,target,x,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(target),6,"first damage each turn differs from a next-damage shield");
 }
}
#[test]
fn a_fully_prevented_single_point_in_simultaneous_damage_spends_the_exact_occurrence_budget() {
 for definition in definitions(false) {
  let(mut game,target,x,y)=setup(&definition);
  let effect=Effect::new(DealDamageBySourcesEffect::new(vec![ChooseSpec::SpecificObject(x),ChooseSpec::SpecificObject(y)],Value::Fixed(1),ChooseSpec::SpecificObject(target)));
  execute_effect(&mut game,&effect,&mut EffectContext::new_default(x,B).with_decision_maker(&mut SelectFirstDecisionMaker)).unwrap();assert_eq!(game.damage_on(target),1);
  let point=Effect::new(DealDamageEffect::new(1,ChooseSpec::SpecificObject(target)));execute_effect(&mut game,&point,&mut EffectContext::new_default(x,B)).unwrap();assert_eq!(game.damage_on(target),2);
 }
}
#[test]
fn gaining_the_calf_level_band_after_damage_does_not_rewind_the_turns_first_point() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-turn-prevention-source.json")).unwrap();let face=&fixture["mechanics"][0]["representative_cards"][0]["executable_faces"][0];
 let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(face["name"].as_str().unwrap(),face["executable_text"].as_str().unwrap(),false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
  let(mut game,target,x,_)=setup(&definition);let point=Effect::new(DealDamageEffect::new(1,ChooseSpec::SpecificObject(target)));
  execute_effect(&mut game,&point,&mut EffectContext::new_default(x,B)).unwrap();assert_eq!(game.damage_on(target),1);
  game.add_counters(target,ironsmith::CounterType::Level,1).unwrap();execute_effect(&mut game,&point,&mut EffectContext::new_default(x,B)).unwrap();assert_eq!(game.damage_on(target),2);
 }
}

#[test]
fn shared_plural_or_union_turn_budgets_fail_closed_while_explicit_each_is_recipient_local() {
 for prefix in ["the first","all but the first"] {
  for recipient in ["creatures you control","you and creatures you control","you or a creature you control"] {
   let text=format!("Type: Enchantment\nPrevent {prefix} 1 damage that would be dealt to {recipient} each turn.");
   assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Selo",text,false).is_err(),"ambiguous shared budget accepted for {recipient}");
  }
  let text=format!("Type: Enchantment\nPrevent {prefix} 1 damage that would be dealt to each creature you control each turn.");
  let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Selo",text,false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
  for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
   let(mut game,source,x,y)=setup(&definition);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 1/20",false).unwrap();let first=game.create_object_from_definition(&body,A,Zone::Battlefield);let second=game.create_object_from_definition(&body,A,Zone::Battlefield);
   hit(&mut game,first,x,false,&mut SelectFirstDecisionMaker);hit(&mut game,second,y,false,&mut SelectFirstDecisionMaker);assert_eq!(game.damage_on(first),if prefix=="the first"{2}else{1});assert_eq!(game.damage_on(second),game.damage_on(first));assert_eq!(game.damage_on(source),0);
  }
 }
}

#[path="custom_final_next_wave_support.rs"] mod level_source_support;
#[test]
fn named_source_trigger_in_level_band_keeps_the_enclosing_source_and_range() {
 for name in ["Level-band witness", "Another named permanent"] {
  let input=serde_json::json!({"name":name,"executable_text":format!("Type: Creature\nPower/Toughness: 2/2\nLEVEL 3+\nWhenever {name} deals combat damage to a player, draw a card.\n7/7")});
  for definition in level_source_support::definitions(&input).unwrap() {
   for level in [0,2,3,4] {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);let a=PlayerId(0);
    let source=game.create_object_from_definition(&definition,a,Zone::Battlefield);
    if level>0{game.add_counters(source,ironsmith::CounterType::Level,level).unwrap();}
    let triggers=game.current_abilities(source).unwrap().iter().filter(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Triggered(_))).count();
    assert_eq!(triggers,usize::from(level>=3),"{name}: level {level}");
    assert_eq!(game.calculated_power(source),Some(if level>=3{7}else{2}));
   }
  }
 }
}
