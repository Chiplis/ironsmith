//! Typed complements for library-facing common clauses.
//!
//! Library grammar still uses the shared actor/action/object/value/destination
//! vocabulary. These facts describe the orthogonal zone position, exposure,
//! selection, result binding, and remainder relationships that ordinary
//! clauses do not need.

use crate::model::clauses::{ClauseActorAst, ClauseDestinationAst};
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::symbols::SymbolReference;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LibraryExposureAst {
    Inspect,
    Reveal,
    ExileFaceUp,
    ExileFaceDown,
    Mill,
    Search,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LibraryPositionAst {
    WholeZone,
    Top(CompilerValueAst),
    Bottom(CompilerValueAst),
    Random(CompilerValueAst),
    NthFromTop(CompilerValueAst),
    UntilMatch {
        qualification: CompilerFilterAst,
        match_count: CompilerValueAst,
        maximum_exposed: Option<CompilerValueAst>,
    },
    BoundCollection(SymbolReference),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LibrarySelectionModeAst {
    Exact,
    Optional,
    AllMatching,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LibrarySelectionAst {
    pub qualification: Option<CompilerFilterAst>,
    pub minimum: CompilerValueAst,
    pub maximum: Option<CompilerValueAst>,
    pub mode: LibrarySelectionModeAst,
    pub random: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LibraryResultKindAst {
    Exposed,
    Matched,
    Found,
    Milled,
    Chosen,
    Remainder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LibraryResultBindingAst {
    pub kind: LibraryResultKindAst,
    pub reference: SymbolReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LibraryOrderAst {
    Preserve,
    Random,
    Chosen,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LibraryRemainderAst {
    pub collection: SymbolReference,
    pub excluding: Vec<SymbolReference>,
    pub destination: ClauseDestinationAst,
    pub order: LibraryOrderAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerLibraryClauseAst {
    pub owner: ClauseActorAst,
    pub chooser: Option<ClauseActorAst>,
    pub source_zones: Vec<Zone>,
    pub position: LibraryPositionAst,
    pub exposure: LibraryExposureAst,
    pub selections: Vec<LibrarySelectionAst>,
    pub destination: Option<ClauseDestinationAst>,
    pub results: Vec<LibraryResultBindingAst>,
    pub remainder: Option<LibraryRemainderAst>,
    pub reveal_results: bool,
    pub shuffle_after: bool,
    pub tapped: bool,
    pub enters_under_source_controller: bool,
    pub any_order_surface: bool,
}
