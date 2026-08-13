//! Document-level composition of already typed statements.
//!
//! Sentence parsers own statement semantics.  This layer records only source
//! order, authored connective strength, lexical scope, and reference flow
//! between those statements.

use crate::model::ast::EffectAst;
use crate::model::symbols::{SymbolReference, SymbolScopeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CompilerStatementId(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerStatementEdgeKindAst {
    Ordered,
    Then,
    Reference,
    Result,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerDocumentStatementAst {
    pub id: CompilerStatementId,
    pub scope: SymbolScopeId,
    pub parent_scope: SymbolScopeId,
    pub effects: Vec<EffectAst>,
    pub imports: Vec<SymbolReference>,
    pub exports: Vec<SymbolReference>,
    pub leading_then: bool,
    pub starting_with_controller: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerStatementEdgeAst {
    pub from: CompilerStatementId,
    pub to: CompilerStatementId,
    pub kind: CompilerStatementEdgeKindAst,
    pub references: Vec<SymbolReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerDocumentProgramAst {
    pub scope: SymbolScopeId,
    pub parent_scope: SymbolScopeId,
    pub statements: Vec<CompilerDocumentStatementAst>,
    pub edges: Vec<CompilerStatementEdgeAst>,
}
