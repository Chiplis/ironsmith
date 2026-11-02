//! Exact Retinue universe; fresh strict native and artifact source routes.
mod custom_final_next_wave_support;
#[test]
fn all_thirty_two_retinue_sources_are_exact_complete_and_strict_on_independent_routes(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-retinue-whole-source-proof.json")).unwrap();
 let expected:&[&str]=&["Akvani Warbrute_G5L","Andazir, Destined Prince_G5L","Bearer of Fallen Tidings_G5L","Blossoming Elemental_G5L","Broken Wing Paragon_G5L","Child of Maaru_G5L","Choir of Steadfast Hearts_G5L","Choir of Trembling Hearts_G5L","Cloaked Instigator_G5L","Devourer of Devotees_G5L","Div of Broken Teeth_G5L","Djinn of the Measured Road_G5L","Enduring Sandshaper_G5L","Garmrud Skyscout_G5L","General of Phantasms_G5L","Harborline Fishermen_G5L","Heist Captain_G5L","Host of the Honored_G5L","Legion of Infinite Maws_G5L","Nizev Conspirators_G5L","Phantomsteed Cavalier_G5L","Rally the Fallen_G5L","Regiment on Wings_G5L","Sahra, Hand of Ruin_G5L","Satrap of Empty Halls_G5L","Sepidgar Conscript_G5L","Shepherd of the Pride_G5L","Stories in Shifting Sand_G5L","Subdue Undeath_G5L","Tombbound Creeper_G5L","Voice of Caravans_G5L","Withered Marshal_G5L"];let expected=expected.iter().map(|s|s.to_string()).collect::<std::collections::BTreeSet<_>>();let mut observed=std::collections::BTreeSet::new();let mut failures=Vec::new();let mut checked=0;
 for row in fixture["mechanics"].as_array().unwrap(){for source in row["representative_cards"].as_array().unwrap(){
  let key=source["source_card_key"].as_str().unwrap();assert!(observed.insert(key.to_string()),"duplicate:{key}");checked+=1;
  let originals=source["source_faces"].as_array().unwrap();assert_eq!(originals.len(),1,"retinue rawcorpus physical sourcefaces:{key}");
  match custom_final_next_wave_support::linked_definitions(source){
   Err(error)=>failures.push(format!("row={}/card={key}/face={}: {error}",row["id"],originals[0]["name"])),
   Ok(routes)=>{assert_eq!(routes.len(),originals.len(),"{key}: sourcebody lost");let pair=&routes[0];assert_eq!(pair[0].card.name.as_str(),originals[0]["name"].as_str().unwrap());assert_eq!(pair[1].card.name,pair[0].card.name);
    let encode=|d:ironsmith::CardDefinition|ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(d).map_err(|error|format!("{error:?}")).and_then(|value|serde_json::to_value(value).map_err(|error|error.to_string()));
    match(encode(pair[0].clone()),encode(pair[1].clone())){(Ok(native),Ok(artifact))=>if native!=artifact{failures.push(format!("{key}: fulltypedindependentroutes differ"));},(a,b)=>failures.push(format!("{key}: encodings {a:?} {b:?}"))}
   }
  }
 }}
 assert_eq!(observed,expected,"28declarations plus4 nondeclaration family consumers/modifiers");assert_eq!(checked,32);assert!(failures.is_empty(),"allstrictRetinuefailures:\n{}",failures.join("\n"));
}
