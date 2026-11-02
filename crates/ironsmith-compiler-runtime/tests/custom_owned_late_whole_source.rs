//! Complete source equations, including consumers/grants and all linked faces.
//! A fixture's presence is not support; root must materialize all inputs first.
#[path="custom_final_next_wave_support.rs"] mod source_support;
fn check(scope:&str,keyword:&str,expected:usize){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/owned-late-whole-source.json")).unwrap();
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["scope"]==scope&&row["keyword"].as_str().unwrap().eq_ignore_ascii_case(keyword)).unwrap();
 let cards=row["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),expected);
 let mut keys=std::collections::BTreeSet::new();let mut errors=Vec::new();let mut faces=0;
 for card in cards {
  let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key));
  assert!(card["source_raw_record"].is_object(),"complete raw source record required: {key}");
  let original_faces=card["source_faces"].as_array().unwrap();
  assert_eq!(original_faces.len(),if card["source_raw_record"]["cardName2"].as_str().is_some_and(|name|!name.is_empty()){2}else{1});
  for (index,face) in original_faces.iter().enumerate(){
   let raw_field=if index==0 {"rulesText"}else{"rulesText2"};
   assert!(face["original_text"].as_str().unwrap().contains(card["source_raw_record"][raw_field].as_str().unwrap()),"raw face omitted: {key}/{index}");
  }
  if card["rewrite_status"]!="frontend_materialized_unverified" {
   errors.push(format!("{key}: complete source input remains unmaterialized ({})",card["rewrite_status"]));continue;
  }
  match source_support::linked_definitions(card){
   Err(error)=>errors.push(format!("{key}: {error}")),
   Ok(routes)=>{
    if routes.len()!=original_faces.len(){errors.push(format!("{key}: native linked-face count {} != {}",routes.len(),original_faces.len()));}
    for (index,[native,artifact]) in routes.into_iter().enumerate(){
     faces+=1;
     if ironsmith::cards::generated_definition_has_unimplemented_content(&native)||ironsmith::cards::generated_definition_has_unimplemented_content(&artifact){errors.push(format!("{key}/{index}: unimplemented content"));}
     // A predicate-only source uses the mechanic's declared identity as a
     // filter; mentioning that identity must not grant the mechanic to itself.
     if key=="Scaravi Skald_SVG"||key=="Stalker of Myth_TTR" {
      for definition in [&native,&artifact] {
       if definition.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(rule) if rule.custom_keyword_name().is_some_and(|name|name.eq_ignore_ascii_case(keyword)))) {
        errors.push(format!("{key}/{index}: predicate-only source incorrectly gained its referenced mechanic"));
       }
      }
     }
     let wire=|definition|ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("{error:?}"));
     match(wire(native),wire(artifact)){
      (Ok(native),Ok(artifact))=>if native!=artifact{errors.push(format!("{key}/{index}: complete native/artifact typed definition differs"));},
      (Err(error),_)|(_,Err(error))=>errors.push(format!("{key}/{index}: typed runtime encoding {error}")),
     }
    }
   }
  }
 }
 assert!(errors.is_empty(),"{scope}:{keyword}: {} complete-source failures across {expected} physical keys/{faces} admitted faces:\n{}",errors.len(),errors.join("\n"));
}
#[test] fn msem_journey_all_five(){check("msem","journey",5)}
#[test] fn rev_cybersoul_all_sixteen_linked_sources(){check("rev","cybersoul",16)}
#[test] fn rev_decoy_all_fourteen_declarations_and_grants(){check("rev","decoy",14)}
#[test] fn rev_glory_all_sixteen_including_graveyard_consumer(){check("rev","glory",16)}
#[test] fn rev_journey_all_thirty_including_status_consumers(){check("rev","journey",30)}
#[test] fn rev_rapture_all_sixteen_including_attack_grants(){check("rev","rapture",16)}
#[test] fn rev_time_loop_all_six_including_generated_saga(){check("rev","time loop",6)}
#[test] fn msem_chronicle_all_twenty_including_membership_consumer(){check("msem","chronicle",20)}
