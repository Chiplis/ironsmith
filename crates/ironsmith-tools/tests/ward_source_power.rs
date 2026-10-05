use ironsmith::CardType;
use ironsmith::card::PowerToughness;
use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_runtime_definition_from_payload,
    compile_strict_snapshot_from_payload,
};

#[test]
fn raubahn_dynamic_ward_keeps_metadata_on_the_strict_payload_route() {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/lossy_metadata.json.fixture"
    ))
    .unwrap();
    let fixture = fixtures
        .iter()
        .find(|fixture| fixture["name"] == "Raubahn, Bull of Ala Mhigo")
        .unwrap();
    let metadata_lines = vec![
        format!("Mana cost: {}", fixture["mana_cost"].as_str().unwrap()),
        format!("Type: {}", fixture["type_line"].as_str().unwrap()),
        format!(
            "Power/Toughness: {}/{}",
            fixture["power"].as_str().unwrap(),
            fixture["toughness"].as_str().unwrap()
        ),
    ];
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
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{:?}",
        snapshot.parse_error
    );
    assert!(!snapshot.parse_lossy, "{}", snapshot.parse_loss_reasons);
    let definition = compile_runtime_definition_from_payload(&payload).unwrap();
    assert_eq!(definition.card.mana_cost.unwrap().to_oracle(), "{1}{R}");
    assert_eq!(definition.card.card_types, vec![CardType::Creature]);
    assert_eq!(
        definition.card.power_toughness,
        Some(PowerToughness::fixed(2, 2))
    );
}
