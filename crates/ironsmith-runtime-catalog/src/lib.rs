#![allow(dead_code)]

//! Compiled-artifact materialization and runtime catalog registration.
//!
//! The gameplay engine owns no parser or artifact decoder. This service sits
//! above the engine and the eight independent effect materializer families.

pub use ironsmith::*;

#[path = "../../ironsmith-engine/src/artifact_materializer.rs"]
pub mod artifact_materializer;

#[derive(Debug)]
pub enum ArtifactRegistrationError {
    Invalid(ironsmith_compiled_artifact::ArtifactValidationError),
    Materialization(artifact_materializer::ArtifactMaterializationError),
    NameMismatch {
        artifact: String,
        definition: String,
    },
}

impl std::fmt::Display for ArtifactRegistrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(error) => error.fmt(formatter),
            Self::Materialization(error) => error.fmt(formatter),
            Self::NameMismatch {
                artifact,
                definition,
            } => write!(
                formatter,
                "compiled-card artifact name {artifact:?} does not match materialized definition {definition:?}"
            ),
        }
    }
}

impl std::error::Error for ArtifactRegistrationError {}

pub trait CardRegistryArtifactExt {
    fn register_compiled_artifact(
        &mut self,
        artifact: &ironsmith_compiled_artifact::CompiledCardArtifact,
    ) -> Result<(), ArtifactRegistrationError>;
}

impl CardRegistryArtifactExt for ironsmith::cards::CardRegistry {
    fn register_compiled_artifact(
        &mut self,
        artifact: &ironsmith_compiled_artifact::CompiledCardArtifact,
    ) -> Result<(), ArtifactRegistrationError> {
        artifact
            .validate()
            .map_err(ArtifactRegistrationError::Invalid)?;
        let definition = artifact_materializer::materialize_artifact(artifact)
            .map_err(ArtifactRegistrationError::Materialization)?;
        if !artifact
            .card
            .name
            .eq_ignore_ascii_case(&definition.card.name)
        {
            return Err(ArtifactRegistrationError::NameMismatch {
                artifact: artifact.card.name.clone(),
                definition: definition.card.name,
            });
        }
        self.register(definition);
        Ok(())
    }
}
