//! Complete family equations, including observers/modifiers and every linked face.
//! Preserved pending input is never silently skipped or counted as support.
#[path="custom_final_next_wave_support.rs"]mod source_support;
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-actions-status-whole-source-complete.json")).unwrap()}
fn canonical(definition:ironsmith::CardDefinition)->Result<serde_json::Value,String>{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("{error:?}"))?;serde_json::to_value(wire).map_err(|error|error.to_string())}
fn whole_family(family:&str,expected:usize){
 let f=fixture();let mut count=0;let mut seen=std::collections::BTreeSet::new();let mut errors=Vec::new();
 for row in f["mechanics"].as_array().unwrap().iter().filter(|row|row["equation_family"]==family){for card in row["representative_cards"].as_array().unwrap(){
  count+=1;let key=card["source_card_key"].as_str().unwrap();assert!(seen.insert(key.to_string()),"duplicate physical source {family}/{key}");
  if !card["executable_faces"].is_array(){errors.push(format!("{key}: whole physical source is not materialized; pending inputs are not support"));continue;}
  match source_support::linked_definitions(card){
   Err(error)=>errors.push(format!("{key}: {error}")),
   Ok(routes)=>{let originals=card["source_faces"].as_array().unwrap();if routes.len()!=originals.len(){errors.push(format!("{key}: complete sibling count changed"));continue;}
    for (index,[direct,artifact]) in routes.into_iter().enumerate(){
     if ironsmith::cards::generated_definition_has_unimplemented_content(&direct)||ironsmith::cards::generated_definition_has_unimplemented_content(&artifact){errors.push(format!("{key}/{}: unsupported body",originals[index]["name"]));}
     match(canonical(direct),canonical(artifact)){(Ok(a),Ok(b))=>if a!=b{errors.push(format!("{key}/{}: full typed native/artifact definitions differ",originals[index]["name"]));},(Err(e),_)|(_,Err(e))=>errors.push(format!("{key}: canonical comparison {e}"))}
    }
   }
  }
 }}
 assert_eq!(count,expected,"complete corpus equation for {family}");assert!(errors.is_empty(),"{}",errors.join("\n"));
}
#[test]fn all30audition_sources_and_modifiers(){whole_family("audition",30)}
#[test]fn all1autocrat_sources(){whole_family("autocrat",1)}
#[test]fn all32collect_backing_sources_observers_and_prohibitions(){whole_family("collect backing",32)}
#[test]fn all32judge_sources_actor_forms_and_consumers(){whole_family("judge",32)}
