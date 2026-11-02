//! Ordinary cost nouns in names cannot become aliases inside rules grammar.
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[test]
fn source_name_cost_nouns_preserve_typed_activation_payment_in_direct_and_artifact_paths() {
 let text="Type: Creature\nPower/Toughness: 2/3\n{2}{W}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana.";
 for name in ["Activation payment","Ability witness","Cost witness","Payment witness","Scoped helper"] {
  let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
  assert!(!loss.is_lossy(),"{name}: {}",loss.reasons_text());let (artifact,direct)=result.unwrap();
  assert!(format!("{:?}",direct.abilities).contains("creatures_only: true"),"{name}");
  let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
  let materialized=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
  assert!(format!("{:?}",materialized.abilities).contains("creatures_only: true"),"{name}");
 }
}
#[test]
fn reserved_short_alias_does_not_prevent_full_name_self_reference() {
 let text="Type: Creature\nPower/Toughness: 2/3\n{1}: Activation payment gets +1/+1 until end of turn.";
 let definition=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_runtime_definition("Activation payment",text,false)).unwrap();
 assert_eq!(definition.abilities.len(),1);
}
