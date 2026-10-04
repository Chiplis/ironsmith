//! Authoritative campaign-route check; complements artifact/runtime scenarios.
#[test]
fn pack_tactics_frozen_family_passes_authoritative_compile_gate() {
    let sources: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/card-failure-campaign/predicates/pack-tactics.json"
    ))
    .unwrap();
    for source in sources.as_array().unwrap() {
        let name = source["name"].as_str().unwrap();
        let payload = ironsmith_tools::load_card_payloads_by_name(
            ironsmith_tools::default_cards_path().to_str().unwrap(),
            name,
        )
        .unwrap()
        .remove(0);
        let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload);
        assert_eq!(
            snapshot.parse_status,
            ironsmith_tools::ParseStatus::StrictCompiled,
            "{name}: {snapshot:#?}"
        );
        assert!(
            snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
            "{name}: {snapshot:#?}"
        );
    }
}
