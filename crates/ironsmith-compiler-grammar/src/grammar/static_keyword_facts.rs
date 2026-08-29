//! Typed grammar facts for static-keyword sentence families.
//!
//! The submodules follow the source-family clusters during migration so that
//! phrase recognition remains owned by grammar while semantic construction
//! stays in the front-end family layer.

pub mod early;
pub mod late;
pub mod mid;
pub mod type_and_color;
