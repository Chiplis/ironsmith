//! Named-choice token programs use ordinary Role Aura attachment and SBA rules.
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(program:&str)->[CardDefinition;2]{
 let text=format!("Type: Sorcery\n{program}");let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Audition choice",&text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(restored,artifact);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
struct Choices{option:usize}
impl DecisionMaker for Choices{
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{vec![ctx.options.iter().filter(|o|o.legal).nth(self.option).unwrap().index]}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
}
fn body()->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Actor host","Type: Creature — Bear\nPower/Toughness: 7/7",false).unwrap()}
fn execute(game:&mut GameState,definition:&CardDefinition,option:usize){let source=game.create_object_from_definition(definition,A,Zone::Stack);let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_decision_maker(&mut Choices{option})).unwrap();}
#[test]
fn all_sourced_color_choices_create_exactly_one_actor_or_role_and_exclude_enemy_hosts(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-audition-choices.json")).unwrap();
 for mechanic in fixture["mechanics"].as_array().unwrap(){let program=mechanic["executable_reminder"].as_str().unwrap();let clause=program.split('.').next().unwrap().trim_start_matches("You choose ");let options=clause.replace(" or ",", ").split(", ").map(str::to_owned).collect::<Vec<_>>();
  for definition in definitions(program){for (index,option) in options.iter().enumerate(){
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);let own=game.create_object_from_definition(&body(),A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);
   execute(&mut game,&definition,index);game.refresh_continuous_state().unwrap();
   let tokens=game.battlefield.iter().copied().filter(|id|matches!(game.object(*id).unwrap().kind,ironsmith::object::ObjectKind::Token)).collect::<Vec<_>>();assert_eq!(tokens.len(),1,"{option}");let token=game.object(tokens[0]).unwrap();
   if option=="actor"{assert!(token.subtypes.iter().any(|subtype|subtype.to_string().eq_ignore_ascii_case("actor")));assert_eq!(game.calculated_power(tokens[0]),Some(1));assert_eq!(token.attached_to,None);assert_eq!(game.calculated_power(own),Some(7));}
   else{assert!(token.subtypes.contains(&ironsmith::Subtype::Role));assert_eq!(token.attached_to,Some(ironsmith::object::AttachmentTarget::Object(own)));let (power,toughness)=match option.as_str(){"cleric"|"wizard"=>(2,2),"scout"=>(2,3),"warlock"=>(2,1),"warrior"=>(3,2),_=>panic!("{option}")};assert_eq!(game.calculated_power(own),Some(power));assert_eq!(game.calculated_toughness(own),Some(toughness));let keyword=match option.as_str(){"cleric"=>ironsmith::static_abilities::StaticAbilityId::Lifelink,"scout"=>ironsmith::static_abilities::StaticAbilityId::Vigilance,"warlock"=>ironsmith::static_abilities::StaticAbilityId::Flying,"wizard"=>ironsmith::static_abilities::StaticAbilityId::Prowess,"warrior"=>ironsmith::static_abilities::StaticAbilityId::Trample,_=>unreachable!()};assert!(game.calculated_characteristics(own).unwrap().static_abilities.iter().any(|a|a.id()==keyword));assert!(game.calculated_subtypes(own).iter().any(|subtype|subtype.to_string().eq_ignore_ascii_case(option)));}
   assert_eq!(game.calculated_power(enemy),Some(7));
  }}
 }
}
#[test]
fn latest_role_replaces_an_older_role_on_the_same_friendly_creature(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-audition-choices.json")).unwrap();let program=fixture["mechanics"][0]["executable_reminder"].as_str().unwrap();
 for definition in definitions(program){let mut game=GameState::new(vec!["A".into(),"B".into()],20);let own=game.create_object_from_definition(&body(),A,Zone::Battlefield);execute(&mut game,&definition,1);execute(&mut game,&definition,2);ironsmith::rules::state_based::apply_state_based_actions_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.battlefield.iter().filter(|id|game.object(**id).unwrap().subtypes.contains(&ironsmith::Subtype::Role)).count(),1);assert_eq!(game.calculated_toughness(own),Some(3));}
}
#[test]
fn role_choice_without_friendly_creature_never_attaches_to_an_opponent(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-audition-choices.json")).unwrap();let program=fixture["mechanics"][0]["executable_reminder"].as_str().unwrap();
 for definition in definitions(program){let mut game=GameState::new(vec!["A".into(),"B".into()],20);let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);execute(&mut game,&definition,1);ironsmith::rules::state_based::apply_state_based_actions_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.calculated_power(enemy),Some(7));assert!(!game.battlefield.iter().any(|id|game.object(*id).unwrap().subtypes.contains(&ironsmith::Subtype::Role)));}
}
