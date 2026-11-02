//! All frozen physical sources, including consumers/recipient grants and complete bodies.
#[path="custom_final_next_wave_support.rs"]mod source_support;
fn canonical(definition:ironsmith::CardDefinition)->Result<serde_json::Value,String>{let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("{error:?}"))?;serde_json::to_value(wire).map_err(|error|error.to_string())}
fn collect(fixture:&str,expected:&[&str]){let f:serde_json::Value=serde_json::from_str(fixture).unwrap();let mut seen=std::collections::BTreeSet::new();let mut errors=Vec::new();
 for row in f["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){let Some(key)=card["source_card_key"].as_str()else{continue;};if !expected.contains(&key){continue;}assert!(seen.insert(key.to_string()),"duplicate physical source {key}");
  if !card["executable_faces"].is_array(){errors.push(format!("{key}: complete source has not been materialized"));continue;}
  match source_support::linked_definitions(card){Err(error)=>errors.push(format!("{key}: {error}")),Ok(routes)=>{if routes.len()!=card["source_faces"].as_array().unwrap().len(){errors.push(format!("{key}: sibling face count changed"));continue;}for [direct,artifact]in routes{
   if ironsmith::cards::generated_definition_has_unimplemented_content(&direct)||ironsmith::cards::generated_definition_has_unimplemented_content(&artifact){errors.push(format!("{key}: unsupported body"));}
   match(canonical(direct),canonical(artifact)){(Ok(a),Ok(b))=>if a!=b{errors.push(format!("{key}: complete typed native/artifact definitions differ"));},(Err(e),_)|(_,Err(e))=>errors.push(format!("{key}: canonical comparison {e}"))}
  }}}
 }}assert_eq!(seen.len(),expected.len(),"the exact frozen corpus equation must be present");assert!(errors.is_empty(),"{}",errors.join("\n"));}

#[test]
fn all_17_salvage_whole_sources_use_independent_native_and_artifact_routes(){
 collect(include_str!("../../../fixtures/custom_mechanics/msem-source-collections-next.json"),&["Depthlight Captain_EIA", "Depthlight Frontliner_EIA", "Depthlight Salvager_EIA", "Dredgeline Colossus_EIA", "Frontline Descender_EIA", "Mettalia's Forward_EIA", "Mettalia, Light from Above_EIA", "Oceanographer_EIA", "Oil Driller_EIA", "Rappel Liner_EIA", "Roving Dreadnought_EIA", "Seabed Delver_EIA", "Silt Miner_EIA", "Specimen Collector_EIA", "Storm the Seafloor_EIA", "Treasure Haulers_EIA", "Vyzer, the Fallen Despot_EIA"]);
}

#[test]
fn all_19_torment_whole_sources_use_independent_native_and_artifact_routes(){
 collect(include_str!("../../../fixtures/custom_mechanics/msem-action-status-complete.json"),&["Agonizing Punishment_STN", "Dawnlit Battlefield_L", "Deadwood Grove_L", "Frozen Hinterlands_STN", "Grinning Torturer_OTH", "Hazy Coppice_STN", "Inferno Lash_STN", "King Florence of Caulder_MSC", "Lush Outskirts_L", "Mad Queen Margarete_MSL", "Shrouded Rooftops_L", "Spires of Caulte_STN", "Sunlit Fissure_STN", "Surge of Darkness_STN", "Tiger of the Drums_MSL", "Torch the City_STN", "Towering Boughs_STN", "Velinor, Storm of Shadows_SWR", "Weathered Fortress_L"]);
}
