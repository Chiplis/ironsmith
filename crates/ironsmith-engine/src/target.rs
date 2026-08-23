//! Target specification system for spells and abilities.
//!
//! Runtime consumes the shared target model from `ironsmith-core` and keeps
//! targeting legality/evaluation behavior in runtime-only modules.

pub use crate::filter::{
    Comparison, FilterContext, ObjectFilter, ObjectRef, PlayerFilter, PlayerFilterExt, PtReference,
    TaggedObjectConstraint, TaggedOpbjectRelation,
};
pub use ironsmith_core::{
    ChooseSpec, ChooseSpecSurfaceHint, SacrificedObjectKind, SourceReferenceSurface,
};
