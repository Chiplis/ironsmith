//! Batch structural audit. Gameplay equivalence is deliberately a separate gate.
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::{CardDefinitionBuilder, ids::CardId};
use serde_json::{Value, json};
use std::{collections::HashSet, io::{self, Write}};

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("missing nonempty string: {key}"))
}

fn compile(name: &str, text: &str, keyword: &str) -> Value {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
                let builder = CardDefinitionBuilder::new(CardId::from_raw(1), name)
                    .with_custom_keyword(keyword);
                ironsmith_compiler_runtime::compile_builder_to_artifact(builder, text, false)
            })
        })
    }));
    match result {
        Err(_) => json!({"status":"panic", "structural_pass":false}),
        Ok((Err(error), loss)) => json!({"status":"rejected", "structural_pass":false,
            "error":error.to_string(), "loss_diagnostics":loss.reasons_text()}),
        Ok((Ok((artifact, definition)), loss)) => {
            let roundtrip = (|| -> Result<(), String> {
                let bytes = artifact.to_json().map_err(|e| e.to_string())?;
                let decoded = CompiledCardArtifact::from_json(&bytes).map_err(|e| e.to_string())?;
                if decoded != artifact { return Err("artifact changed on JSON roundtrip".into()); }
                let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded)
                    .map_err(|e| e.to_string())?;
                if ironsmith_text::compiled_text_lines(&restored) != ironsmith_text::compiled_text_lines(&definition) {
                    return Err("materialized rendered behavior changed".into());
                }
                Ok(())
            })();
            json!({"status":if loss.is_lossy() {"semantic_loss"} else if roundtrip.is_err() {"artifact_failure"} else {"compiled"},
                "structural_pass":!loss.is_lossy() && roundtrip.is_ok(),
                "loss_count":loss.count(), "loss_diagnostics":loss.reasons_text(),
                "artifact_roundtrip_error":roundtrip.err(),
                "rendered_text":ironsmith_text::compiled_text_lines(&definition)})
        }
    }
}

fn run() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("usage: audit_custom_mechanics FIXTURE.json")?;
    let input: Value = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if input["schema_version"] != 1 { return Err("unsupported schema_version".into()); }
    let mechanics = input["mechanics"].as_array().ok_or("mechanics must be an array")?;
    let mut ids = HashSet::new();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for mechanic in mechanics {
        let row = (|| -> Result<Value, String> {
            let id = required(mechanic, "id")?;
            if !ids.insert(id.to_owned()) { return Err(format!("duplicate mechanic id: {id}")); }
            let keyword = required(mechanic, "keyword")?;
            required(mechanic, "original_reminder")?;
            if !mechanic["executable_reminder"].is_null() { required(mechanic, "executable_reminder")?; }
            let cards = mechanic["representative_cards"].as_array().filter(|v| !v.is_empty())
                .ok_or("representative_cards must be a nonempty array")?;
            let mut results = Vec::new();
            for card in cards {
                // A malformed card still emits a row and cannot hide later cards.
                let result = (|| -> Result<Value, String> {
                    let card_id = required(card, "id")?;
                    let name = required(card, "name")?;
                    let linked = card["source_faces"].as_array().is_some_and(|faces| faces.len() > 1)
                        || card["executable_faces"].as_array().is_some_and(|faces| faces.len() > 1)
                        || card.get("source_layout").or_else(|| card.get("source_shape"))
                            .and_then(Value::as_str).is_some_and(|layout| matches!(layout,
                                "doubleface" | "transform" | "transform_like" | "transformLike"
                                | "modal_dfc" | "split" | "aftermath" | "flip" | "adventure" | "prepare"));
                    Ok(json!({"id":card_id,
                        "original":compile(name, required(card,"original_text")?, keyword),
                        "executable":if card["executable_text"].is_null() { json!({"status":"pending_rewrite","structural_pass":false}) }
                        else if linked { json!({"status":"requires_linked_face_adapter","structural_pass":false,
                            "note":"Use audit-cohort.mjs and the native frontend adapter to verify every face and its links."}) }
                        else { compile(name, required(card,"executable_text")?, keyword) }}))
                })();
                results.push(result.unwrap_or_else(|e| json!({"status":"invalid_fixture","error":e})));
            }
            let structural_pass = results.iter().all(|r| r["executable"]["structural_pass"] == true);
            // User-authored status claims are reported, never promoted to verified gameplay.
            Ok(json!({"id":id,"keyword":keyword,"cards":results,
                "structural_pass":structural_pass,"semantic_gate":"requires_behavioral_verification",
                "semantic_equivalence_claim":mechanic.get("semantic_equivalence"),
                "behavioral_assertions":mechanic.get("behavioral_assertions")}))
        })();
        writeln!(out, "{}", row.unwrap_or_else(|e| json!({"id":mechanic.get("id"),"status":"invalid_fixture","error":e})))
            .map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() {
    let result = std::thread::Builder::new().stack_size(64 * 1024 * 1024)
        .spawn(run).expect("audit thread").join().expect("audit thread panicked");
    if let Err(error) = result { eprintln!("{error}"); std::process::exit(1); }
}
