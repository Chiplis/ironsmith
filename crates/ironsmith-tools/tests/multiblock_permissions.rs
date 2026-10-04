use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload,
};

#[test]
fn unlimited_blocking_subset_uses_full_metadata_without_oracle_fallback() {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/multiblock_permissions.json.fixture"
    ))
    .unwrap();
    let mut checked = 0;
    let mut failures = Vec::new();
    for fixture in fixtures
        .iter()
        .filter(|fixture| fixture["repair_group"] != "pending_compound_or_conditional")
    {
        let mut metadata_lines = vec![
            format!("Mana cost: {}", fixture["mana_cost"].as_str().unwrap()),
            format!("Type: {}", fixture["type_line"].as_str().unwrap()),
        ];
        if let (Some(power), Some(toughness)) =
            (fixture["power"].as_str(), fixture["toughness"].as_str())
        {
            metadata_lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        let oracle = fixture["oracle_text"].as_str().unwrap();
        let payload = CardPayload {
            name: fixture["name"].as_str().unwrap().to_string(),
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
    assert_eq!(checked, 18);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
