//! UNVALIDATED exact-card closure after the shared self-recipient batch owner.
use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload,
};
#[test]
fn akroan_war_full_metadata_body_is_strict_without_fallback() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/source_lifetime_control.json.fixture"
    ))
    .unwrap();
    let row = rows
        .iter()
        .find(|row| row["name"] == "The Akroan War")
        .unwrap();
    assert_eq!(row["proposed_complete"], true);
    let oracle = row["oracle_text"].as_str().unwrap();
    let metadata_lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    let payload = CardPayload {
        name: row["name"].as_str().unwrap().into(),
        parse_name: None,
        oracle_text: oracle.into(),
        raw_oracle_text: oracle.into(),
        parse_input: build_parse_input(&metadata_lines, oracle),
        metadata_lines,
        other_face_name: None,
        linked_face_layout: None,
    };
    let snapshot = compile_strict_snapshot_from_payload(&payload);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{:?}",
        snapshot.parse_error
    );
    assert!(!snapshot.parse_lossy, "{}", snapshot.parse_loss_reasons);
}
