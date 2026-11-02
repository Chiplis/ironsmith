use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,EffectExecutor};
use ironsmith_core::{FrozenNumericParameter,CreationNumericProperty};
fn definitions()->[CardDefinition;2]{
 let text="Type: Sorcery\nCreate a token that's a copy of target creature except it's an artifact instead of its other types and it has \"At the beginning of combat on your turn, if you spent at least the mana value of this permanent as it was created mana to cast spells this turn, you may draw a card.\"";
 let compiled=ironsmith_compiler::CompilerFacade::new().compile_definition(
  ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"Creation parameter"),
  text.to_string(),ironsmith_compiler::CompilePolicy{allow_unsupported:false}).unwrap();
 let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.definition).unwrap();
 let (artifact,_)=ironsmith_compiler_runtime::compile_to_artifact("Creation parameter",text,false).unwrap();
 let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()]
}
fn thresholds(game:&GameState,id:ironsmith::ObjectId)->Vec<FrozenNumericParameter>{
 let mut result=vec![];
 for ability in game.object(id).unwrap().abilities.iter(){if let ironsmith::ability::AbilityKind::Static(ability)=&ability.kind{let _:Result<_,std::convert::Infallible>=ability.canonical_model().expect("creation grant retains its typed model").try_map(|v|Ok(v),|v|Ok(v),|v|Ok(v),|condition|{if let ironsmith::effect::Condition::PlayerSpentManaToCastSpellsThisTurnAtLeast{amount,..}=&condition{result.push(*amount);}Ok(condition)});}}
 result
}
#[test]
fn sampled_copy_exception_parameter_is_fixed_and_copied_without_resampling(){
 for definition in definitions(){let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,a,Zone::Stack);
 let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Original","Type: Creature\nMana cost: {4}\nPower/Toughness: 2/2",false).unwrap();let original=game.create_object_from_definition(&body,a,Zone::Battlefield);
 let mut ctx=EffectContext::new_default(source,a).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(original)]);
 for effect in definition.spell_effect.as_ref().unwrap().all_effects(){ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();}
 let first=*game.battlefield.iter().find(|id|**id!=original).unwrap();assert_eq!(thresholds(&game,first),vec![FrozenNumericParameter::Fixed(4)]);
 game.object_mut(first).unwrap().mana_cost=None;assert_eq!(thresholds(&game,first),vec![FrozenNumericParameter::Fixed(4)],"later mana-cost changes cannot alter a captured parameter");
 let effect=ironsmith::effects::CreateTokenCopyEffect::one(ironsmith::target::ChooseSpec::SpecificObject(first)).without_mana_cost();let result=effect.execute(&mut game,&mut EffectContext::new_default(source,a)).unwrap();let ironsmith::effect::OutcomeValue::Objects(ids)=result.value else{panic!("copy result")};assert_eq!(thresholds(&game,ids[0]),vec![FrozenNumericParameter::Fixed(4)],"copied fixed parameters are not sampled from the new zero mana value");
 }}
#[test]
fn fixed_parameter_preserves_numeric_wire_compatibility(){assert_eq!(serde_json::to_string(&FrozenNumericParameter::Fixed(4)).unwrap(),"4");assert_eq!(serde_json::from_str::<FrozenNumericParameter>("4").unwrap(),FrozenNumericParameter::Fixed(4));assert_eq!(FrozenNumericParameter::Creation(CreationNumericProperty::ManaValue).fixed(),None);}
