use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload,
};
#[test]
fn exact_payment_predicate_cards_preserve_real_restrictions_and_metadata() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/mana_payment_predicate_restrictions.json.fixture"
    ))
    .unwrap();
    assert_eq!(rows.len(), 4);
    for r in rows {
        let mut lines = vec![
            format!("Mana cost: {}", r["mana_cost"].as_str().unwrap()),
            format!("Type: {}", r["type_line"].as_str().unwrap()),
        ];
        if let (Some(p), Some(t)) = (r["power"].as_str(), r["toughness"].as_str()) {
            lines.push(format!("Power/Toughness: {p}/{t}"));
        }
        let oracle = r["oracle_text"].as_str().unwrap();
        let payload = CardPayload {
            name: r["name"].as_str().unwrap().into(),
            parse_name: None,
            oracle_text: oracle.into(),
            raw_oracle_text: oracle.into(),
            parse_input: build_parse_input(&lines, oracle),
            metadata_lines: lines,
            other_face_name: None,
            linked_face_layout: None,
        };
        let snapshot = compile_strict_snapshot_from_payload(&payload);
        assert_eq!(
            snapshot.parse_status,
            ParseStatus::StrictCompiled,
            "{}: {:?}",
            payload.name,
            snapshot.parse_error
        );
        assert!(!snapshot.parse_lossy, "{}", payload.name);
        assert!(
            snapshot
                .compiled_text
                .unwrap()
                .to_lowercase()
                .contains("spend this mana only")
        );
    }
}
