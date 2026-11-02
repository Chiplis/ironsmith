//! Exact formal actor+consumer inventories. These source obligations stay
//! strict while their ordinary frontend/backend gaps are completed.
#[path = "custom_final_next_wave_support.rs"] mod source_support;
use serde_json::Value;
use std::collections::BTreeSet;
fn key(card: &Value) -> &str {
    let key = card["source_card_key"].as_str().expect("source identity");
    key.strip_prefix("rev:").or_else(|| key.strip_prefix("revolution:")).or_else(|| key.strip_prefix("msem:")).unwrap_or(key)
}
fn collect(fixture: &str, row_id: &str, expected: &[&str], expected_faces: usize) {
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    let rows: Vec<_> = fixture["mechanics"].as_array().unwrap().iter().filter(|row| row["id"] == row_id).collect();
    assert_eq!(rows.len(), 1, "exact formal source row");
    let mut actual = BTreeSet::new(); let mut faces = 0; let mut failures = Vec::new();
    for card in rows[0]["representative_cards"].as_array().unwrap() {
        let identity = key(card); assert!(actual.insert(identity.to_owned()), "duplicate physical source {identity}");
        let originals = card["source_faces"].as_array().expect("complete immutable original faces");
        faces += originals.len();
        match source_support::linked_definitions(card) {
            Err(error) => failures.push(format!("{identity}: {error}")),
            Ok(routes) => {
                if routes.len() != originals.len() { failures.push(format!("{identity}: {} complete faces became {} compiled faces", originals.len(), routes.len())); }
                for [native, artifact] in routes {
                    if ironsmith::cards::generated_definition_has_unimplemented_content(&native)
                        || ironsmith::cards::generated_definition_has_unimplemented_content(&artifact) {
                        failures.push(format!("{identity}: unimplemented source content"));
                    }
                    let encode = |definition| -> Result<Value, String> {
                        let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).map_err(|error| format!("{error:?}"))?;
                        serde_json::to_value(wire).map_err(|error| error.to_string())
                    };
                    match (encode(native), encode(artifact)) {
                        (Ok(native), Ok(artifact)) if native == artifact => {},
                        (Ok(_), Ok(_)) => failures.push(format!("{identity}: complete typed native/artifact definitions differ")),
                        (native, artifact) => failures.push(format!("{identity}: typed encoding errors native={native:?} artifact={artifact:?}")),
                    }
                }
            }
        }
    }
    assert_eq!(actual, expected.iter().map(|key| (*key).to_owned()).collect());
    assert_eq!(faces, expected_faces, "count physical source faces, including non-action siblings");
    assert!(failures.is_empty(), "All complete-source failures:\n{}", failures.join("\n"));
}
#[test]
fn cackle_all_three_complete_actors_and_consumers_on_both_independent_routes() {
    collect(include_str!("../../../fixtures/custom_mechanics/msem-standard-actions.json"), "msem-rules-cackle",
        &["Mind-Pryer_HZN", "Deals with Devils_OTH", "Mad Queen Margarete_MSL"], 3);
}
#[test]
fn soar_complete_linked_front_and_back_on_both_independent_routes() {
    collect(include_str!("../../../fixtures/custom_mechanics/msem-standard-actions.json"), "msem-rules-soar",
        &["Mounting Up//Taking Flight_MSC"], 2);
}
#[test]
fn recursive_both_designation_and_nonrecursive_consumers_keep_all_ordinary_siblings() {
    collect(include_str!("../../../fixtures/custom_mechanics/msem-recursive-source-complete.json"), "msem-rules-recursive",
        &["Xirix's Study_SPK", "Mayar, Nullcrowned_BTB"], 2);
}
#[test]
fn policy_all_ten_physical_cards_keep_both_complete_faces() {
    collect(include_str!("../../../fixtures/custom_mechanics/revolution-policy.json"), "rev-policy-conditions",
        &["Ascend to Power//Omnipotence_CNY", "Distributive Justice//Equality_CNY", "Free Market//Industrialism_CNY", "Golden Age//Harmony_CNY", "Meeting of the Elites//Aristocracy_CNY", "Research Grant//Progress_CNY", "Security Initiative//Surveillance State_CNY", "Silence the Opposition//Authoritarianism_CNY", "Sparks of a Revolution//Freedom_CNY", "Together in Arms//Communitarianism_CNY"], 20);
}
