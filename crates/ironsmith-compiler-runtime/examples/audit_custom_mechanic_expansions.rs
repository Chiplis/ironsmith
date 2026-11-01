//! JSONL adapter for actual frontend compileDraftInput faces; structural audit only.
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
use ironsmith_compiler::card::LinkedFaceLayout;
use ironsmith_compiler::{CardDefinitionBuilder, CompilePolicy, CompilerFacade, ids::CardId};
use std::io::{BufRead, Write};
use serde_json::{Value,json};

#[derive(Debug)]
struct CompileCardInput {
    name: String,
    text: String,
    token: bool,
    custom_keyword_identities: Vec<String>,
    /// Custom keywords whose definition created this card's single generic
    /// optional or alternative payment ("a spell using <keyword>").
    custom_keyword_payments: Vec<String>,
    allow_unsupported: bool,
    enforce_filter_claims: bool,
    enforce_line_claims: bool,
    local_id: u32,
    other_face_id: Option<u32>,
    other_face_name: Option<String>,
    linked_face_layout: Option<String>,
    semantic_score: Option<f32>,
}

fn input_from_value(v: Value) -> CompileCardInput {
    let strings = |key: &str| v[key].as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
    CompileCardInput {
        name: v["name"].as_str().unwrap_or("").into(),
        text: v["text"].as_str().unwrap_or("").into(),
        token: v["token"].as_bool().unwrap_or(false),
        custom_keyword_identities: strings("customKeywordIdentities"),
        custom_keyword_payments: strings("customKeywordPayments"),
        allow_unsupported: false, enforce_filter_claims: true, enforce_line_claims: true,
        local_id: v["localId"].as_u64().unwrap_or(1) as u32,
        other_face_id: v["otherFaceId"].as_u64().map(|n| n as u32),
        other_face_name: v["otherFaceName"].as_str().map(str::to_owned),
        linked_face_layout: v["linkedFaceLayout"].as_str().map(str::to_owned),
        semantic_score: None,
    }
}

fn compile_artifact(input: CompileCardInput) -> Result<CompiledCardArtifact, String> {
    let local_id = input.local_id.max(1);
    let mut builder = CardDefinitionBuilder::new(CardId::from_raw(local_id), input.name.clone());
    if input.token { builder = builder.token(); }
    for identity in &input.custom_keyword_identities {
        builder = builder.with_custom_keyword(identity.clone());
    }
    let mut compiled = ironsmith_compiler::word_claims::with_enforcement_options(
        input.enforce_filter_claims,
        input.enforce_line_claims,
        || {
            CompilerFacade::new().compile_definition(
                builder,
                input.text.clone(),
                CompilePolicy {
                    allow_unsupported: input.allow_unsupported,
                },
            )
        },
    )
    .map_err(|error| error.to_string())?;
    for keyword in &input.custom_keyword_payments {
        if !compiled.definition.attribute_custom_keyword_payment(keyword) {
            return Err(format!("custom keyword payment {keyword:?} has no unique optional or alternative payment to attribute"));
        }
    }
    compiled.definition.card.id = CardId::from_raw(local_id);
    compiled.definition.card.other_face = input.other_face_id.map(CardId::from_raw);
    compiled.definition.card.other_face_name = input.other_face_name.clone();
    compiled.definition.card.linked_face_layout = match input.linked_face_layout.as_deref() {
        Some("split") => LinkedFaceLayout::Split,
        Some("prepare") => LinkedFaceLayout::Prepare,
        Some("flip") => LinkedFaceLayout::Flip,
        Some("transform") | Some("transform_like") | Some("transformLike") => {
            LinkedFaceLayout::TransformLike
        }
        _ => LinkedFaceLayout::None,
    };
    // Scryfall `transform` is a transforming DFC (CR 712.1), not a modal one.
    compiled.definition.card.transforming_dfc = input.linked_face_layout.as_deref() == Some("transform");
    let wire_definition = wire_definition_from_serializable(&compiled.definition)
        .map_err(|error| format!("failed to encode compiled definition: {error}"))?;
    let wire_definition = ironsmith_text::with_choice_decision_labels(wire_definition, |wire|
        ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire)
            .map_err(|error| error.to_string()))?;
    let runtime_definition =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(
            wire_definition.clone(),
        )
        .map_err(|error| format!("failed to materialize compiled definition: {error}"))?;
    let canonical_text = ironsmith_text::compiled_text_lines(&runtime_definition).join("\n");
    let ability_labels = ironsmith_text::ability_surface_texts(&runtime_definition);

    let mut artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(local_id),
            name: compiled.definition.card.name.clone(),
            face_name: None,
            other_face: input.other_face_id.map(ArtifactCardId),
            linked_face_layout: Some(format!("{:?}", compiled.definition.card.linked_face_layout)),
        },
        CompiledCardPayload {
            definition: wire_definition,
            canonical_text,
            ability_labels,
        },
        concat!("ironsmith-compiler/", env!("CARGO_PKG_VERSION")),
        input.text.as_bytes(),
    );
    artifact.compiler_facts.insert(
        "allowUnsupported".to_string(),
        input.allow_unsupported.to_string(),
    );
    artifact.compiler_facts.insert(
        "enforceFilterClaims".into(),
        input.enforce_filter_claims.to_string(),
    );
    artifact.compiler_facts.insert(
        "enforceLineClaims".into(),
        input.enforce_line_claims.to_string(),
    );
    artifact.semantic_score = input.semantic_score.map(|score| score.clamp(0.0, 1.0));
    artifact.refresh_checksum();
    Ok(artifact)
}


fn unsupported(v:&Value)->bool { match v {Value::Object(m)=>m.iter().any(|(k,x)| ((k=="kind"||k=="id")&&x.as_str().is_some_and(|s|s.contains("Unimplemented")||s.contains("Unsupported")))||unsupported(x)),Value::Array(a)=>a.iter().any(unsupported),_=>false} }
fn audit(d:&Value)->Value {
 let faces=d["faces"].as_array().unwrap();let mut output=Vec::new();
 for (i,f) in faces.iter().enumerate(){
  let mut req=json!({"name":f["name"],"text":f["parse_input"],"customKeywordIdentities":f.get("customKeywordIdentities").cloned().unwrap_or(json!([])),"customKeywordPayments":f.get("customKeywordPayments").cloned().unwrap_or(json!([])),"allowUnsupported":false,"enforceFilterClaims":true,"enforceLineClaims":true,"localId":i+1,"token":d.get("token").and_then(Value::as_bool).unwrap_or(false)});
  if faces.len()==2 {req["otherFaceId"]=json!(if i==0{2}else{1});req["otherFaceName"]=faces[1-i]["name"].clone();req["linkedFaceLayout"]=if d["layout"]=="doubleface"{json!("transform_like")}else{d["layout"].clone()};}
  let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||ironsmith_compiler::parse_loss::capture(|| compile_artifact(input_from_value(req)))));
  output.push(match r {Ok((Ok(a), loss))=>{let bytes = a.to_json().unwrap(); let decoded = CompiledCardArtifact::from_json(&bytes).unwrap(); assert_eq!(a, decoded); ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(); let a=serde_json::to_value(a).unwrap();let def=&a["payload"]["definition"];json!({"name":f["name"],"status":"compiled","definition":def,"compiled_text":a["payload"]["canonicalText"],"has_unimplemented":unsupported(def),"parse_lossy":loss.is_lossy(),"loss_diagnostics":loss.reasons_text(),"artifact_roundtrip":true,"struct_findings":[],"artifact_checksum":a["payloadChecksum"],"compiler_facts":a["compilerFacts"],"token_requested":d.get("token").and_then(Value::as_bool).unwrap_or(false)})},Ok((Err(e), loss))=>json!({"name":f["name"],"status":"compile_failed","error":e,"parse_lossy":loss.is_lossy(),"loss_diagnostics":loss.reasons_text()}),Err(_)=>json!({"name":f["name"],"status":"compile_failed","error":"compiler panicked","fatal":true})});
 }
 let fatal=output.iter().any(|f|f["fatal"]==true);json!({"canCreate":output.iter().all(|f|f["status"]=="compiled"&&f["has_unimplemented"]==false&&f["parse_lossy"]==false),"faces":output,"fatal":fatal})
}
fn run() {
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for line in stdin.lock().lines() {
        let result = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => match serde_json::from_str::<Value>(&line) {
                Ok(input) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| audit(&input)))
                    .unwrap_or_else(|_| json!({"canCreate":false,"fatal":true,"error":"invalid request or audit panic"})),
                Err(error) => json!({"canCreate":false,"error":error.to_string()}),
            },
            Err(error) => json!({"canCreate":false,"error":error.to_string()}),
        };
        writeln!(out, "{}", result).unwrap();
        out.flush().unwrap();
        if result["fatal"] == true { break; }
    }
}
fn main(){std::thread::Builder::new().stack_size(64*1024*1024).spawn(run).unwrap().join().unwrap();}
