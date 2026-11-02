//! Metadata admission controls backed by frozen source type lines/token rules.
//! Full source bodies continue through the materializer + audit adapter.
use ironsmith_core::custom_subtypes::{CustomSubtype,CustomSubtypeDefinition};
use ironsmith_core::{Subtype,SubtypeFamily};
#[path="cf8_p08/support.rs"] mod support;

#[test]
fn exact_source_type_metadata_compiles_with_scoped_vocabulary_and_survives_artifacts() {
 let registry:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/source-subtypes.json")).unwrap();
 for control in registry["native_metadata_controls"].as_array().unwrap() {
  let vocabulary:CustomSubtypeDefinition=serde_json::from_value(control["definition"].clone()).unwrap();
  let expected=Subtype::Custom(CustomSubtype::new(&vocabulary.singular,vocabulary.family).unwrap());
  let text=control["test_parse_input"].as_str().unwrap();let name=control["name"].as_str().unwrap();
  let definitions=ironsmith_compiler::with_custom_subtypes(&[vocabulary.clone()],||support::definitions(name,text)).unwrap();
  for definition in definitions {
   assert!(definition.card.subtypes.contains(&expected),"{}: {:?}",control["source_card_key"],definition.card.subtypes);
   assert!(expected.belongs_to_family(vocabulary.family));
   if vocabulary.family==SubtypeFamily::Planar {assert!(expected.is_planar_subtype());assert!(!expected.is_creature_type());}
   let mut game=ironsmith::GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,ironsmith::PlayerId(0),ironsmith::Zone::Battlefield);
   assert!(game.current_characteristics(source).unwrap().subtypes.contains(&expected));
  }
  assert!(ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).is_err(),"source-scoped vocabulary leaked: {}",vocabulary.singular);
 }
}

#[test]
fn plane_vocabulary_stays_distinct_from_creature_and_planeswalker_families() {
 let plane=CustomSubtypeDefinition{singular:"Sourceworld".into(),plural:"Sourceworlds".into(),family:SubtypeFamily::Planar};
 ironsmith_compiler::with_custom_subtypes(&[plane],||{
  let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Plane vocabulary","Type: Plane — Sourceworld",false).unwrap();
  let subtype=definition.card.subtypes[0];
  assert!(subtype.is_planar_subtype());assert!(!subtype.is_creature_type());assert!(!subtype.is_planeswalker_subtype());
 }).unwrap();
 assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("No scoped definition","Type: Plane — Sourceworld",false).is_err());
}
