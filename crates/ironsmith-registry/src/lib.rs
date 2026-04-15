//! Registry ownership crate for the split workspace.
//!
//! This crate owns the registry generation policy and the bridged runtime
//! registry implementation source. Runtime compiles those bridged sources
//! without taking a Cargo dependency on `ironsmith-registry`, which keeps the
//! workspace dependency rules intact while moving registry policy ownership out
//! of the runtime crate.

/// Marker type documenting the concrete registry/catalog ownership boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RegistryCatalog;

/// Marker type documenting the concrete registry loading/build ownership boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RegistryLoader;

pub use ironsmith_compiler::{
    CardTextError, CompilerFacade, CompilerSourceDocument, ParseAnnotations, TextSpan,
    WorkspaceSplitMarker,
};
