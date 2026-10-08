//! Source-only admission guard. These tests have not been executed.
//! Replace this HOLD with full direct/artifact runtime evidence only after the
//! grant owns both successful exile receipts and each exiled card's owner.
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

#[test]
fn sinister_concierge_complete_frozen_body_remains_held_on_both_routes() {
    let row: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sinister_concierge_hold.json.fixture"
    ))
    .unwrap();
    assert_eq!(row["oracle_id"], "9b14c20f-c4ee-42ea-99cd-099b4eb25883");
    let name = row["card_name"].as_str().unwrap();
    for field in ["raw_oracle_text", "normalized_oracle_text"] {
        let text = row[field].as_str().unwrap();
        assert!(
            compile_to_runtime_definition(name, text, false).is_err(),
            "{field}: do not admit an unproved exile-set/suspend program on the direct route"
        );
        assert!(
            compile_to_artifact(name, text, false).is_err(),
            "{field}: do not serialize an unproved exile-set/suspend program"
        );
    }
}
