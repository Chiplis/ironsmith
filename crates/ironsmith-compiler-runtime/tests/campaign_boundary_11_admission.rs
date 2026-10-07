//! Source-authored admission regressions. UNRUN under the campaign execution gate.
use ironsmith_compiled_artifact::{ArtifactValidationError, FORMAT_VERSION};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;
use ironsmith_runtime_catalog::artifact_materializer::{ArtifactMaterializationError, materialize_artifact};

#[test]
fn direct_materializer_and_registry_reject_old10_and_relabeled_old_schema() {
    let name = "Activation combat boundary witness";
    let (current, _) = ironsmith_compiler_runtime::compile_to_artifact(
        name, "Type: Land — Forest\n{T}: Add {G}.", false,
    ).unwrap();
    assert_eq!(current.format_version, FORMAT_VERSION);
    materialize_artifact(&current).expect("the fresh current definition must be admitted");
    let mut valid_registry = ironsmith::cards::CardRegistry::new();
    valid_registry.register_compiled_artifact(&current).unwrap();
    assert!(valid_registry.get(name).is_some());

    let mut old_format = current.clone();
    old_format.format_version = 10;
    old_format.refresh_checksum();
    assert!(matches!(materialize_artifact(&old_format),
        Err(ArtifactMaterializationError::InvalidArtifact(
            ArtifactValidationError::UnsupportedFormat { found: 10, expected }
        )) if expected == FORMAT_VERSION));

    let mut relabeled = current;
    relabeled.engine_schema_hash =
        "e27b521de2a44a2c1c3349a8b8cbf5392da6882c14270112de24faecced0adc9".into();
    relabeled.refresh_checksum();
    assert!(matches!(materialize_artifact(&relabeled),
        Err(ArtifactMaterializationError::InvalidArtifact(
            ArtifactValidationError::EngineSchemaMismatch { .. }
        ))));

    for artifact in [old_format, relabeled] {
        let mut registry = ironsmith::cards::CardRegistry::new();
        assert!(registry.register_compiled_artifact(&artifact).is_err());
        assert!(registry.get(name).is_none(), "rejection must precede insertion");
    }
}
