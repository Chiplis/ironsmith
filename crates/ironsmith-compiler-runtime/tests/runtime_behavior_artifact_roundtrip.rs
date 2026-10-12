//! Run the same gameplay scenarios against serialized/materialized definitions.
//! The shared suite calls this test-local factory instead of the tools builder.
mod ironsmith_tools {
    pub fn parse_card_definition_with_runtime_builder(
        name: &str,
        text: &str,
        _allow_unsupported: bool,
    ) -> Result<ironsmith::CardDefinition, String> {
        let (artifact, _) = ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
            .map_err(|error| error.to_string())?;
        let json = artifact.to_json().map_err(|error| error.to_string())?;
        let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&json)
            .map_err(|error| error.to_string())?;
        restored.validate().map_err(|error| error.to_string())?;
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
            .map_err(|error| error.to_string())
    }
}

include!("../../ironsmith-tools/tests/runtime_behavior_regressions.rs");
