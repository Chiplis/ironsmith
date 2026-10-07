use ironsmith_tools::{CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload};

#[test]
fn four_frozen_dynamic_characteristic_bodies_compile_strictly_with_metadata() {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/dynamic_characteristic_statics.json.fixture")).unwrap();
    assert_eq!(fixtures.len(), 4);
    let mut failures = Vec::new();
    for row in fixtures {
        let mut metadata_lines = vec![format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap())];
        if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
            metadata_lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        let oracle = row["oracle_text"].as_str().unwrap();
        let payload = CardPayload { name: row["name"].as_str().unwrap().to_string(),
            parse_name: None, oracle_text: oracle.into(), raw_oracle_text: oracle.into(),
            parse_input: build_parse_input(&metadata_lines, oracle), metadata_lines,
            other_face_name: None, linked_face_layout: None };
        let snapshot = compile_strict_snapshot_from_payload(&payload);
        if snapshot.parse_status != ParseStatus::StrictCompiled || snapshot.parse_lossy {
            failures.push(format!("{}: {:?}; {}", payload.name,
                snapshot.parse_error, snapshot.parse_loss_reasons));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
