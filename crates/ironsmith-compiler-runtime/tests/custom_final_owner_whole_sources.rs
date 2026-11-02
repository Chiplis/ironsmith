//! Complete frozen-corpus ownership scope; availability is not gameplay certification.
#[path="custom_final_next_wave_support.rs"] mod source;
use ironsmith::CardDefinition;
use serde_json::Value;
fn wire(definition:CardDefinition)->Result<Value,String>{
 let definition=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error|format!("complete typed encoding: {error:?}"))?;
 serde_json::to_value(definition).map_err(|error|error.to_string())
}
#[test]
fn all_two_hundred_one_physical_sources_preserve_whole_faces_and_independent_routes(){
 let data:Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-owner-complete-inventory.json")).unwrap();
 let mut originals=std::collections::BTreeMap::new();let mut families=std::collections::BTreeMap::new();let mut failures=Vec::new();let mut cases=0usize;let mut face_cases=0usize;let mut routes=0usize;
 for row in data["mechanics"].as_array().unwrap(){
  let family=row["keyword"].as_str().unwrap().to_ascii_lowercase();
  for card in row["representative_cards"].as_array().unwrap(){
   cases+=1;*families.entry(family.clone()).or_insert(0usize)+=1;
   let key=card["source_card_key"].as_str().unwrap();let faces=card["source_faces"].as_array().unwrap();face_cases+=faces.len();
   if let Some(prior)=originals.insert(key.to_string(),faces.clone()){assert_eq!(prior,*faces,"{key}: family associations must preserve identical complete physical originals");}
   let compiled=match source::linked_definitions(card){Ok(compiled)=>compiled,Err(error)=>{failures.push(format!("{family}/{} / {key}: {error}",row["id"]));continue;}};
   if compiled.len()!=faces.len(){failures.push(format!("{key}: omitted linked sibling: {} != {}",compiled.len(),faces.len()));}
   for (original,[native,artifact]) in faces.iter().zip(compiled){
    let name=original["name"].as_str().unwrap();routes+=2;
    if native.card.name.as_str()!=name||artifact.card.name.as_str()!=name{failures.push(format!("{key}: exact linked face name/order differs for {name}"));}
    match(wire(native),wire(artifact)){
     (Ok(native),Ok(artifact))if native==artifact=>{},
     (Ok(_),Ok(_))=>failures.push(format!("{key}/{name}: independent routes differ in complete typed definition")),
     (native,artifact)=>failures.push(format!("{key}/{name}: full wire encoding failed native={:?},artifact={:?}",native.err(),artifact.err())),
    }
   }
  }
 }
 assert_eq!(cases,202);assert_eq!(face_cases,254);assert_eq!(originals.len(),201);assert_eq!(originals.values().map(Vec::len).sum::<usize>(),252);
 let expected:std::collections::BTreeMap<String,usize>=[("attain",27),("breach",15),("crumble",1),("deploy",18),("disgraced",18),("fleeting",20),("foreshadow",26),("lionize",18),("memory",1),("resilience",1),("resurface",1),("siphon",20),("surpass",24),("voracious",12)].into_iter().map(|(family,count)|(family.to_string(),count)).collect();
 assert_eq!(families,expected,"all declarations, grants, observers, modifiers and receipt consumers must remain in scope");
 assert!(failures.is_empty(),"Every strict full-source failure (none excluded or skipped):\n{}",failures.join("\n"));
 assert_eq!(routes,508,"both independent routes required for all254complete face cases, including multiple family associations");
}
