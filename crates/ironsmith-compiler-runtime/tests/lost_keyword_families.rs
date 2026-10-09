//! Source-authored and deliberately unrun (cf8 p04): "Permanents your
//! opponents control lose hexproof, indestructible, protection, shroud, and
//! ward until end of turn." (Shay Cormac): bare "protection" and "ward" name
//! every instance of those keywords, whatever their quality or cost, so they
//! remove the whole static family (CR 702.16, 702.21).
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/lost_keyword_families.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

#[test]
fn shay_cormac_strips_every_protection_and_ward() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("RemoveStaticAbilityFamily(Protection)"), "{debug}");
        assert!(debug.contains("RemoveStaticAbilityFamily(Ward)"), "{debug}");
        assert!(debug.contains("Hexproof") && debug.contains("Shroud"), "{debug}");
    }
}
