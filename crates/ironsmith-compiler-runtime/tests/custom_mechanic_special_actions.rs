//! Source-backed, direct/artifact parity for an independently described
//! face-down entry and its separate exile/cast permission.
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
#[path = "custom_final_next_wave_support.rs"] mod source_support;

#[test]
fn substantiated_entry_keeps_filtered_priority_cast_permission_on_both_routes() {
    let input = "Type: Sorcery\nPut target card in your graveyard onto the battlefield face down as a 2/2 creature. You may exile and cast it face up any time you have priority if it is a noncreature card with a mana cost. You still pay its costs.";
    let direct = compile_to_runtime_definition("Described entry probe", input, false).unwrap();
    let (artifact, _) = compile_to_artifact("Described entry probe", input, false).unwrap();
    let encoded = artifact.to_json().unwrap();
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&encoded).unwrap();
    let materialized = materialize_artifact(&restored).unwrap();
    for definition in [direct, materialized] {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DescribedFaceDownCastPermission"), "{debug}");
        assert!(debug.contains("require_printed_mana_cost: true"), "{debug}");
        assert!(debug.contains("excluded_card_types: [Creature]"), "{debug}");
        assert!(debug.contains("no_turn_up_permission: true"), "{debug}");
    }
}

#[test]
fn exact_source_substantiate_variants_retain_explicit_action_permission() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-special-actions.json")).unwrap();
    let rows = fixture["mechanics"].as_array().unwrap().iter()
        .filter(|row| row["keyword"] == "Substantiate").collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    let mut keys = std::collections::BTreeSet::new();
    let mut face_count = 0;
    for row in rows {
        for card in row["representative_cards"].as_array().unwrap() {
            assert!(keys.insert(card["source_card_key"].as_str().unwrap().to_owned()));
            let routes = source_support::linked_definitions(card)
                .unwrap_or_else(|error| panic!("{}: {error}", card["name"]));
            face_count += routes.len();
            for [native, artifact] in routes {
                let native_wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(native.clone()).unwrap();
                let artifact_wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(artifact.clone()).unwrap();
                assert_eq!(serde_json::to_value(native_wire).unwrap(), serde_json::to_value(artifact_wire).unwrap(), "complete semantic parity for {}", card["name"]);
                for definition in [native, artifact] {
                    assert!(format!("{definition:?}").contains("DescribedFaceDownCastPermission"));
                }
            }
        }
    }
    assert_eq!(face_count, 2);
    assert_eq!(keys, ["Saffrisa's Darkcaster_OTH".to_owned(), "Weaver of Mists_MS3".to_owned()].into_iter().collect());
}
