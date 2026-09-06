//! Compiler provenance, symbols, and grammar-neutral leaf AST vocabulary.

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub mod effect {
    pub use ironsmith_core::Value;
}

pub mod target {
    pub use ironsmith_core::{ObjectFilter, PlayerFilter};
}

pub mod types {
    pub use ironsmith_core::{CardType, Subtype, Supertype};
}

pub mod provenance {
    pub use ironsmith_compiler_source::provenance::{
        DashStyle, ProvenanceId, ProvenanceRecord, ProvenanceStore, ProvenanceView, Provenanced,
        PunctuationKind, QuoteStyle, ReminderTextDecision, RenderingHint, SemanticProvenance,
        SourcePosition, SourceSliceKind, SourceSpan, SourceUnit, SourceUnitId,
    };
}

pub mod model {
    pub use crate::provenance;
    pub use crate::symbols;
}

pub mod parse_context;
pub mod parse_types;
pub mod restrictions;
pub mod symbols;
pub mod reference_ledger;
pub mod tag_ref;
pub use tag_ref::TagRef;

pub use parse_context::*;
pub use parse_types::*;
pub use provenance::{
    DashStyle, ProvenanceId, ProvenanceRecord, ProvenanceStore, ProvenanceView, Provenanced,
    PunctuationKind, QuoteStyle, ReminderTextDecision, RenderingHint, SemanticProvenance,
    SourcePosition, SourceSliceKind, SourceSpan, SourceUnit, SourceUnitId,
};
pub use restrictions::*;
pub use symbols::*;
