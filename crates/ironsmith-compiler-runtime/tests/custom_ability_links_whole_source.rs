//! Whole physical corpus equations and complete independent source programs.
#[path="custom_final_next_wave_support.rs"] mod source;
#[path="custom_typed_source_parity_support.rs"] mod parity;
use serde_json::Value;
use std::collections::BTreeSet;
fn root()->std::path::PathBuf {std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")}
fn read(path:&str)->Value {serde_json::from_str(&std::fs::read_to_string(root().join(path)).unwrap_or_else(|error|panic!("{path}: {error}"))).unwrap()}
#[test]
fn all_eleven_families_have_exact_source_equations_and_no_available_input_skips() {
 let proof=read("fixtures/custom_mechanics/msem-ability-links-whole-source-scope.json");
 let corpus=read(proof["corpus"]["path"].as_str().unwrap());
 let cards=corpus["cards"].as_array().unwrap();
 assert_eq!(cards.len(),proof["corpus"]["physical_record_count"].as_u64().unwrap()as usize);
 let groups=proof["groups"].as_array().unwrap();assert_eq!(groups.len(),11);
 let cases=proof["cases"].as_array().unwrap();
 let mut failures=Vec::new();let mut completed=0usize;
 for group in groups {
  let keys:BTreeSet<_>=group["operative_keys"].as_array().unwrap().iter().map(|key|key.as_str().unwrap()).collect();
  assert_eq!(keys.len(),group["operative_count"].as_u64().unwrap()as usize);
  // Recompute the declared inflection equation from the complete corpus,
  // independently of the proof's case/locator list.
  let family=group["family"].as_str().unwrap();
  let stem=match family {"encourage"=>"encourag","pressure"=>"pressur","tithe"=>"tith",other=>other};
  let surfaces:Vec<_>=["","e","es","s","ed","ing","d"].iter().map(|suffix|format!("{stem}{suffix}")).collect();
  let scanned:BTreeSet<String>=cards.iter().filter_map(|raw|{
   let mut text=format!("{}\n{}",raw["rulesText"].as_str().unwrap_or(""),raw["rulesText2"].as_str().unwrap_or("")).to_lowercase();
   for field in ["fullName","cardName","cardName2","fullName2"] {
    if let Some(name)=raw[field].as_str().filter(|name|!name.is_empty()) {text=text.replace(&name.to_lowercase()," ");}
   }
   text.split(|ch:char|!ch.is_alphabetic()).any(|word|surfaces.iter().any(|surface|surface==word))
    .then(||format!("msem:{}",raw["key"].as_str().unwrap()))
  }).collect();
  assert_eq!(scanned,keys.iter().map(|key|key.to_string()).collect::<BTreeSet<_>>(),"whole corpus scope equation changed: {family}");
  let group_cases:Vec<_>=cases.iter().filter(|case|group["case_ids"].as_array().unwrap().contains(&case["id"])).collect();
  assert_eq!(group_cases.len(),keys.len());
  for case in group_cases {
   let key=case["source_card_key"].as_str().unwrap();
   let raw=cards.iter().find(|card|card["key"]==key).unwrap();assert_eq!(raw,&case["source_record"],"whole authoritative source changed: {key}");
   let locator=&case["locator"];let fixture=read(locator["path"].as_str().unwrap());
   let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==locator["row_id"]).unwrap();
   let card=row["representative_cards"].as_array().unwrap().iter().find(|card|card["id"]==locator["record_id"]).unwrap_or_else(||panic!("missing whole physical source {key}"));
   let faces=card["source_faces"].as_array().unwrap();
   let expected=case["expected_face_count"].as_u64().unwrap()as usize;assert_eq!(faces.len(),expected,"linked face lost: {key}");
   for (index,face) in faces.iter().enumerate() {
    let field=if index==0 {"rulesText"} else {"rulesText2"};let original=raw[field].as_str().unwrap_or("");
    assert!(face["original_text"].as_str().unwrap().contains(original.trim()),"original face body changed: {key}/{field}");
   }
   // Missing executable input is a strict failure, never an availability skip.
   match source::linked_definitions(card) {
    Err(error)=>failures.push(format!("{}/{key}: {error}",group["family"])),
    Ok(routes)=>{
     assert_eq!(routes.len(),expected,"complete linked transport changed: {key}");
     for [native,artifact] in routes {
      if let Err(error)=parity::compare(&native,&artifact){failures.push(format!("{}/{key}: {error}",group["family"]));}
     }
     completed+=1;
    }
   }
  }
 }
 assert!(failures.is_empty(),"strict whole-source failures (no omissions):\n{}",failures.join("\n"));
 assert_eq!(completed,cases.len());
}
