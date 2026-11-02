//! Prompt metadata exists before the independent native/raw artifact routes.
#[path="custom_final_next_wave_support.rs"] mod support;
use ironsmith::ability::AbilityKind;
fn labels(effect:&ironsmith::Effect,out:&mut Vec<Vec<String>>){
 if let Some(choice)=effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>(){
  assert!(!choice.faces_dilemma,"instruction alternatives remain resolution choices");
  out.push(choice.decision_labels.clone());
 }
 effect.visit_child_effects(&mut |child|labels(child,out));
}
fn verify(name:&str,text:&str,verbs:&[&str]){
 let source=serde_json::json!({"name":name,"executable_text":text,"original_text":text});
 let [direct,artifact]=support::definitions(&source).unwrap();
 let native=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(direct.clone()).unwrap();
 let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(artifact.clone()).unwrap();
 assert_eq!(serde_json::to_value(native).unwrap(),serde_json::to_value(wire).unwrap(),"entire typed definitions retain identical labels and executable fields");
 for definition in [direct,artifact]{
  let mut found=Vec::new();
  if let Some(program)=&definition.spell_effect{for effect in program.all_effects(){labels(effect,&mut found);}}
  for ability in &definition.abilities{match &ability.kind{
   AbilityKind::Triggered(ability)=>for effect in ability.effects.all_effects(){labels(effect,&mut found);},
   AbilityKind::Activated(ability)=>for effect in ability.effects.all_effects(){labels(effect,&mut found);},
   _=>{},
  }}
  assert!(!found.is_empty());
  for labels in found{assert_eq!(labels.len(),verbs.len());for (label,verb)in labels.iter().zip(verbs){assert!(!label.trim().is_empty());assert!(label.to_lowercase().contains(verb),"{labels:?}");}}
 }
}
#[test]fn actual_sinful_tempter_nested_unless_choice_preserves_structured_labels(){
 verify("Sinful Tempter","Mana cost: {3}{B}{R}\nType: Creature — Demon\nPower/Toughness: 5/4\nWhenever you attack, each opponent loses 3 life unless they discard a card or sacrifice a nonland permanent. Repeat this process once.\nYou may cast this spell from your hand for {B}{R} rather than pay its mana cost. If you cast this spell this way, sacrifice it at the beginning of the next end step.",&["discard","sacrifice"]);
}
#[test]fn unrelated_nested_inline_choices_have_the_same_native_and_raw_artifact_labels(){
 verify("Independent alternative witness","Type: Sorcery\nYou gain 4 life unless you pay 2 life or sacrifice an artifact.",&["pay","sacrifice"]);
}
