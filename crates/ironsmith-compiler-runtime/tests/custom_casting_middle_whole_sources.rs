//! Complete immutable casting-middle scope; pending materialization fails closed.
//! No clause-only compilation, source skips, unsupported allowances, or seed-count proof.
#[path = "custom_final_next_wave_support.rs"] mod source_support;
use std::collections::BTreeSet;
fn fixtures()->(serde_json::Value,serde_json::Value){
 (serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-middle-whole-source-inputs.json")).unwrap(),
  serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-middle-whole-source-scope.json")).unwrap())
}
fn collect(family:&str,expected:usize){
 let (fixture,proof)=fixtures();let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==format!("msem-whole-{family}")).unwrap();
 let equation=proof["families"].as_array().unwrap().iter().find(|row|row["family"]==family).unwrap();
 let cards=row["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),expected);assert_eq!(equation["operative_count"],expected);
 let expected_keys:BTreeSet<_>=equation["operative_keys"].as_array().unwrap().iter().map(|key|key.as_str().unwrap().to_owned()).collect();
 let actual_keys:BTreeSet<_>=cards.iter().map(|card|format!("msem:{}",card["source_card_key"].as_str().unwrap())).collect();
 assert_eq!(actual_keys.len(),expected,"duplicate physical sources");assert_eq!(actual_keys,expected_keys);
 for negative in equation["negative_records"].as_array().unwrap(){assert!(!actual_keys.contains(negative["key"].as_str().unwrap()),"name/flavor/ordinary-word control cannot become an operative source");}
 let mut failures=Vec::new();let mut compiled_faces=0;
 for card in cards{
  let key=card["source_card_key"].as_str().unwrap();let original=card["original_text"].as_str().unwrap();let raw=card["raw_source_record"]["rulesText"].as_str().unwrap();
  assert!(original.contains(raw),"{key}: authoritative whole body was changed");
  assert_eq!(card["source_faces"].as_array().unwrap().len(),1,"scope inventory has only single-faced operative physical cards; a future linked face must expand the authored equation");
  assert_eq!(card["source_faces"][0]["rulesText"],raw);
  if card["executable_text"].as_str().is_none() && card["executable_faces"].as_array().is_none(){failures.push(format!("{key}: whole-source frontend materialization pending"));continue;}
  match source_support::linked_definitions(card){
   Ok(faces)=>{assert_eq!(faces.len(),card["source_faces"].as_array().unwrap().len(),"{key}: dropped a printed face");for pair in faces{assert_eq!(pair.len(),2);for definition in pair{assert_eq!(definition.card.name,card["name"].as_str().unwrap(),"source identity must survive both independent routes");compiled_faces+=1;}}},
   Err(error)=>failures.push(format!("{key}: {error}")),
  }
 }
 assert!(failures.is_empty(),"{family}: {}/{} complete physical sources failed; no source skipped:\n{}",failures.len(),expected,failures.join("\n"));
 assert_eq!(compiled_faces,2*expected,"every whole source must compile through both independent routes");
}
#[test]
fn deception_whole_physical_sources(){collect("deception",19);}
#[test]
fn engage_whole_physical_sources(){collect("engage",11);}
#[test]
fn heretic_whole_physical_sources(){collect("heretic",13);}
#[test]
fn kindle_whole_physical_sources(){collect("kindle",13);}
#[test]
fn paranoia_whole_physical_sources(){collect("paranoia",12);}
#[test]
fn quickdraw_whole_physical_sources(){collect("quickdraw",11);}
#[test]
fn shimmer_whole_physical_sources(){collect("shimmer",16);}
#[test]
fn trial_whole_physical_sources(){collect("trial",22);}
#[test]
fn vision_whole_physical_sources(){collect("vision",13);}
#[test]
fn winnow_whole_physical_sources(){collect("winnow",25);}
