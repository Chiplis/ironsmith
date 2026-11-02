//! Formal category scope does not blanket-exclude ordinary consumers.
#[path="custom_final_next_wave_support.rs"] mod source_support;
fn check(fixture:&serde_json::Value,keyword:&str,expected:usize){
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["keyword"]==keyword).unwrap();
 let cards=row["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),expected);
 let mut keys=std::collections::BTreeSet::new();let mut errors=Vec::new();
 for card in cards {
  let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key));
  if card["rewrite_status"]!="frontend_materialized_unverified"{errors.push(format!("{key}: full source unmaterialized: {}",card["rewrite_status"]));continue;}
  match source_support::linked_definitions(card){
   Err(error)=>errors.push(format!("{key}: {error}")),
   Ok(faces)=>{
    if faces.len()!=card["source_faces"].as_array().unwrap().len(){errors.push(format!("{key}: linked face omitted"));}
    for(index,[native,artifact])in faces.into_iter().enumerate(){
     if ironsmith::cards::generated_definition_has_unimplemented_content(&native)||ironsmith::cards::generated_definition_has_unimplemented_content(&artifact){errors.push(format!("{key}/{index}: unimplemented content"));}
     let wire=|definition|ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("{error:?}"));
     match(wire(native),wire(artifact)){
      (Ok(a),Ok(b))=>if a!=b{errors.push(format!("{key}/{index}: complete native/artifact typed definition differs"));},
      (Err(error),_)|(_,Err(error))=>errors.push(format!("{key}/{index}: encoding failed {error}")),
     }
    }
   }
  }
 }
 assert!(errors.is_empty(),"{keyword}: {} whole-source errors across {expected} physical keys:\n{}",errors.len(),errors.join("\n"));
}
#[test]
fn all_eleven_ordinary_quest_counter_and_type_consumers_are_not_intrinsic_actor_exclusions(){
 let fixture=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-quest-ordinary-consumers.json")).unwrap();
 check(&fixture,"Quest Cards",11);
}
#[test]
fn all_twenty_seven_transcend_sources_include_internal_consumers_and_all_linked_faces(){
 let fixture=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-category-bars.json")).unwrap();
 check(&fixture,"Transcend",27);
}
