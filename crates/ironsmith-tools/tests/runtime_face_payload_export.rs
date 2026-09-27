//! Export exact canonical face payloads that a parent-name-only inventory misses.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
#[test]
#[ignore = "audit inventory exporter; writes payloads without compiling or executing cards"]
fn export_canonical_runtime_face_payloads() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("cards.json");
    let bytes = std::fs::read(&path).unwrap();
    let cards: Vec<Value> = serde_json::from_slice(&bytes).unwrap();
    let names: BTreeSet<String> = cards
        .iter()
        .filter(|c| c["digital"] != true)
        .filter_map(|c| c["card_faces"].as_array())
        .flatten()
        .filter_map(|f| f["name"].as_str().map(String::from))
        .collect();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        path.to_str().unwrap(),
        &names.iter().cloned().collect::<Vec<_>>(),
    )
    .unwrap();
    let rows:Vec<_>=payloads.into_values().flatten().map(|p|json!({"name":p.name,"parse_name":p.parse_name,"parse_input":p.parse_input,"oracle_text":p.raw_oracle_text,
        "actions_only":false,"contracts_only":false,"include_definition":false})).collect();
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let report = json!({"scope":"Canonical explicit face-name payloads; no compilation or gameplay claim.","source":"cards.json","cards_sha256":hash,
        "requested_face_names":names,"payload_count":rows.len(),"cards":rows});
    std::fs::write(
        root.join("reports/runtime-audit/canonical-face-payloads.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "Exported {} canonical face payloads",
        report["payload_count"]
    );
}
