//! JSON-lines subprocess for the bounded, resumable corpus audit driver.
//! `--inventory <cards.json>` exports the canonical source payloads; otherwise
//! stdin contains one AuditInput per line and stdout one result per line.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use ironsmith::CardId;
use ironsmith_compiler::CardDefinitionBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Serialize, Deserialize)]
struct AuditInput {
    name: String,
    parse_name: Option<String>,
    parse_input: String,
    oracle_text: String,
    #[serde(default)]
    contracts_only: bool,
    #[serde(default)]
    include_definition: bool,
    #[serde(default)]
    actions_only: bool,
}

fn input(payload: ironsmith_tools::CardPayload) -> AuditInput {
    AuditInput {
        name: payload.name,
        parse_name: payload.parse_name,
        parse_input: payload.parse_input,
        oracle_text: payload.raw_oracle_text,
        contracts_only: false,
        include_definition: false,
        actions_only: false,
    }
}

fn inventory(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let raw: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let names: BTreeSet<String> = raw
        .iter()
        .filter_map(|card| card.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();
    let face_names: BTreeSet<String> = raw
        .iter()
        .filter_map(|card| card.get("card_faces").and_then(Value::as_array))
        .flatten()
        .filter_map(|face| face.get("name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    // Explicit includes preserve banned/not-currently-legal paper cards too.
    let records = ironsmith_tools::load_registry_cards_with_explicit_includes(path, &names)?;
    let mut cards: BTreeMap<String, AuditInput> = records
        .into_values()
        .map(|record| (record.payload.name.clone(), input(record.payload)))
        .collect();
    // Request every named face explicitly. The canonical parent's linked-layout
    // helper expands transform/split/flip, but not adventure/modal/prepare/etc.
    // Parent-only coverage would silently omit those independently executable
    // spells. Standalone face probes still do not validate transitions between
    // faces or their shared card identity.
    let requested: Vec<_> = names.union(&face_names).cloned().collect();
    let faces = ironsmith_tools::load_card_payloads_by_names(path, &requested)?;
    for face in faces.into_values().flatten() {
        cards
            .entry(face.name.clone())
            .or_insert_with(|| input(face));
    }
    let mut exclusions = BTreeMap::new();
    for card in &raw {
        if let Some(name) = card.get("name").and_then(Value::as_str) {
            if !cards.contains_key(name) {
                exclusions.entry(name.to_string()).or_insert(json!({
                    "name": name,
                    "status": "not_exercised",
                    "reason": if card.get("digital").and_then(Value::as_bool).unwrap_or(false) {
                        "digital-only card excluded by canonical paper-card loader"
                    } else { "canonical loader did not produce a payload" }
                }));
            }
        }
    }
    let face_exclusions: Vec<_> = face_names
        .iter()
        .filter(|name| !cards.contains_key(*name))
        .map(|name| json!({"name":name,"status":"not_exercised",
            "reason":"canonical loader did not produce a payload for this explicitly requested face"}))
        .collect();
    Ok(json!({
        "raw_rows": raw.len(), "unique_source_names": names.len(),
        "unique_face_names": face_names.len(), "face_exclusions": face_exclusions,
        "explicit_named_face_inventory": true,
        "linked_face_transition_coverage": "not_proven_by_standalone_face_probes",
        "payload_count": cards.len(), "exclusions": exclusions.into_values().collect::<Vec<_>>(),
        "cards": cards.into_values().collect::<Vec<_>>()
    }))
}

fn audit(request: &AuditInput) -> Value {
    let builder = CardDefinitionBuilder::new(
        CardId::new(),
        request.parse_name.as_deref().unwrap_or(&request.name),
    );
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_registry::compile_builder_to_artifact(builder, &request.parse_input, false)
    });
    let (artifact, definition) = match compiled {
        Ok(value) => value,
        Err(error) => {
            let status = if matches!(error, ironsmith_tools::CompilerIntegrationError::Parse(_)) {
                "compile_failed"
            } else {
                "materialization_failed"
            };
            return json!({"status": status, "error": error.to_string()});
        }
    };
    let serialized =
        serde_json::to_value(&artifact.payload.definition).expect("wire definition serializes");
    let mut contracts = ironsmith_tools::runtime_audit::contracts::audit(&serialized);
    contracts.extend(ironsmith_tools::runtime_audit::capabilities::audit(
        &definition,
    ));
    let observations = if request.actions_only {
        ironsmith_tools::runtime_audit::actions::audit(&definition)
    } else if request.contracts_only {
        Vec::new()
    } else {
        ironsmith_tools::runtime_audit::execution::audit(&definition)
    };
    let mut result = json!({
        "status": "compiled", "parse_lossy": loss.is_lossy(), "parse_loss": loss.reasons_text(),
        "compiled_text": definition.canonical_text,
        "has_unimplemented": ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "contracts": contracts, "execution": observations,
        "execution_requested": !request.contracts_only,
        "artifact_checksum": artifact.payload_checksum,
        "semantic_correctness": "not_proven"
    });
    if request.include_definition {
        result["definition"] = serialized;
    }
    result
}

fn worker() {
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line.expect("worker input readable");
        if line.trim().is_empty() {
            continue;
        }
        let started = Instant::now();
        let request: AuditInput = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                writeln!(
                    out,
                    "{}",
                    json!({"status": "invalid_request", "error": error.to_string()})
                )
                .unwrap();
                out.flush().unwrap();
                continue;
            }
        };
        let result = catch_unwind(AssertUnwindSafe(|| audit(&request)));
        let mut result = match result {
            Ok(value) => value,
            Err(payload) => {
                json!({"status": "panicked", "error": payload.downcast_ref::<String>().cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|message| message.to_string()))
                .unwrap_or_else(|| "non-string panic".to_string())})
            }
        };
        result["name"] = json!(request.name);
        result["elapsed_ms"] = json!(started.elapsed().as_millis());
        writeln!(out, "{result}").unwrap();
        out.flush().unwrap();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        if arg != "--inventory" {
            return Err(format!("unknown argument {arg}").into());
        }
        let path = args.next().ok_or("--inventory requires cards.json path")?;
        serde_json::to_writer(std::io::stdout().lock(), &inventory(&path)?)?;
        return Ok(());
    }
    std::thread::Builder::new()
        .name("runtime-audit-worker".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(worker)?
        .join()
        .map_err(|_| "audit worker thread panicked")?;
    Ok(())
}
