//! Strict clause claims must survive triggered and activated parent programs.
#[test]
fn complete_counter_duration_grants_and_counter_menus_keep_structural_word_coverage(){
 for text in [
  "Type: Creature\nPower/Toughness: 2/2\n{B}, {T}: Choose target permanent. Put a bounty counter on that permanent. For as long as that permanent has a bounty counter on it, that permanent has \"When this permanent is put into a graveyard from the battlefield, each opponent draws a card.\"",
  "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature attacks, choose a kind of counter that it doesn't have on it from among flying and lifelink. Put a counter of that kind on it.",
  "Type: Creature\nPower/Toughness: 2/2\nWhenever another creature you control enters, for each creature you control, choose a kind of counter that that creature doesn't have on it from among flying and lifelink. Put a counter of that kind on that creature.",
 ]{
  let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Clause scope",text,false)));
  assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,_)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
 }
}
