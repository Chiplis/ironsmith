//! Frozen full-body metadata route assertions. Authored, UNRUN.
use ironsmith_tools::{CardPayload, ParseStatus, build_parse_input, compile_strict_snapshot_from_payload};

#[test]
fn five_complete_candidates_keep_metadata_and_never_use_oracle_only_fallback() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/intervening_predicate_cohort.json.fixture"
    )).unwrap();
    assert_eq!(rows.iter().filter(|row| row["proposed_complete"] == true).count(), 5);
    for row in rows.iter().filter(|row| row["proposed_complete"] == true) {
        let metadata_lines = vec![
            format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap()),
            format!("Power/Toughness: {}/{}", row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap()),
        ];
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
        assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled,
            "{}: {:?}; {}", payload.name, snapshot.parse_error, snapshot.parse_loss_reasons);
        assert!(!snapshot.parse_lossy, "{}: {}", payload.name, snapshot.parse_loss_reasons);
    }
}

#[test]
fn lesson_refresh_regressions_compile_complete_metadata_without_fallback() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/intervening_predicate_cohort.json.fixture"
    )).unwrap();
    for (name, oracle_id) in [
        ("Dragonfly Swarm", "83abf1d0-04e4-49e5-bf63-59f77829ffd1"),
        ("Walltop Sentries", "fb3a0910-a582-41ef-b5b9-3cda1f1cd5ce"),
    ] {
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        assert_eq!(row["oracle_id"], oracle_id);
        let raw = row["oracle_text"].as_str().unwrap();
        // The refresh retained the raw ward reminder and its normalized form.
        // Both are complete bodies; neither is an oracle-only retry.
        let normalized = raw.replace(" (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)", "");
        for body in [raw, normalized.as_str()] {
            for head in ["there's", "there’s", "there is"] {
                let oracle = body.replace("there's", head);
                let metadata_lines = vec![
                    format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
                    format!("Type: {}", row["type_line"].as_str().unwrap()),
                    format!("Power/Toughness: {}/{}", row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap()),
                ];
                let payload = CardPayload {
                    name: name.to_string(), parse_name: None,
                    oracle_text: oracle.clone(), raw_oracle_text: raw.to_string(),
                    parse_input: build_parse_input(&metadata_lines, &oracle), metadata_lines,
                    other_face_name: None, linked_face_layout: None,
                };
                let snapshot = compile_strict_snapshot_from_payload(&payload);
                assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled,
                    "{name}: {head}: {:?}; {}", snapshot.parse_error, snapshot.parse_loss_reasons);
                assert!(!snapshot.parse_lossy, "{name}: {head}: {}", snapshot.parse_loss_reasons);
                assert!(!snapshot.has_unimplemented, "{name}: {head}");
            }
        }
    }
}
