//! Complete physical source sets; no representative or clause substitution.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use std::{collections::{BTreeMap,BTreeSet},path::PathBuf};
use serde_json::Value;
fn root()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")}
fn read(path:&str)->Value{serde_json::from_str(&std::fs::read_to_string(root().join(path)).unwrap_or_else(|error|panic!("{path}: {error}"))).unwrap()}
fn verify(family:&str){
 let scope=read("fixtures/custom_mechanics/msem-actions-linked-whole-source-scope.json");
 let spec=scope["families"].as_array().unwrap().iter().find(|row|row["family"]==family).unwrap();
 let expected:BTreeSet<_>=spec["operative_keys"].as_array().unwrap().iter().map(|key|key.as_str().unwrap().to_owned()).collect();
 assert_eq!(expected.len(),spec["operative_physical_count"].as_u64().unwrap() as usize);
 let mut inputs=BTreeMap::new();
 for path in spec["canonical_fixture_paths"].as_array().unwrap(){
  let fixture=read(path.as_str().unwrap());
  for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
   let raw=card["source_card_key"].as_str().expect("full source key");
   let key=if raw.starts_with("msem:"){raw.to_owned()}else{format!("msem:{raw}")};
   if expected.contains(&key){assert!(inputs.insert(key.clone(),card.clone()).is_none(),"duplicate physical input {key}");}
  }}
 }
 assert_eq!(inputs.keys().cloned().collect::<BTreeSet<_>>(),expected,"whole-source set differs from audited corpus equation");
 let mut failures=Vec::new();let mut faces=0;
 for retained in spec["preserved_sources"].as_array().unwrap(){
  let key=retained["key"].as_str().unwrap();let card=&inputs[key];
  assert_eq!(card["source_shape"],"normal","audited linked-face shape changed: {key}");
  let raw_faces=retained["faces"].as_array().unwrap();let source_faces=card["source_faces"].as_array().expect("complete immutable faces");
  assert_eq!(source_faces.len(),raw_faces.len(),"{key}: a linked face was dropped");faces+=raw_faces.len();
  for (actual,original) in source_faces.iter().zip(raw_faces){assert_eq!(actual["name"],original["name"],"{key}");assert_eq!(actual["original_text"],original["original_text"],"{key}: source altered");}
  assert_eq!(card["original_text"],raw_faces[0]["original_text"],"{key}: whole body altered");
  if card["executable_text"].as_str().is_none(){failures.push(format!("{key}: complete input awaits frontend materialization ({})",card["rewrite_status"]));continue;}
  match source_support::definitions(card){
   Ok([native,artifact])=>{
    assert_eq!(native.card.name,artifact.card.name,"{key}: route name mismatch");
    assert_eq!(native.abilities.len(),artifact.abilities.len(),"{key}: route ability-count mismatch");
    for definition in [native,artifact]{if format!("{definition:?}").contains("Unimplemented"){failures.push(format!("{key}: unsupported runtime program"));}}
   },
   Err(error)=>failures.push(format!("{key}: {error}")),
  }
 }
 assert_eq!(faces,spec["operative_face_count"].as_u64().unwrap() as usize);
 assert!(failures.is_empty(),"{family} whole-source failures ({} of {}):\n{}",failures.len(),inputs.len(),failures.join("\n"));
}
#[test]fn complete_abound_sources_strict_independent_routes(){verify("abound");}
#[test]fn complete_catalyze_sources_strict_independent_routes(){verify("catalyze");}
#[test]fn complete_dynamize_sources_strict_independent_routes(){verify("dynamize");}
#[test]fn complete_rediscover_sources_strict_independent_routes(){verify("rediscover");}
#[test]fn complete_submerge_sources_strict_independent_routes(){verify("submerge");}
#[test]fn complete_voyage_sources_strict_independent_routes(){verify("voyage");}
