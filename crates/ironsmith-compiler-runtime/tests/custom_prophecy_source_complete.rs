//! Exact public-reveal source programs, strict reader ownership, and artifact transport.
use ironsmith_compiled_artifact::CompiledCardArtifact;
mod custom_final_next_wave_support;
#[test]
fn all_fifteen_actual_sources_preserve_their_complete_programs_on_independent_routes(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-prophecy-source-complete.json")).unwrap();
 let cards=fixture["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),15);
 let actual:std::collections::BTreeSet<_>=cards.iter().map(|s|s["source_card_key"].as_str().unwrap().split(':').next_back().unwrap().to_owned()).collect();
 let expected:std::collections::BTreeSet<_>=EXPECTED_PROPHECY.iter().map(|s|s.split(':').next_back().unwrap().to_owned()).collect();assert_eq!(actual,expected);
 let mut failures=Vec::new();
 for card in cards {
  if let Err(error)=custom_final_next_wave_support::definitions(card) {failures.push(format!("{}: {error}",card["source_card_key"]));}
 }
 assert!(failures.is_empty(),"{} full-source failures:\n{}",failures.len(),failures.join("\n"));
}
#[test]
fn opaque_reveal_labels_keep_program_and_persistent_price_independent(){
 let text="Mana cost: {4}\nType: Creature\nPower/Toughness: 2/2\nAs you reveal this card from a hidden zone, you may perform the moon augury action once each turn ending in exile as follows: [Exile this card].\nWhenever this card performs the moon augury action, if this card is in exile, for as long as it remains exiled, that player may cast that card by paying {1}{G} rather than paying its mana cost.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Opaque reveal offer",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();assert_eq!(direct.abilities.len(),2);assert!(direct.spell_effect.is_none());
 let serialized=String::from_utf8(artifact.to_json().unwrap()).unwrap();assert!(serialized.contains("moon augury"));assert!(!serialized.contains("prophecy"));
 let restored=CompiledCardArtifact::from_json(serialized.as_bytes()).unwrap();assert_eq!(artifact,restored);
}

const EXPECTED_PROPHECY:&[&str]=&["msem:Dylso Charmer_OPH", "msem:Foretold Strength_OPH", "msem:Glintwing Replica_OPH", "msem:Moonwing Watcher_OPH", "msem:Proclaimed Cure_OPH", "msem:Promised Seeker_OPH", "msem:Prophecized Plating_OPH", "msem:Reflected Moment_OPH", "msem:Reins of Influence_OPH", "msem:Reweave Reality_OPH", "msem:Sodden Sorrow_OPH", "msem:Stormwing Raven_OPH", "msem:Summoner's Call_OPH", "msem:Supreme Stampede_OPH", "msem:Fortunous Flames_OTH"];
