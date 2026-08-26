#[path = "subject_verb_registry_shapes/attacking.rs"]
mod attacking;
#[path = "subject_verb_registry_shapes/clause.rs"]
mod clause;
#[path = "subject_verb_registry_shapes/delayed.rs"]
mod delayed;
#[path = "subject_verb_registry_shapes/joint.rs"]
mod joint;
#[path = "subject_verb_registry_shapes/sequences.rs"]
mod sequences;

pub use attacking::*;
pub use clause::*;
pub use delayed::*;
pub use joint::*;
pub use sequences::*;
