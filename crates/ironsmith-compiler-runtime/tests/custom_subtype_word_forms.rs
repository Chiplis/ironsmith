//! Exact source punctuation/Unicode/digit forms plus generic multiword admission.
use ironsmith_core::custom_subtypes::{CustomSubtype,CustomSubtypeDefinition};
use ironsmith_core::{Subtype,SubtypeFamily};
#[path="cf8_p08/support.rs"] mod support;

#[test]
fn all_six_punctuated_unicode_and_digit_source_forms_keep_exact_source_evidence() {
 let registry:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/source-subtypes.json")).unwrap();
 let controls:Vec<_>=registry["native_metadata_controls"].as_array().unwrap().iter().filter(|row|
  row["definition"]["singular"].as_str().unwrap().chars().any(|ch|!ch.is_ascii_alphabetic())).collect();
 assert_eq!(controls.len(),6);
 for row in controls {
  let definition:CustomSubtypeDefinition=serde_json::from_value(row["definition"].clone()).unwrap();
  let expected=Subtype::Custom(CustomSubtype::new(&definition.singular,definition.family).unwrap());
  let text=row["test_parse_input"].as_str().unwrap();
  let pair=ironsmith_compiler::with_custom_subtypes(&[definition],||support::definitions(row["name"].as_str().unwrap(),text)).unwrap();
  for card in pair {assert!(card.card.subtypes.contains(&expected),"{} {:?}",row["source_card_key"],card.card.subtypes);}
  assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Outside source scope",text,false).is_err());
 }
}
#[test]
fn multiword_names_use_longest_explicit_scope_match_and_preserve_neighboring_subtypes() {
 let vocabulary=[CustomSubtypeDefinition{singular:"Clockwork Sage".into(),plural:"Clockwork Sages".into(),family:SubtypeFamily::Creature}];
 ironsmith_compiler::with_custom_subtypes(&vocabulary,||{
  let expected=Subtype::Custom(CustomSubtype::new("Clockwork Sage",SubtypeFamily::Creature).unwrap());
  for text in ["Type: Creature — Clockwork Sage Wizard\nPower/Toughness: 1/1","Type: Creature — Clockwork Sages Wizard\nPower/Toughness: 1/1"] {
   for card in support::definitions("Scoped phrase",text) {assert!(card.card.subtypes.contains(&expected));assert!(card.card.subtypes.contains(&Subtype::Wizard));}
  }
  assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Unknown neighbor","Type: Creature — Clockwork Sage Undefinedneighbor\nPower/Toughness: 1/1",false).is_err());
 }).unwrap();
}
#[test]
fn multiword_planar_names_remain_one_collective_type_with_exact_family() {
 let vocabulary=[CustomSubtypeDefinition{singular:"Silver Horizon".into(),plural:"Silver Horizons".into(),family:SubtypeFamily::Planar}];
 ironsmith_compiler::with_custom_subtypes(&vocabulary,||{
  for card in support::definitions("Collective planar name","Type: Plane — Silver Horizon") {
   assert_eq!(card.card.subtypes.len(),1);assert!(card.card.subtypes[0].is_planar_subtype());assert!(!card.card.subtypes[0].is_creature_type());
  }
 }).unwrap();
}
#[test]
fn punctuation_alias_conflicts_fail_closed_without_widening_the_scope() {
 let vocabulary=[CustomSubtypeDefinition{singular:"Cloud-Kin".into(),plural:"Cloud-Kins".into(),family:SubtypeFamily::Creature},CustomSubtypeDefinition{singular:"Cloudkin".into(),plural:"Cloudkins".into(),family:SubtypeFamily::Creature}];
 assert!(ironsmith_compiler::with_custom_subtypes(&vocabulary,||()).is_err());
 assert!(ironsmith_core::custom_subtypes::lookup("Cloudkin").is_none());
 for invalid in ["-Cloud","Cloud-","Cloud;;Kin","123","Cloud/*Kin"] {assert!(CustomSubtype::new(invalid,SubtypeFamily::Creature).is_err());}
}
