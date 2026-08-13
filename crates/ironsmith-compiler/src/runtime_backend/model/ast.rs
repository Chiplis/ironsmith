//! Temporary source-compatibility facade for the compiler-owned AST.
//!
//! New code imports `crate::model::ast`; this module is removed with
//! `BRIDGE-CANONICAL-MODEL-REEXPORT` in PR-33.

pub(crate) use crate::model::ast::*;
