//! Every owned real-source Recite/Bloodprice candidate compiles as a complete
//! card through direct and artifact routes, including its unrelated abilities.
use ironsmith::CardDefinition;
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
fn compile(name: &str, keyword: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    name,
                )
                .with_custom_keyword(keyword),
                text,
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    compiled
        .definition
        .attribute_custom_keyword_payment(keyword);
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone())
            .unwrap();
    let artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(direct.card.id.0),
            name: direct.card.name.clone(),
            face_name: None,
            other_face: None,
            linked_face_layout: None,
        },
        CompiledCardPayload {
            definition: wire,
            canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"),
            ability_labels: ironsmith_text::ability_surface_texts(&direct),
        },
        "enspirit-cost-composition",
        text.as_bytes(),
    );
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}

#[test]
fn complete_real_source_recite_and_bloodprice_candidates_compile_losslessly() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-middle-keywords.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["mechanics"].as_array().unwrap() {
        if !matches!(row["id"].as_str(), Some("msem-594" | "msem-655")) {
            continue;
        }
        for card in row["representative_cards"].as_array().unwrap() {
            let name = card["name"].as_str().unwrap();
            let keyword = row["keyword"].as_str().unwrap().to_ascii_lowercase();
            let text = card["executable_text"]
                .as_str()
                .expect("complete source must have materialized candidate");
            for definition in compile(name, &keyword, text) {
                assert_eq!(definition.card.name, name);
            }
            count += 1;
        }
    }
    assert_eq!(count, 24, "17 Recite and seven Bloodprice source cards");
}
