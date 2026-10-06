//! Authored source evidence only. Execution remains deferred.
use ironsmith_tools::{CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload};

#[test]
fn four_frozen_color_bodies_compile_without_partial_or_metadata_fallback() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/static_color_bodies.json.fixture")).unwrap();
    assert_eq!(rows.len(), 4);
    for row in rows {
        let mut metadata_lines = vec![
            format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap()),
        ];
        if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
            metadata_lines.push(format!("Power/Toughness: {p}/{t}"));
        }
        // Scryfall's computed colors/identity are evidence, not indicators.
        assert!(row["color_indicator"].is_null());
        let oracle = row["oracle_text"].as_str().unwrap();
        let payload = CardPayload {
            name: row["name"].as_str().unwrap().into(), parse_name: None,
            oracle_text: oracle.into(), raw_oracle_text: oracle.into(),
            parse_input: build_parse_input(&metadata_lines, oracle), metadata_lines,
            other_face_name: None, linked_face_layout: None,
        };
        let snapshot = compile_strict_snapshot_from_payload(&payload);
        assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled,
            "{}: {:?}", payload.name, snapshot.parse_error);
        assert!(!snapshot.parse_lossy, "{}: {}", payload.name, snapshot.parse_loss_reasons);
    }
}
