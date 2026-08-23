//! Compatibility marker for the split browser package.
//!
//! Browser consumers initialize the engine, compiler, and verifier artifacts
//! through the JavaScript facade. Shared transport types remain available here
//! for Rust-side package tooling.

pub use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, ArtifactDecodeError, ArtifactDiagnostics,
    ArtifactValidationError, CompiledCardArtifact, CompiledCardPayload, ENGINE_SCHEMA_HASH,
    FORMAT_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitBrowserPackage;
