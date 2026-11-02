//! Named source operands in level bands use the enclosing permanent identity.
mod custom_final_next_wave_support;
#[test]
fn level_band_static_prevention_binds_generic_source_names_without_guessing_other_names() {
 for name in ["Quiet Sentinel","Quiet Sentinel, Renewed","Reach"] {
  let source=serde_json::json!({"name":name,"executable_text":format!("Type: Creature\nPower/Toughness: 2/2\nLEVEL 1-2\n3/3\nPrevent the first 1 damage that would be dealt to {name} each turn.\nLEVEL 3+\n4/4\nPrevent all but the first 1 damage that would be dealt to {name} each turn.")});
  let definitions=custom_final_next_wave_support::definitions(&source).unwrap();
  assert_eq!(definitions.len(),2);
 }
 let other=serde_json::json!({"name":"Quiet Sentinel","executable_text":"Type: Creature\nPower/Toughness: 2/2\nLEVEL 1-2\n3/3\nPrevent the first 1 damage that would be dealt to Someone Else each turn."});
 assert!(custom_final_next_wave_support::definitions(&other).is_err(),"an unrelated proper name cannot become the source or a recipient-local budget by inference");
}
