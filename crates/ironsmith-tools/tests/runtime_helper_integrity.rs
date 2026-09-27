//! Capture fallback/metadata behavior of the scenario helper on confirmed names.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "audit helper integrity exporter, not a gameplay correctness assertion"]
fn report_runtime_helper_integrity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let summary = root.join("reports/runtime-audit/summary.json");
    let bytes = std::fs::read(&summary).unwrap();
    let selection: Value = serde_json::from_slice(&bytes).unwrap();
    let names: Vec<String> = selection["confirmed_card_outcomes"]["confirmed_failure_cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_string())
        .collect();
    let cards = root.join("cards.json");
    let payloads =
        ironsmith_tools::load_card_payloads_by_names(cards.to_str().unwrap(), &names).unwrap();
    let mut rows = Vec::new();
    for name in &names {
        let key = ironsmith_tools::normalize_lookup_name(name);
        let payload = &payloads[&key][0];
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_tools::compile_runtime_definition_from_payload(payload)
        });
        let row = match result {
            Ok(d) => {
                json!({"card":name,"status":"compiled","parse_lossy":loss.is_lossy(),"parse_loss":loss.reasons_text(),
    "metadata":{"name":d.card.name.to_string(),"mana_cost":d.card.mana_cost.as_ref().map(|m|m.to_oracle()),
     "card_types":d.card.card_types.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),
     "power_toughness_debug":d.card.power_toughness.as_ref().map(|pt|format!("{pt:?}"))},
    "parse_input":payload.parse_input,"parse_name":payload.parse_name,"payload_name":payload.name})
            }
            Err(error) => {
                json!({"card":name,"status":"helper_compile_failed","error":error,"parse_lossy":loss.is_lossy(),"parse_loss":loss.reasons_text()})
            }
        };
        rows.push(row);
    }
    let binary = std::env::current_exe().unwrap();
    let source = root.join("crates/ironsmith-tools/tests/runtime_helper_integrity.rs");
    let selection_sha: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let report = json!({"scope":"Fresh execution of compile_runtime_definition_from_payload with parse-loss capture for all selected confirmed names; no gameplay run.","selected_cards":names.len(),"rows":rows,
  "provenance":{"binary":binary,"binary_sha256":hash(&binary),"source_sha256":hash(&source),"cards_sha256":hash(&cards),"selection_summary_sha256":selection_sha}});
    std::fs::write(
        root.join("reports/runtime-audit/confirmed-helper-integrity.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("Wrote {} helper-integrity rows", names.len());
}
