//! Complete preserved physical bodies; missing materialization is a failure, never a skipped source.
#[path="custom_final_next_wave_support.rs"] mod source_support;
fn collect(family:&str,expected:&[&str]){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-early-casting-whole-sources.json")).unwrap();
 let cards=fixture["mechanics"].as_array().unwrap().iter().filter(|row|row["family"]==family).flat_map(|row|row["representative_cards"].as_array().unwrap()).collect::<Vec<_>>();
 let actual=cards.iter().map(|card|card["id"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();
 let expected=expected.iter().copied().collect::<std::collections::BTreeSet<_>>();assert_eq!(actual,expected,"whole physical source equation for {family}");assert_eq!(cards.len(),actual.len(),"no duplicate physical source in one family");
 let mut failures=Vec::new();
 for card in cards {
  let key=card["id"].as_str().unwrap();
  match source_support::linked_definitions(card){
   Err(error)=>failures.push(format!("{key}: {error}")),
   Ok(pairs)=>{
    assert_eq!(pairs.len(),card["source_faces"].as_array().unwrap().len(),"every original linked face of {key} remains present");
    for [direct,artifact] in pairs {
     assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&direct),"native unsupported content: {key}");
     assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&artifact),"artifact unsupported content: {key}");
     assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&artifact),"independent native/artifact program surfaces differ: {key}");
    }
   }
  }
 }
 assert!(failures.is_empty(),"{} whole-source admission failures for {family}:\n{}",failures.len(),failures.join("\n"));
}
#[test]
fn all_bleed_physical_sources_preserve_their_whole_bodies_on_independent_routes(){
 collect("bleed",&[
  "msem:Absence of the Unmaker_TWR",
  "msem:Ambitious Dreams_TWR",
  "msem:Amongst the Flames_TWR",
  "msem:Cauldish Schemes_TWR",
  "msem:Commoners' Justice_TWR",
  "msem:Fall of Whiterun Harbor_TWR",
  "msem:Flash of Inspiration_TWR",
  "msem:Lady Genvieve of Caulder_TWR",
  "msem:Left for Carrion_TWR",
  "msem:Opportunistic Raiders_TWR",
  "msem:Painful Reminiscence_TWR",
  "msem:Restless Hedonist_TWR",
  "msem:Rokai's Gambit_OTH",
  "msem:Social Graces_TWR",
  "msem:Talks in Hidden Hollows_TWR",
  "msem:Tectonic Wound_TWR",
  "msem:Treacherous Informant_OTH",
  "msem:Undermurk Scavenger_OTH",
  "msem:Weaken Defenses_TWR",
  "msem:Withering Agony_TWR",
 ]);
}
#[test]
fn all_erratic_physical_sources_preserve_their_whole_bodies_on_independent_routes(){
 collect("erratic",&[
  "msem:Angelbane Spider_MIS",
  "msem:Branching Antlers_MIS",
  "msem:Chaosphere Ravager_MIS",
  "msem:Cosmostorm Rider_MIS",
  "msem:Dunehaze Revolutionary_OTH",
  "msem:Eternal Recurrence_MIS",
  "msem:Fearless Charger_MIS",
  "msem:Formed Anew_MIS",
  "msem:Ifram's Greenfur_MIS",
  "msem:Potential Beetle_MIS",
  "msem:Progenitor of Rebirth_MIS",
  "msem:Purgator of Storms_MIS",
  "msem:Splitflame Phoenix_MIS",
  "msem:Trifold Chaos_MIS",
  "msem:Unpredictable Bloodshed_MIS",
  "msem:Weaver of Ifram_MIS",
  "msem:Wild Gravetender_MIS",
  "msem:Zaidha, Seraph of Calamity_MIS",
 ]);
}
#[test]
fn all_infiltrate_physical_sources_preserve_their_whole_bodies_on_independent_routes(){
 collect("infiltrate",&[
  "msem:Captain of the Exiled_TWR",
  "msem:Cauldish Hookblade_TWR",
  "msem:Exile's Vanguard_TWR",
  "msem:Exiled Warband_TWR",
  "msem:Ferris's Crewmate_TWR",
  "msem:Hand of the Exiled_TWR",
  "msem:Helene, Steel of the Exiled_TWR",
  "msem:Merciless Exiled_TWR",
  "msem:Poacher's Bounty_OTH",
  "msem:Rapscallion Duo_TWR",
 ]);
}
#[test]
fn all_mirage_physical_sources_preserve_their_whole_bodies_on_independent_routes(){
 collect("mirage",&[
  "msem:Antithesis_POA",
  "msem:Arcane Figment_HQZ",
  "msem:Bath in the Solitude_POA",
  "msem:Blurry Counterpart_POA",
  "msem:Choking Despair_KLC",
  "msem:Crippling Spirit_POA",
  "msem:Dawn Silhouette_POA",
  "msem:Dunehaze Revolutionary_OTH",
  "msem:Ephemeral Support_KLC",
  "msem:Evanescier Greypelt_KLC",
  "msem:Evanescier Priest_KLC",
  "msem:Haze Raptor_POA",
  "msem:Hollow Hooligan_KLC",
  "msem:Imaginary Flames_POA",
  "msem:Lost in Fugue_KLC",
  "msem:Mesmerizing Shimmer_POA",
  "msem:Mongrel of Grenoui Hill_KLC",
  "msem:Moonshadow Remnant_KLC",
  "msem:Mystic's Mantle_KLC",
  "msem:Paralyzing Ennui_KLC",
  "msem:Precursors' Disdain_KLC",
  "msem:Preserving Cloak_POA",
  "msem:Pyramid Lord_POA",
  "msem:Sandborn Mage_POA",
  "msem:Sandmouth Dragon_POA",
  "msem:Sandstorm Sentinel_OTH",
  "msem:Through the Dust_POA",
  "msem:Torches Raised_KLC",
  "msem:Walking with Ancestors_HQZ",
  "msem:Wall of Fog_POA",
  "msem:Whistled Dragons_POA",
 ]);
}
#[test]
fn all_motivate_physical_sources_preserve_their_whole_bodies_on_independent_routes(){
 collect("motivate",&[
  "msem:Althestran Oathsworn_TOW",
  "msem:Celestian Crusader_TOW",
  "msem:Frontline Commander_TOW",
  "msem:General Merthilios_TOW",
  "msem:Goriana Battlechief_TOW",
  "msem:Goriana Naturalist_TOW",
  "msem:Goriana Tactician_TOW",
  "msem:Imposing Bulkvine_TOW",
  "msem:Loxodon Battlepriest_TOW",
  "msem:Merthilios' Escort_TOW",
  "msem:Nroxi Beastmaster_TOW",
  "msem:Rampaging Tremorwurm_TOW",
  "msem:Valtrian Cavalier_TOW",
  "msem:Warchief of the Goriana_OTH",
 ]);
}
#[test]
fn all_reprise_physical_sources_preserve_their_whole_bodies_on_independent_routes(){
 collect("reprise",&[
  "msem:A Villain Revealed_OTH",
  "msem:Arcanist's Dispersal_AFM",
  "msem:Aspect of An'yama_OTH",
  "msem:Bellow of the Highlands_OTH",
  "msem:Beragnian Blizzard_AFM",
  "msem:Biochemical Bloom_HQZ",
  "msem:Brutal Blow_HZN",
  "msem:Echoing Knowledge_AFM",
  "msem:Encore of the Drowned_OTH",
  "msem:Final Revelation_AFM",
  "msem:Hatsuo's Maneuver_OTH",
  "msem:Hisashi's Repose_OTH",
  "msem:Imminent Collision_AFM",
  "msem:Lafterii Ingenuity_AFM",
  "msem:Merkurio's Insight_AFM",
  "msem:Merkurio's Rebuttal_AFM",
  "msem:Meteor Volley_AFM",
  "msem:One Person's Trash_OTH",
  "msem:Roggar's Gambit_AFM",
  "msem:Savrios' Timeshifter_AFM",
  "msem:Secrets of Coralight Court_OTH",
  "msem:Shape the Tides_AFM",
  "msem:Surge of Depravity_AFM",
 ]);
}
