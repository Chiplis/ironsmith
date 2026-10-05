use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload,
};

#[test]
fn target_characteristic_comparison_payloads_compile_strictly_without_metadata_fallback() {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/target_characteristic_comparisons.json.fixture"
    ))
    .unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for row in &fixtures {
        let mut metadata_lines = vec![
            format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap()),
        ];
        if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
            metadata_lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        if let Some(loyalty) = row["loyalty"].as_str() {
            metadata_lines.push(format!("Loyalty: {loyalty}"));
        }
        let oracle = row["oracle_text"].as_str().unwrap();
        let payload = CardPayload {
            name: row["name"].as_str().unwrap().to_string(),
            parse_name: None,
            oracle_text: oracle.to_string(),
            raw_oracle_text: oracle.to_string(),
            parse_input: build_parse_input(&metadata_lines, oracle),
            metadata_lines,
            other_face_name: None,
            linked_face_layout: None,
        };
        let snapshot = compile_strict_snapshot_from_payload(&payload);
        if snapshot.parse_status != ParseStatus::StrictCompiled || snapshot.parse_lossy {
            failures.push(format!(
                "{}: {:?}; {}",
                payload.name, snapshot.parse_error, snapshot.parse_loss_reasons
            ));
        }
        checked += 1;
    }
    assert_eq!(checked, 1);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
