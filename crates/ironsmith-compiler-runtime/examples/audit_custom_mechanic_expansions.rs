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
    valid_ids:Result<(),String>,
    supplied_card_definitions:Vec<Value>,
    name: String,
    text: String,
    token: bool,
    custom_keyword_identities: Vec<String>,
    /// Custom keywords whose definition created this card's single generic
    /// optional or alternative payment ("a spell using <keyword>").
    custom_keyword_payments: Vec<String>,
    custom_subtypes: Result<Vec<ironsmith_compiler::CustomSubtypeDefinition>, String>,
    custom_keyword_counter_definitions: Vec<(String,String)>,
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
    let local_id=match v.get("localId").filter(|value|!value.is_null()) {
        Some(value)=>value.as_u64().and_then(|id|u32::try_from(id).ok()).ok_or_else(||"localId must fit an unsigned card definition ID".to_string()),
        None=>Ok(1),
    };
    let other_face_id=match v.get("otherFaceId").filter(|value|!value.is_null()) {
        Some(value)=>value.as_u64().and_then(|id|u32::try_from(id).ok()).map(Some).ok_or_else(||"otherFaceId must fit an unsigned card definition ID".to_string()),
        None=>Ok(None),
    };
    let valid_ids=local_id.as_ref().err().or_else(||other_face_id.as_ref().err()).map_or(Ok(()),|error|Err(error.clone()));
    CompileCardInput {
        valid_ids,
        supplied_card_definitions:v.get("suppliedCardDefinitions").or_else(||v.get("supplied_card_definitions")).and_then(Value::as_array).cloned().unwrap_or_default(),        name: v["name"].as_str().unwrap_or("").into(),
        text: v["text"].as_str().unwrap_or("").into(),
        token: v["token"].as_bool().unwrap_or(false),
        custom_keyword_identities: strings("customKeywordIdentities"),
        custom_keyword_payments: strings("customKeywordPayments"),
        custom_subtypes: serde_json::from_value(v.get("customSubtypes").cloned().unwrap_or(json!([])))
            .map_err(|error| format!("invalid custom subtype metadata: {error}")),
        custom_keyword_counter_definitions: v["customKeywordCounterDefinitions"].as_array().map(|definitions|definitions.iter().filter_map(|definition|Some((definition["name"].as_str()?.to_string(),definition["reminder"].as_str()?.to_string()))).collect()).unwrap_or_default(),
        allow_unsupported: false, enforce_filter_claims: true, enforce_line_claims: true,
        local_id: local_id.unwrap_or(0),
        other_face_id: other_face_id.unwrap_or(None),
        other_face_name: v["otherFaceName"].as_str().map(str::to_owned),
        linked_face_layout: v["linkedFaceLayout"].as_str().map(str::to_owned),
        semantic_score: None,
    }
}

fn compile_artifact(input: CompileCardInput) -> Result<CompiledCardArtifact, String> {
    input.valid_ids.clone()?;
    let reserved = input.local_id.max(1).max(input.other_face_id.unwrap_or(0));
    CardId::reserve_generated_namespace_through(CardId::from_raw(reserved)).map_err(str::to_owned)?;
    let vocabulary = input.custom_subtypes.clone()?;
    let sources = input.supplied_card_definitions.iter().map(|source| {
        ironsmith_compiled_artifact::validate_standalone_supplied_definition_metadata(source["text"].as_str().unwrap_or(""),source.as_object().ok_or("supplied definition must be an object")?)?;
        let strings=|key:&str|source.get(key).or_else(||source.get(match key {"customKeywordIdentities"=>"custom_keyword_identities","customKeywordPayments"=>"custom_keyword_payments",_=>key})).and_then(Value::as_array).map(|array|array.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default();
        Ok(ironsmith_compiler::SuppliedCardDefinitionSource {
            name: source["name"].as_str().unwrap_or("").into(), text: source["text"].as_str().unwrap_or("").into(),
            identities: strings("customKeywordIdentities"), payments: strings("customKeywordPayments"),
            subtypes: serde_json::from_value(source.get("customSubtypes").or_else(||source.get("custom_subtypes")).cloned().unwrap_or(json!([]))).map_err(|error|error.to_string())?,
        })
    }).collect::<Result<Vec<_>,String>>()?;
    ironsmith_compiler::with_custom_subtypes(&vocabulary, || {
        if sources.is_empty() {
            let definition = compile_definition_in_scope(&input)?;
            return encode_artifact(input, definition, None);
        }
        let bundle = ironsmith_compiler::compile_supplied_card_definition_bundle(
            CardId::from_raw(input.local_id.max(1)), &input.name, &input.text, !input.token, &sources,
            || compile_definition_in_scope(&input),
        )?;
        encode_artifact(input, bundle.root, Some((bundle.scope, bundle.catalog)))
    })?
}

fn compile_definition_in_scope(input: &CompileCardInput) -> Result<ironsmith_compiler::cards::CardDefinition, String> {
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
    for (name,reminder) in &input.custom_keyword_counter_definitions {
        let key=ironsmith_core::counter::normalize_named_counter_name(name)?;
        if key.is_empty() { return Err("empty ability-counter name".into()); }
        let program=ironsmith_compiler::word_claims::with_enforcement_options(input.enforce_filter_claims,input.enforce_line_claims,|| {
            CompilerFacade::new().compile_definition(CardDefinitionBuilder::new(CardId::new(),"Ability counter program").with_custom_keyword(key.clone()),format!("Type: Creature\nPower/Toughness: 1/1\n{reminder}"),CompilePolicy{allow_unsupported:false})
        }).map_err(|error|format!("ability counter {name}: {error}"))?;
        if program.definition.abilities.iter().all(|ability|matches!(&ability.kind,ironsmith_compiler::ability::AbilityKind::Static(value) if value.custom_keyword_name().is_some())) || program.definition.spell_effect.is_some(){return Err(format!("ability counter {name} must define permanent abilities"));}
        for subtype in &program.definition.card.custom_subtypes {
            ironsmith_core::custom_subtypes::merge_subtype_associations(&mut compiled.definition.card.custom_subtypes, *subtype);
        }
        if let Some(previous)=compiled.definition.ability_counter_definitions.insert(key,program.definition.abilities.clone()) {
            if serde_json::to_value(&previous).map_err(|error|error.to_string())? != serde_json::to_value(&program.definition.abilities).map_err(|error|error.to_string())? {return Err(format!("conflicting ability-counter definition {name}"));}
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
    Ok(compiled.definition)
}

fn encode_artifact(input: CompileCardInput, definition: ironsmith_compiler::cards::CardDefinition,
    graph: Option<(CardId, Vec<ironsmith_compiler::cards::CardDefinition>)>,
) -> Result<CompiledCardArtifact, String> {
    let local_id = input.local_id.max(1);
    let wire_definition = wire_definition_from_serializable(&definition)
        .map_err(|error| format!("failed to encode compiled definition: {error}"))?;
    let wire_definition = if let Some((scope, catalog)) = graph {
        let catalog = catalog.iter().map(wire_definition_from_serializable)
            .collect::<Result<Vec<_>, _>>().map_err(|error|error.to_string())?;
        ironsmith_runtime_catalog::artifact_materializer::bind_card_creation_scope(wire_definition, scope, &catalog)?
    } else { wire_definition };
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
            name: definition.card.name.clone(),
            face_name: None,
            other_face: input.other_face_id.map(ArtifactCardId),
            linked_face_layout: Some(format!("{:?}", definition.card.linked_face_layout)),
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
  let mut req=json!({"name":f["name"],"text":f["parse_input"],"customKeywordIdentities":f.get("customKeywordIdentities").cloned().unwrap_or(json!([])),"customKeywordPayments":f.get("customKeywordPayments").cloned().unwrap_or(json!([])),"customSubtypes":f.get("customSubtypes").or_else(||d.get("customSubtypes")).cloned().unwrap_or(json!([])),"customKeywordCounterDefinitions":f.get("customKeywordCounterDefinitions").or_else(||d.get("customKeywordCounterDefinitions")).cloned().unwrap_or(json!([])),"allowUnsupported":false,"enforceFilterClaims":true,"enforceLineClaims":true,"localId":i+1,"token":d.get("token").and_then(Value::as_bool).unwrap_or(false)});
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

#[cfg(test)]
mod explicit_id_admission_tests {
    use super::*;
    #[test]
    fn audit_ids_are_checked_without_truncating_to_another_identity() {
        for field in ["localId","otherFaceId"] {
            let mut input=json!({"name":"Root","text":"Type: Sorcery\nDraw a card."});
            input[field]=json!(u64::MAX);
            let decoded=input_from_value(input);
            assert!(decoded.valid_ids.is_err());
            assert!(compile_artifact(decoded).unwrap_err().contains(field));
        }
    }
}
