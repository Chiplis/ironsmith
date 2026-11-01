//! Type-line vocabulary must survive compilation and artifact transport.
use ironsmith::types::Subtype;
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[test]
fn known_type_names_preserve_complete_subtype_identity() {
    for (line, expected) in [
        ("Artifact — Monument", vec![Subtype::Monument]),
        ("Creature — Time Lord", vec![Subtype::TimeLord]),
        ("Land — Urza's Power-Plant", vec![Subtype::Urzas, Subtype::PowerPlant]),
        ("Creature — Assembly-Worker", vec![Subtype::AssemblyWorker]),
        ("Creature — C'tan", vec![Subtype::Ctan]),
        ("Plane — New Phyrexia", vec![Subtype::NewPhyrexia]),
        ("Dungeon — Undercity", vec![Subtype::Undercity]),
    ] {
        let (artifact, definition) = ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Metadata witness", format!("Type: {line}"), false)).unwrap();
        assert_eq!(definition.card.subtypes, expected, "{line}");
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(restored, artifact);
        let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
        assert_eq!(materialized.card.subtypes, expected, "{line}");
    }
    assert!(Subtype::Monument.is_artifact_subtype());
}
#[test]
fn unread_type_metadata_cannot_silently_compile_as_a_different_card() {
    for line in ["Artifact Unreadkind", "Artifact — Unreadsubtype", "Creature — Wizard Unreadsubtype", "Plane — Unreadplane"] {
        let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_runtime_definition("Unknown metadata", format!("Type: {line}"), false));
        assert!(result.is_err(), "unread metadata compiled: {line}");
    }
}

#[test]
fn every_registered_planar_name_survives_its_collective_type_line() {
    for subtype in Subtype::all_planar_types() {
        let name = subtype.display_name();
        let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Planar metadata", format!("Type: Plane — {name}"), false).unwrap();
        assert_eq!(definition.card.subtypes, vec![*subtype], "{name}");
    }
}
