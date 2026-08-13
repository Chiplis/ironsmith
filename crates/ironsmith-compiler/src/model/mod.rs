pub(crate) mod ast;
pub(crate) mod compiler_semantic;
pub(crate) mod facts;
pub mod ir;
pub mod parse_types;
pub mod provenance;
pub mod reference;
pub(crate) mod reference_state;
pub mod restrictions;
pub mod semantic;
pub mod symbols;
pub(crate) mod token_definition;
pub(crate) mod visit;

pub use ast::{
    CompilerAbility, CompilerAbilityKind, CompilerAbilityPayload, CompilerActivatedAbility,
    CompilerDocument, CompilerDocumentItem, CompilerTriggeredAbility,
};

pub use ir::{
    RewriteActivatedLine, RewriteKeywordLine, RewriteLevelHeader, RewriteLevelItem,
    RewriteLevelItemKind, RewriteModalBlock, RewriteModalMode, RewriteSagaChapterLine,
    RewriteSemanticDocument, RewriteSemanticItem, RewriteStatementLine, RewriteStaticLine,
    RewriteTriggeredLine, RewriteUnsupportedLine,
};
pub use parse_types::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExchangeValueKindAst,
    ExtraTurnAnchorAst, FutureZoneReplacementCausePolicyAst, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
    RedirectNextTimeDamageDestinationAst, RetargetModeAst, ReturnControllerAst,
    SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst, ZoneReplacementDurationAst,
};
pub use provenance::{
    DashStyle, ProvenanceId, ProvenanceRecord, ProvenanceStore, ProvenanceView, Provenanced,
    PunctuationKind, QuoteStyle, ReminderTextDecision, RenderingHint, SemanticProvenance,
    SourcePosition, SourceSliceKind, SourceSpan, SourceUnit, SourceUnitId,
};
pub use reference::{
    AnnotatedEffect, AnnotatedEffectSequence, LoweredEffects, RefState, ReferenceEnv,
    ReferenceExports, ReferenceFrame, ReferenceImports,
};
pub use restrictions::{ParsedRestrictions, RestrictionBucket};
pub use semantic::{
    AdditionalCostChoiceOptionAst, GiftTimingAst, LineAst, ParsedAbility, ParsedCardItem,
    ParsedCardItemKind, ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst,
    ParsedModalActivatedHeader, ParsedModalAst, ParsedModalGate, ParsedModalHeader,
    ParsedModalModeAst,
};
pub use symbols::{
    Cardinality, ObjectDomain, ReferenceQuery, ReferenceRole, SymbolBinding, SymbolId,
    SymbolReference, SymbolResolutionError, SymbolScope, SymbolScopeId, SymbolScopeKind,
    SymbolTable,
};
