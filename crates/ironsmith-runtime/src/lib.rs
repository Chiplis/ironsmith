//! Compatibility facade preserving the historical `ironsmith` import path.

pub use ::engine::*;

pub use ironsmith_runtime_catalog::{ArtifactRegistrationError, CardRegistryArtifactExt};

pub mod artifact_materializer {
    pub use ironsmith_runtime_catalog::artifact_materializer::*;
}

pub mod compiled_text {
    pub use ironsmith_text::compiled_text::*;
}
