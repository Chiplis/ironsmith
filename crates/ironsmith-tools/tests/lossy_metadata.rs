use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::{CardType, Subtype};
use ironsmith_tools::{
    CardPayload, ParseStatus, build_parse_input, compile_runtime_definition_from_payload,
    compile_strict_snapshot_from_payload,
};

#[test]
fn contextual_source_repair_preserves_metadata_and_linked_face_identity() {
    let metadata_lines = vec![
        "Mana cost: {2}{W}{U}".to_string(),
        "Type: Legendary Creature — Bird Wizard".to_string(),
        "Power/Toughness: 2/3".to_string(),
        "Color indicator: White, Blue".to_string(),
    ];
    let oracle = "Other Bird creatures get +1/+1 for each feather counter on Counter Custodian.";
    let payload = CardPayload {
        name: "Counter Custodian // Counter Remnant".to_string(),
        parse_name: Some("Counter Custodian".to_string()),
        oracle_text: oracle.to_string(),
        raw_oracle_text: oracle.to_string(),
        parse_input: build_parse_input(&metadata_lines, oracle),
        metadata_lines,
        other_face_name: Some("Counter Remnant".to_string()),
        linked_face_layout: Some(LinkedFaceLayout::TransformLike),
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
    assert_eq!(definition.card.name, "Counter Custodian");
    assert_eq!(definition.card.mana_cost.unwrap().to_oracle(), "{2}{W}{U}");
    assert_eq!(definition.card.card_types, vec![CardType::Creature]);
    assert_eq!(
        definition.card.subtypes,
        vec![Subtype::Bird, Subtype::Wizard]
    );
    assert_eq!(
        definition.card.power_toughness,
        Some(PowerToughness::fixed(2, 3))
    );
    assert!(definition.card.color_indicator.is_some());
    assert_eq!(
        definition.card.linked_face_layout,
        LinkedFaceLayout::TransformLike
    );
    assert_eq!(
        definition.card.other_face_name.as_deref(),
        Some("Counter Remnant")
    );
    assert!(definition.card.other_face.is_some());
    assert!(definition.card.transforming_dfc);
}

#[test]
fn repaired_named_face_trigger_retains_the_real_transform_link() {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/lossy_metadata.json.fixture"
    ))
    .unwrap();
    let fixture = fixtures
        .iter()
        .find(|fixture| fixture["name"] == "Norman Osborn")
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
        name: fixture["canonical_name"].as_str().unwrap().to_string(),
        parse_name: Some("Norman Osborn".into()),
        oracle_text: ironsmith_tools::postprocess_oracle_text(oracle),
        raw_oracle_text: oracle.to_string(),
        parse_input: build_parse_input(&metadata_lines, oracle),
        metadata_lines,
        other_face_name: Some(fixture["other_face_name"].as_str().unwrap().to_string()),
        linked_face_layout: Some(LinkedFaceLayout::TransformLike),
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
    assert_eq!(definition.card.name, "Norman Osborn");
    assert_eq!(definition.card.mana_cost.unwrap().to_oracle(), "{1}{U}");
    assert_eq!(
        definition.card.power_toughness,
        Some(PowerToughness::fixed(1, 1))
    );
    assert_eq!(
        definition.card.other_face_name.as_deref(),
        Some("Green Goblin")
    );
    assert_eq!(
        definition.card.linked_face_layout,
        LinkedFaceLayout::TransformLike
    );
    assert!(definition.card.transforming_dfc && definition.card.other_face.is_some());
}
