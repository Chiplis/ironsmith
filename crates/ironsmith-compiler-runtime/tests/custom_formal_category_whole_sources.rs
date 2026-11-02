//! Whole actor/consumer source admission is independent of formal matched-name inventories.
#[path="custom_final_next_wave_support.rs"] mod source_support;
fn whole_sources(file:&str,keyword:Option<&str>,expected_keys:&[&str]){
 let expected=expected_keys.len();
 let path=format!("{}/../../fixtures/custom_mechanics/{file}",env!("CARGO_MANIFEST_DIR"));
 let fixture:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|error|panic!("missing preserved cohort {file}: {error}"))).unwrap();
 let mut keys=std::collections::BTreeSet::new();let mut records=0;let mut passed=0;let mut errors=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap(){
  if keyword.is_some_and(|keyword|row["keyword"].as_str()!=Some(keyword)){continue;}
  for card in row["representative_cards"].as_array().unwrap(){
   records+=1;let key=card["source_card_key"].as_str().or_else(||card["id"].as_str()).expect("exact physical key").trim_start_matches("msem:");
   assert!(keys.insert(key.to_owned()),"duplicate physical source {key} in {file}");let label=format!("{file} / {} / {key}",row["id"]);
   let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||source_support::linked_definitions(card)));
   match result {
    Ok(Ok(faces))=>{let original_faces=card["source_faces"].as_array().map_or(1,Vec::len);if faces.len()!=original_faces{errors.push(format!("{label}: expected {original_faces} original faces, got {}",faces.len()));}else{for pair in faces { assert_eq!(ironsmith_text::compiled_text_lines(&pair[0]),ironsmith_text::compiled_text_lines(&pair[1]),"{label}"); }passed+=1;}}
    Ok(Err(error))=>errors.push(format!("{label}: {error}")),Err(_)=>errors.push(format!("{label}: compiler panic (see source diagnostic)"))
   }
  }
 }
 assert_eq!(records,expected,"original actor+consumer records");assert_eq!(keys.len(),expected,"exact physical key union");
 assert_eq!(keys,expected_keys.iter().map(|key|key.to_string()).collect::<std::collections::BTreeSet<_>>(),"exact source inventory must match producer+consumer proof");
 assert!(errors.is_empty(),"{} strict whole-source failures ({passed}/{expected} passed):\n{}",errors.len(),errors.join("\n"));
}
#[test]fn all_twenty_one_oath_producers_observers_and_modifiers(){whole_sources("msem-oaths-source-complete.json",None,&["Bloodstained Crusade_TPW", "By Blood and Fire_TPW", "Enduring Scion_TPW", "Esalia's Witness_TPW", "Esmerie's Breeze-Seeker_TPW", "Favor of Petals_TPW", "Glory in Undeath_TPW", "Heedless Questant_TPW", "Hulking Fleshdebter_TPW", "Llech Crann Ambassador_TPW", "Llech Crann Knight_TPW", "Llech Crann Paladin_TPW", "Oath of the Wildlords_TPW", "Oathbound Scion_TPW", "Oathsworn Bloodsire_TPW", "Pactwhisper Demon_TPW", "Questant's Beginning_TPW", "Rhysta, Bladebound_TPW", "Rose Order Pactblade_TPW", "Rosewhisper Garden_TPW", "Yearning Heart_TPW"]);}
#[test]fn all_three_realm_sources_including_favor_abilities(){whole_sources("msem-realm-category.json",None,&["Hinode, Glorious Peak_MSC", "Lorilyf, Home-Tree_HZN", "Realms of the Goliaths_OTH"]);}
#[test]fn all_sixteen_module_actors_and_submodule_consumer(){whole_sources("msem-module.json",None,&["Chaos Module_BTB", "Creation Module_BTB", "Fury Module_BTB", "Identity Module_BTB", "Imagination Module_BTB", "Logic Module_BTB", "Morality Module_BTB", "Order Module_BTB", "Power Module_BTB", "Precognition Module_BTB", "Pyp, the Indexer_BTB", "Rebirth Module_BTB", "Replication Module_BTB", "Rhythm Module_BTB", "Root Module_BTB", "Shadow Module_BTB"]);}
#[test]fn all_twenty_one_diverge_sources_including_instant_timing_modifier(){whole_sources("msem-category-bars.json",Some("Diverge"),&["Achievement Aspirant_HQZ", "Amateur Inventor_HQZ", "Broadlane Well-to-Do_HQZ", "Child of Tezoculpa_HQZ", "Colonial Alchemist_HQZ", "Dashing Qith_HQZ", "Faithful of Abendo_HQZ", "Guardian of the Wards_HQZ", "Lovestruck Heir_HQZ", "Magde, Icatian Refugee_HQZ", "Maghbri Votary_HQZ", "Nephalia Preacher_HQZ", "Nohrish Soldier_HQZ", "Peerless Porcelain_HQZ", "Quarry of the Ivory Mask_HQZ", "Sisters of the Flock_HQZ", "Skyshroud Neophyte_HQZ", "Still-Smoke Courser_HQZ", "Twyllian Turncoat_HQZ", "Vine Street Brute_HQZ", "Whisper of Saebin_HQZ"]);}
#[test]fn all_three_forsake_sources_and_alternate_characteristic_programs(){whole_sources("msem-forsake.json",None,&["Cling to Old Dreams_HQZ", "Faces of War_HQZ", "Witch's Whisper_HQZ"]);}
