//! Complete fourteen-family physical input, independent routes and whole typed transport.
//! Parent materializes the preserved source input; every missing source stays a failure.
#[path = "custom_final_next_wave_support.rs"]
mod source;
use serde_json::Value;
fn wire(definition: ironsmith::CardDefinition) -> Result<Value,String> {
 let definition=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition)
  .map_err(|error|format!("complete runtime encoding: {error:?}"))?;
 serde_json::to_value(definition).map_err(|error|error.to_string())
}
#[test]
fn every_middle_family_preserves_complete_physical_faces_and_compiles_independent_routes(){
 let fixture:Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-middle-whole-source-input.json")).unwrap();
 let expected=[("Bloodprice",11),("Cadence",22),("Chosen one",13),("Defiance",8),("Desecrate",12),("Embody",11),("Enspirit",22),("Harvest",12),("Mark",15),("Puzzlebox",12),("Recite",19),("Sleight",2),("Spirited",40),("Upgrade",33)];
 let mut families=std::collections::BTreeMap::<String,std::collections::BTreeSet<String>>::new();
 let mut physical=std::collections::BTreeMap::<String,&Value>::new();
 for row in fixture["mechanics"].as_array().unwrap(){
  let family=row["keyword"].as_str().unwrap();
  for card in row["representative_cards"].as_array().unwrap(){
   let key=card["source_card_key"].as_str().unwrap().to_string();
   assert!(families.entry(family.into()).or_default().insert(key.clone()),"duplicate physical source inside one family");
   if let Some(previous)=physical.insert(key.clone(),card){assert_eq!(previous["source_faces"],card["source_faces"],"shared source originals differ: {key}");}
  }
 }
 assert_eq!(families.iter().map(|(f,keys)|(f.as_str(),keys.len())).collect::<Vec<_>>(),expected);
 assert_eq!(physical.len(),231);
 let mut failures=Vec::new();let mut face_count=0usize;let mut routes=0usize;
 for (key,card) in physical {
  let originals=card["source_faces"].as_array().expect("complete immutable physical faces");face_count+=originals.len();
  let definitions=match source::linked_definitions(card){Ok(value)=>value,Err(error)=>{failures.push(format!("{key}: {error}"));continue;}};
  if definitions.len()!=originals.len(){failures.push(format!("{key}: {} compiled faces != {} originals",definitions.len(),originals.len()));}
  for (original,[native,artifact]) in originals.iter().zip(definitions){
   let name=original["name"].as_str().unwrap();
   if native.card.name.as_str()!=name || artifact.card.name.as_str()!=name{failures.push(format!("{key}: original face name/order differs: {name}"));}
   routes+=2;
   match(wire(native),wire(artifact)){
    (Ok(left),Ok(right)) if left==right=>{},
    (Ok(_),Ok(_))=>failures.push(format!("{key} / {name}: complete native/artifact typed definitions differ")),
    (left,right)=>failures.push(format!("{key} / {name}: transport native={:?},artifact={:?}",left.err(),right.err())),
   }
  }
 }
 assert_eq!(face_count,245);
 assert!(failures.is_empty(),"All physical source failures (none skipped):\n{}",failures.join("\n"));
 assert_eq!(routes,490,"all 245 physical faces need actual independent native and artifact routes");
}
