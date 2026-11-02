//! Every optional text subset is a compiled program, independent of keyword names.
use ironsmith_core::ability_model::AbilityKind;
use ironsmith_core::StaticAbilityPayload;
fn compile(text:&str)->ironsmith_compiler::cards::CardDefinition {
 let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_core::CardId::new(),"Finite text witness");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(builder,text,ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());result.unwrap().definition
}
const MARKER:&str="For each time you pay this spell's optional additional cost, remove one bracketed text fragment from it.";
#[test]
fn every_independent_restriction_subset_retains_a_complete_structured_spell_program(){
 let text=format!("Mana cost: {{1}}{{U}}\nType: Instant\nAs you cast this spell, you may pay the following additional cost any number of times: {{1}}.\n{MARKER}\nDestroy target [artifact] creature [an opponent controls].");
 let definition=compile(&text);
 let plan=definition.abilities.iter().find_map(|ability|match &ability.kind{AbilityKind::Static(ability)=>match &ability.payload{StaticAbilityPayload::OptionalTextFragments(plan)=>Some(plan),_=>None},_=>None}).unwrap();
 assert_eq!(plan.labels.len(),2);assert_eq!(plan.variants.len(),4);assert_eq!(plan.casting_payment,Some(0));assert!(plan.validate().is_ok());
 let mut programs=std::collections::HashSet::new();for (mask,variant) in plan.variants.iter().enumerate(){assert_eq!(variant.removed,mask as u64);let program=format!("{:?}",variant.spell_effect);assert_eq!(program.contains("Artifact"),mask&1==0);assert_eq!(program.contains("Opponent"),mask&2==0);programs.insert(program);assert!(variant.abilities.iter().all(|ability|!matches!(&ability.kind,AbilityKind::Static(ability) if matches!(&ability.payload,StaticAbilityPayload::OptionalTextFragments(_)))));}
 assert_eq!(programs.len(),4,"each selector fact varies independently");
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Finite text witness",text,false).unwrap();
 let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(restored,artifact);
 for runtime in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
  let manager=runtime.abilities.iter().find_map(|ability|match &ability.kind {
   ironsmith::ability::AbilityKind::Static(ability) if ability.compiled_model().is_some_and(|model|matches!(&model.payload,ironsmith_core::StaticAbilityPayload::OptionalTextFragments(_)))=>Some(ability),_=>None,
  }).expect("typed fragment manager survives runtime materialization");
  assert_eq!(manager.id(),ironsmith_core::StaticAbilityId::OptionalTextFragments);
 }
}
#[test]
fn whole_sentence_removal_does_not_remove_the_surrounding_spell_program(){
 let text=format!("Type: Sorcery\nAs you cast this spell, you may pay the following additional cost any number of times: Pay 1 life.\n{MARKER}\nYou gain 1 life. [Draw a card.]");
 let definition=compile(&text);let plan=definition.abilities.iter().find_map(|ability|match &ability.kind{AbilityKind::Static(ability)=>match &ability.payload{StaticAbilityPayload::OptionalTextFragments(plan)=>Some(plan),_=>None},_=>None}).unwrap();
 assert_eq!(plan.variants[0].spell_effect.as_ref().unwrap().all_effects_owned().len(),2);assert_eq!(plan.variants[1].spell_effect.as_ref().unwrap().all_effects_owned().len(),1);
}
#[test]
fn persistent_fragment_manager_and_signed_loyalty_slots_are_invariant(){
 let definition=compile("Type: Legendary Planeswalker — Jace\nLoyalty: 4\n{1}: Remove one bracketed text fragment from this permanent.\n[+1]: You gain 1 life [and draw a card].");
 let plan=definition.abilities.iter().find_map(|ability|match &ability.kind{AbilityKind::Static(ability)=>match &ability.payload{StaticAbilityPayload::OptionalTextFragments(plan)=>Some(plan),_=>None},_=>None}).unwrap();
 assert_eq!(plan.labels.len(),1,"loyalty brackets are executable costs, not optional fragments");assert_eq!(plan.casting_payment,None);assert_eq!(plan.invariant_slots,vec![0]);assert!(plan.validate().is_ok());
 for variant in &plan.variants{assert_eq!(variant.abilities.len(),2);assert_eq!(format!("{:?}",variant.abilities[0]),format!("{:?}",definition.abilities[0]));}
}
#[test]
fn generic_repeatable_optional_cost_prefix_accepts_mana_discard_life_and_sacrifice(){
 for payment in ["{1}","Discard a card","Pay 3 life","Sacrifice a creature"]{
 let definition=compile(&format!("Type: Sorcery\nAs you cast this spell, you may pay the following additional cost any number of times: {payment}.\nYou gain 1 life."));
 assert_eq!(definition.optional_costs.len(),1);assert!(definition.optional_costs[0].repeatable);
 }
}
#[test]
fn incomplete_and_excessive_fragment_documents_are_rejected_before_enumeration(){
 let cost="As you cast this spell, you may pay the following additional cost any number of times: {1}.";
 for body in ["Draw [a card.","Draw a card]."]{
 let text=format!("Type: Sorcery\n{cost}\n{MARKER}\n{body}");let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_core::CardId::new(),"Malformed text");assert!(ironsmith_compiler::CompilerFacade::new().compile_definition(builder,text,ironsmith_compiler::CompilePolicy::default()).is_err());
 }
 let text=format!("Type: Sorcery\n{cost}\n{MARKER}\n{}",(0..13).map(|_|"[You gain 1 life.]").collect::<Vec<_>>().join(" "));let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_core::CardId::new(),"Resource limit");assert!(ironsmith_compiler::CompilerFacade::new().compile_definition(builder,text,ironsmith_compiler::CompilePolicy::default()).is_err());
}
