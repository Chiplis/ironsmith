use crate::cards::ParseAnnotations;
use crate::cards::builders::CardDefinitionBuilder;

use super::canonical_references::CanonicalReferenceResolutionAst;
use super::compiler_semantic::ParsedCardItem;
use super::provenance::ProvenanceStore;
use super::symbols::SymbolTable;

#[derive(Debug, Clone)]
pub struct ParsedOverloadBranch {
    pub items: Vec<ParsedCardItem>,
}

#[derive(Debug, Clone)]
pub struct ParsedCleaveBranch {
    pub items: Vec<ParsedCardItem>,
}

/// The sole card-level front-end result. It owns compiler semantic nodes,
/// diagnostics, provenance, and scoped symbols; runtime materialization is a
/// separate operation in the lowering phase.
#[derive(Debug, Clone)]
pub struct ParsedCardAst {
    pub builder: CardDefinitionBuilder,
    pub annotations: ParseAnnotations,
    pub provenance: ProvenanceStore,
    pub symbols: SymbolTable,
    pub reference_resolution: CanonicalReferenceResolutionAst,
    pub items: Vec<ParsedCardItem>,
    pub overload_branch: Option<ParsedOverloadBranch>,
    pub cleave_branch: Option<ParsedCleaveBranch>,
    pub allow_unsupported: bool,
}
