//! Authored strict admission cases; UNRUN during the source-only campaign.
use ironsmith_tools::{CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload};

#[test]
fn eleven_complete_temporary_prevention_bodies_compile_without_oracle_fallback() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/temporary_prevention_bindings.json.fixture")).unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for row in rows.iter().filter(|row| row["proposed_complete"] == true) {
        let mut metadata_lines = vec![format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap())];
        if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
            metadata_lines.push(format!("Power/Toughness: {p}/{t}"));
        }
        let oracle = row["oracle_text"].as_str().unwrap();
        let payload = CardPayload {
            name: row["name"].as_str().unwrap().into(), parse_name: None,
            oracle_text: oracle.into(), raw_oracle_text: oracle.into(),
            parse_input: build_parse_input(&metadata_lines, oracle), metadata_lines,
            other_face_name: None, linked_face_layout: None,
        };
        let snapshot = compile_strict_snapshot_from_payload(&payload);
        if snapshot.parse_status != ParseStatus::StrictCompiled || snapshot.parse_lossy {
            failures.push(format!("{}: {:?}; {}", payload.name, snapshot.parse_error, snapshot.parse_loss_reasons));
        }
        checked += 1;
    }
    assert_eq!(checked, 11);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
