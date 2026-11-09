//! Every available exact source face compiles independently and survives artifact transport.
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[test]
fn complete_source_programs_round_trip() {
    let mut count = 0;
    let mut unavailable = Vec::new();
    for fixture in [include_str!(
        "../../../fixtures/custom_mechanics/msem-gestalt-next.json"
    )] {
        let fixture: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for row in fixture["mechanics"].as_array().unwrap() {
            for card in row["representative_cards"].as_array().unwrap() {
                let Some(text) = card["executable_text"].as_str() else {
                    unavailable.push(row["id"].as_str().unwrap().to_string());
                    continue;
                };
                let vocabulary: Vec<ironsmith_core::custom_subtypes::CustomSubtypeDefinition> =
                    serde_json::from_value(
                        card["custom_subtypes"]
                            .as_array()
                            .map(|items| serde_json::Value::Array(items.clone()))
                            .unwrap_or_else(|| serde_json::json!([])),
                    )
                    .unwrap();
                let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    card["name"].as_str().unwrap(),
                );
                for identity in card["custom_keyword_identities"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    builder = builder.with_custom_keyword(identity.as_str().unwrap());
                }
                ironsmith_compiler::with_custom_subtypes(&vocabulary, || {
                    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
                        ironsmith_compiler::word_claims::with_enforcement_options(
                            true,
                            true,
                            || {
                                ironsmith_compiler_runtime::compile_builder_to_artifact(
                                    builder, text, false,
                                )
                            },
                        )
                    });
                    assert!(
                        !loss.is_lossy(),
                        "{}: {}",
                        card["name"],
                        loss.reasons_text()
                    );
                    let (artifact, direct) =
                        compiled.unwrap_or_else(|error| panic!("{}: {error:?}", card["name"]));
                    let decoded =
                        CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
                    assert_eq!(artifact, decoded);
                    let restored =
                        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(
                            &decoded,
                        )
                        .unwrap();
                    // Effects compare by executor identity (Arc::ptr_eq), so two
                    // independent routes are compared through their canonical
                    // wire encodings.
                    let canonical = |definition| {
                        serde_json::to_value(
                            ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition)
                                .unwrap(),
                        )
                        .unwrap()
                    };
                    assert_eq!(canonical(direct), canonical(restored));
                })
                .unwrap();
                count += 1;
            }
        }
    }
    assert!(count > 0);
    assert!(
        unavailable.is_empty(),
        "missing source programs: {unavailable:?}"
    );
}
