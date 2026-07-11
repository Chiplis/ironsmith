#[path = "subject_verb_registry_shapes/clause.rs"]
mod clause;
#[path = "subject_verb_registry_shapes/delayed.rs"]
mod delayed;
#[path = "subject_verb_registry_shapes/joint.rs"]
mod joint;
#[path = "subject_verb_registry_shapes/sequences.rs"]
mod sequences;

pub(crate) use clause::*;
pub(crate) use delayed::*;
pub(crate) use joint::*;
pub(crate) use sequences::*;
