//! Typed grammar facts for static-keyword sentence families.
//!
//! The submodules follow the source-family clusters during migration so that
//! phrase recognition remains owned by grammar while semantic construction
//! stays in the front-end family layer.

pub(crate) mod early;
pub(crate) mod late;
pub(crate) mod mid;
pub(crate) mod type_and_color;
