use crate::cards::ParseAnnotations;
use crate::cards::builders::CardDefinitionBuilder;

use super::canonical_references::CanonicalReferenceResolutionAst;
use super::compiler_semantic::ParsedCardItem;
use super::provenance::ProvenanceStore;
use super::symbols::SymbolTable;

#[derive(Debug, Clone)]
pub(crate) struct ParsedOverloadBranch {
    pub(crate) items: Vec<ParsedCardItem>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedCleaveBranch {
    pub(crate) items: Vec<ParsedCardItem>,
}

/// The sole card-level front-end result. It owns compiler semantic nodes,
/// diagnostics, provenance, and scoped symbols; runtime materialization is a
/// separate operation in the lowering phase.
#[derive(Debug, Clone)]
pub(crate) struct ParsedCardAst {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) provenance: ProvenanceStore,
    pub(crate) symbols: SymbolTable,
    pub(crate) reference_resolution: CanonicalReferenceResolutionAst,
    pub(crate) items: Vec<ParsedCardItem>,
    pub(crate) overload_branch: Option<ParsedOverloadBranch>,
    pub(crate) cleave_branch: Option<ParsedCleaveBranch>,
    pub(crate) allow_unsupported: bool,
}
