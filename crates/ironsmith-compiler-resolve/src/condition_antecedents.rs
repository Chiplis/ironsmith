//! Antecedent binding, now owned by the AST.
//!
//! Binding a condition's antecedent into a filter reads and writes only the
//! filter, so it belongs beside the vocabulary rather than in resolution.

pub use ironsmith_compiler_semantic::condition_antecedent::{
    bind_condition_filter_antecedent, merge_filter_overlay,
};
