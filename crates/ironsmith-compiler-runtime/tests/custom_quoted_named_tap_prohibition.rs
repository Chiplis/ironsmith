//! Full public nested-token route, strict claims and exact typed label ownership.
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
#[test]
fn quoted_token_rules_retain_independent_block_and_exact_named_payment_prohibitions(){
 for label in ["cadence","cadence craft","stellar weaving"] {
  let text=format!("Type: Sorcery\nCreate a 1/1 black Elemental creature token with \"This creature can't block.\" and \"This creature can't be tapped to pay the generic tap payment named {label}.\"");
  let (artifact,direct)=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Quoted payment rule",&text,false)).unwrap();
  let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
  let loaded=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
  for definition in [direct,loaded] {
   let actor=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,actor,Zone::Stack);let mut ctx=EffectContext::new_default(source,actor);
   for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
   let token=*game.battlefield.first().unwrap();
   assert!(game.object_has_ability(token,&ironsmith::static_abilities::StaticAbility::cant_block()));
   let labels=game.current_abilities(token).unwrap().iter().filter_map(|ability|match &ability.kind {
    ironsmith::ability::AbilityKind::Static(rule)=>match rule.compiled_model().map(|model|&model.payload) {
     Some(ironsmith_core::StaticAbilityPayload::GenericManaTapPaymentProhibition{name})=>Some(name.clone()),_=>None,
    },_=>None,
   }).collect::<Vec<_>>();
   assert_eq!(labels,vec![label.to_string()]);
  }
 }
}
#[test]
fn unsupported_sibling_text_is_not_claimed_as_part_of_the_payment_name(){
 let text="Type: Sorcery\nCreate a 1/1 black Elemental creature token with \"This creature can't be tapped to pay the generic tap payment named cadence. Invent an unrelated action.\"";
 assert!(ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_runtime_definition("No blanket claims",text,false)).is_err());
}
