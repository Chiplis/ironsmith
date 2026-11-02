//! External invocation labels retain normal triggered programs and artifact identity.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
fn definitions(text:&str)->[CardDefinition;2]{
 let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("External class witness",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);let indirect=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect));[direct,indirect]
}
#[test]
fn externally_invoked_classes_preserve_each_body_without_an_autonomous_condition(){
 for delimiter in ["—",","]{for definition in definitions(&format!("Type: Artifact\nExternally triggered ability named resonance {delimiter} You gain 2 life.\nExternally triggered ability named choral tone {delimiter} Draw a card.")){
  assert_eq!(definition.abilities.len(),2);
  let mut classes=Vec::new();
  for ability in &definition.abilities{let ironsmith::AbilityKind::Triggered(triggered)=&ability.kind else{panic!("class is still a triggered ability")};let marker=triggered.trigger.downcast_ref::<ironsmith::triggers::special::CustomTrigger>().unwrap();classes.push(marker.id.to_string());}
  assert_eq!(classes,vec!["externally-triggered:resonance","externally-triggered:choral tone"]);
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&definition,PlayerId(0),Zone::Battlefield);
  let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::LifeGainEvent::new(PlayerId(0),1),Default::default());assert!(ironsmith::triggers::check_triggers(&game,&event).is_empty(),"external classes do not acquire an invented autonomous trigger");
 }}
}
#[test]
fn empty_or_unreadable_external_programs_fail_instead_of_losing_the_body(){
 for text in ["Type: Artifact\nExternally triggered ability named resonance —","Type: Artifact\nExternally triggered ability named resonance — Dance the unsupported dance."]{
  let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Unknown body",text,false)));assert!(result.is_err()||loss.is_lossy());
 }
}
