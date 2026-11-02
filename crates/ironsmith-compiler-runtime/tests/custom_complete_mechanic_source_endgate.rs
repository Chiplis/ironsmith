//! Universal source completeness/strict compiler/artifact gate. Gameplay remains separately required.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use serde_json::Value;
use std::collections::{BTreeMap,BTreeSet};
use std::path::{Path,PathBuf};
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")}
fn json(path:&Path)->Result<Value,String>{let text=std::fs::read_to_string(path).map_err(|e|format!("{}: {e}",path.display()))?;serde_json::from_str(&text).map_err(|e|format!("{}: {e}",path.display()))}
fn manifest()->Result<Value,String>{let path=std::env::var_os("IRONSMITH_CUSTOM_SOURCE_ENDGATE_MANIFEST").map(PathBuf::from).unwrap_or_else(||root().join("fixtures/custom_mechanics/custom-source-endgate.json"));json(&path)}
fn string<'a>(v:&'a Value,k:&str)->Result<&'a str,String>{v[k].as_str().ok_or_else(||format!("missing string {k}"))}
fn array<'a>(v:&'a Value,k:&str)->Result<&'a Vec<Value>,String>{v[k].as_array().ok_or_else(||format!("missing array {k}"))}
fn corpus_record<'a>(corpus:&'a Value,key:&str)->Result<&'a Value,String>{array(corpus,"cards")?.iter().find(|c|c["key"].as_str()==Some(key)).ok_or_else(||format!("authoritative physical key missing: {key}"))}
fn immutable_fields(source:&Value,expected:&Value)->Result<(),String>{for(k,v)in expected.as_object().ok_or("immutable source fields must be an object")?{if source.get(k)!=Some(v){return Err(format!("immutable original field changed: {k}"));}}Ok(())}
fn normalized_key(scope:&str,key:&str)->String{let bare=key.strip_prefix("revolution:").or_else(||key.strip_prefix("rev:")).or_else(||key.strip_prefix("msem:")).unwrap_or(key);format!("{scope}:{bare}")}
fn locate(fixture:&Value,case:&Value)->Result<Value,String>{
 let row=array(fixture,"mechanics")?.iter().find(|r|r["id"]==case["row_id"]).ok_or_else(||format!("canonical row {} missing",case["row_id"]))?;
 let key=string(case,"physical_key")?;let scope=key.split(':').next().ok_or("physical key lacks scope")?;
 let cards=array(row,"representative_cards")?;
 let matches:Vec<_>=cards.iter().filter(|c|c["source_card_key"].as_str().or_else(||c["id"].as_str()).is_some_and(|k|normalized_key(scope,k)==key)).collect();
 let card=matches.iter().find(|c|c["id"]==case["record_id"]).copied().or_else(||(matches.len()==1).then(||matches[0])).ok_or_else(||format!("missing/ambiguous canonical physical record {key}"))?;
 let source=if let Some(field)=case["nested_field"].as_str(){card.get(field).ok_or_else(||format!("independent clause missing: {field}"))?}else{card};
 immutable_fields(source,&case["immutable_original_fields"])?;Ok(source.clone())
}
fn parity_deltas(left:&Value,right:&Value)->Vec<String>{
 fn walk(path:&str,a:&Value,b:&Value,out:&mut Vec<String>){
  if a==b||out.len()>=8{return;}
  match(a,b){
   (Value::Object(a),Value::Object(b))=>{let keys:BTreeSet<_>=a.keys().chain(b.keys()).collect();for k in keys{match(a.get(k),b.get(k)){(Some(a),Some(b))=>walk(&format!("{path}/{k}"),a,b,out),_=>out.push(format!("{path}/{k}: field presence differs"))}if out.len()>=8{break;}}},
   (Value::Array(a),Value::Array(b))=>{if a.len()!=b.len(){out.push(format!("{path}: length {} != {}",a.len(),b.len()));}for(i,(a,b))in a.iter().zip(b).enumerate(){walk(&format!("{path}/{i}"),a,b,out);if out.len()>=8{break;}}},
   _=>out.push(format!("{path}: native={} artifact={}",a.to_string().chars().take(240).collect::<String>(),b.to_string().chars().take(240).collect::<String>())),
  }
 }
 let mut result=Vec::new();walk("$",left,right,&mut result);result
}
fn compile_case(source:Value,expected_faces:usize)->Result<usize,String>{
 let routes=source_support::linked_definitions(&source)?;
 if routes.len()!=expected_faces{return Err(format!("complete physical face count changed: expected {expected_faces}, got {}",routes.len()));}
 let mut failures=Vec::new();
 for(index,[direct,artifact])in routes.into_iter().enumerate(){
  let left=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(direct).map_err(|e|format!("face {index} native typed encoding: {e:?}"));
  let right=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(artifact).map_err(|e|format!("face {index} artifact typed encoding: {e:?}"));
  match(left,right){
   (Ok(a),Ok(b))=>match(serde_json::to_value(a),serde_json::to_value(b)){
    (Ok(a),Ok(b))if a==b=>{},
    (Ok(a),Ok(b))=>failures.push(format!("face {index}: complete typed native/artifact definition differs: {}",parity_deltas(&a,&b).join("; "))),
    (Err(e),_)|(_,Err(e))=>failures.push(format!("face {index}: full typed serialization: {e}")),
   },
   (Err(e),_)|(_,Err(e))=>failures.push(e),
  }
 }
 if failures.is_empty(){Ok(expected_faces)}else{Err(failures.join("\n"))}
}
#[test]
fn all_keyword_and_formal_source_equations_are_explicit_before_whole_corpus_certification(){
 let m=manifest().expect("read universal source endgate manifest");let mut failures=Vec::new();
 let groups=array(&m,"groups").expect("source groups");
 let families=groups.iter().filter(|g|g["kind"]=="keyword_family").count();
 let categories=groups.iter().filter(|g|g["kind"]=="category").count();
 if families!=197{failures.push(format!("expected197 keyword groups, got {families}"));}
 if categories!=18{failures.push(format!("expected18 category groups, got {categories}"));}
 if array(&m,"formal_rule_links").map(Vec::len).unwrap_or(0)!=205{failures.push("expected205 formal named-rule mappings".into());}
 let cases=array(&m,"cases").expect("physical cases");let case_map:BTreeMap<_,_>=cases.iter().filter_map(|c|c["id"].as_str().map(|id|(id,c))).collect();
 if case_map.len()!=cases.len(){failures.push("duplicate source case IDs".into());}
 for group in groups{
  let gid=group["id"].as_str().unwrap_or("missing group ID");
  let unresolved=group["source_universe_status"].as_str().is_some_and(|s|s.starts_with("SEED")||s.starts_with("OLDER_BODY_LEXICAL")||s.contains("NOT_AUTOMATIC_OPERATIVE_COMPLETENESS")||s=="nonseed_body_inventory_requires_operative_classification");
  let has_in_scope=group["case_ids"].as_array().into_iter().flatten().filter_map(|id|id.as_str()).filter_map(|id|case_map.get(id)).any(|case|case["support_scope"]!="excluded_whole_card");
  if unresolved&&has_in_scope{failures.push(format!("{gid}: whole physical-source scope needs explicit proof ({}) owner={}",group["source_universe_status"],group["owner"]));}
  if unresolved&&!has_in_scope{eprintln!("SOURCE_SCOPE_OUTSIDE_EXECUTION {gid}: {} (preserved boundary/zero-source evidence; no engine support claim)",group["source_universe_status"]);}

  let mut represented=BTreeSet::new();
  for id in group["case_ids"].as_array().into_iter().flatten(){
   match id.as_str().and_then(|id|case_map.get(id)){
    Some(case)if case["support_scope"]!="independent_clause_not_whole_card"=>{if let Some(k)=case["physical_key"].as_str(){represented.insert(k);}},
    Some(_)=>{},None=>failures.push(format!("{gid}: case reference {id} missing")),
   }
  }
  for k in group["expected_physical_keys"].as_array().into_iter().flatten(){if let Some(k)=k.as_str(){if !represented.contains(k){failures.push(format!("{gid}: authoritative key {k} disappeared from physical matrix"));}}}
 }
 assert!(failures.is_empty(),"{} source-equation failures (separate from engine compilation):\n{}",failures.len(),failures.join("\n"));
}
#[test]
fn every_nonexcluded_physical_source_face_passes_strict_native_and_complete_artifact_parity(){
 let m=manifest().expect("read universal source endgate manifest");let mut failures=Vec::new();let mut corpora=BTreeMap::<String,Value>::new();let mut fixtures=BTreeMap::<String,Value>::new();
 let physical=m["physical_sources"].as_object().expect("authoritative physical matrix");let proofs=m["exclusion_proofs"].as_object().expect("immutable exclusion evidence");
 // Whole-corpus proofs become stale even when a changed/new operative source
 // is outside the previously selected matrix. Pin the complete frozen corpora.
 for(_,authority)in m["corpus_authority"].as_object().expect("frozen corpus authorities"){
  let result=(||->Result<(),String>{let f=string(authority,"path")?;let bytes=std::fs::read(root().join(f)).map_err(|e|e.to_string())?;if ironsmith_compiled_artifact::sha256_hex(&bytes)!=string(authority,"sha256")?{return Err(format!("complete corpus hash changed: {f}; regenerate exact source equations"));}Ok(())})();
  if let Err(e)=result{failures.push(e);}
 }
 // Validate every original full physical record, including excluded sources.
 for(k,p)in physical{
  let result=(||->Result<(),String>{let f=string(p,"source_file")?;if !corpora.contains_key(f){corpora.insert(f.into(),json(&root().join(f))?);}let actual=corpus_record(&corpora[f],string(p,"source_key")?)?;if actual!=&p["expected_record"]{return Err("complete immutable authoritative record changed".into());}Ok(())})();
  if let Err(e)=result{failures.push(format!("{k}: {e}"));}
 }
 let(mut compiled,mut excluded,mut clauses)=(0usize,0usize,0usize);
 for case in array(&m,"cases").expect("all physical cases"){
  let id=case["id"].as_str().unwrap_or("missing case ID");
  let result=(||->Result<(),String>{
   let key=string(case,"physical_key")?;if !physical.contains_key(key){return Err("source has no immutable authoritative record".into());}
   // Even an excluded body's selected fixture must preserve every captured
   // original; exclusion changes execution scope, never source fidelity.
   let selected=if case["source_record_only"].as_bool()==Some(true){None}else{
    let f=string(case,"fixture")?;if !fixtures.contains_key(f){fixtures.insert(f.into(),json(&root().join(f))?);}Some(locate(&fixtures[f],case)?)
   };
   if case["support_scope"]=="excluded_whole_card"{
    let refs=array(case,"exclusion_proofs")?;if refs.is_empty(){return Err("whole-source exclusion has no immutable reason evidence".into());}
    for r in refs{
     let proof=proofs.get(r.as_str().ok_or("invalid exclusion proof reference")?).ok_or("missing exclusion proof")?;
     if !proof["expected"].as_object().is_some_and(|v|!v.is_empty()){return Err("exclusion must retain a complete sourced reason/rule snapshot".into());}
     let path=root().join(string(proof,"path")?);
     // External frozen scope packages retain their full immutable snapshot in
     // this manifest. If the evidence file is present, reject any later drift.
     if path.exists(){let current=json(&path)?;if current.pointer(string(proof,"pointer")?)!=Some(&proof["expected"]){return Err(format!("immutable exclusion evidence changed: {}",path.display()));}}
    }
    // Excluded sources are retained in the exact physical matrix and never
    // masquerade as successful compilation or a successful independent clause.
    excluded+=1;return Ok(());
   }
   if case["source_record_only"].as_bool()==Some(true){return Err("authoritative source has no selected canonical executable fixture record".into());}
   let source=selected.ok_or("canonical executable source input missing")?;
   let n=case["expected_face_count"].as_u64().ok_or("missing exact physical face count")?as usize;
   // Isolate a parser panic's thread-local scoped vocabulary from later cases.
   // Native interpretation and artifact materialization remain independent.
   let joined=std::thread::spawn(move||compile_case(source,n)).join().map_err(|panic|panic.downcast_ref::<String>().cloned().or_else(||panic.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_else(||"native source compile panicked".into()))?;
   compiled+=joined?;if case["support_scope"]=="independent_clause_not_whole_card"{clauses+=1;}Ok(())
  })();
  if let Err(e)=result{failures.push(format!("{id}: {e}"));}
 }
 eprintln!("SOURCE_ENDGATE compiled_faces={compiled} excluded_whole_cases={excluded} independent_clause_cases={clauses} failures={}",failures.len());
 assert!(failures.is_empty(),"{} complete source failures (no unsupported fallback, field normalization or representative skipping):\n{}",failures.len(),failures.join("\n"));
}
