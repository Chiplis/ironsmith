//! Frozen campaign inputs; authored without running the campaign validation gate.
use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload,
};

#[test]
fn three_complete_cast_quantity_cards_compile_strictly_without_metadata_fallback() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/cast_event_quantities.json.fixture"
    ))
    .unwrap();
    assert_eq!(rows.len(), 3);
    let mut failures = Vec::new();
    for row in rows {
        let mut metadata_lines = vec![
            format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap()),
        ];
        if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
            metadata_lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        let oracle = row["oracle_text"].as_str().unwrap();
        assert_eq!(row["baseline"]["raw_oracle_text"], oracle);
        assert_eq!(row["baseline"]["parse_status"], "parse_failed");
        assert!(row["baseline"]["parse_error"].as_str().unwrap().contains(
            "event-derived amount requires a compatible trigger or prior effect"
        ));
        let payload = CardPayload {
            name: row["name"].as_str().unwrap().to_owned(),
            parse_name: None,
            oracle_text: oracle.to_owned(),
            raw_oracle_text: oracle.to_owned(),
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
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
