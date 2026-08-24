//! Canonical compiler AST and semantic facts.

#[path = "../../ironsmith-compiler/src/model/activated_abilities.rs"]
pub mod activated_abilities;
#[path = "../../ironsmith-compiler/src/model/ast.rs"]
pub mod ast;
#[path = "../../ironsmith-compiler/src/model/canonical_references.rs"]
pub mod canonical_references;
#[path = "../../ironsmith-compiler/src/model/clauses.rs"]
pub mod clauses;
#[path = "../../ironsmith-compiler/src/model/compiler_semantic.rs"]
pub mod compiler_semantic;
#[path = "../../ironsmith-compiler/src/model/control_flow.rs"]
pub mod control_flow;
#[path = "../../ironsmith-compiler/src/model/coordination.rs"]
pub mod coordination;
#[path = "../../ironsmith-compiler/src/model/costs.rs"]
pub mod costs;
#[path = "../../ironsmith-compiler/src/model/document_program.rs"]
pub mod document_program;
#[path = "../../ironsmith-compiler/src/model/facts.rs"]
pub mod facts;
#[path = "../../ironsmith-compiler/src/model/interaction_clauses.rs"]
pub mod interaction_clauses;
#[path = "../../ironsmith-compiler/src/model/ir.rs"]
pub mod ir;
#[path = "../../ironsmith-compiler/src/model/legality.rs"]
pub mod legality;
#[path = "../../ironsmith-compiler/src/model/library_clauses.rs"]
pub mod library_clauses;
#[path = "../../ironsmith-compiler/src/model/object_action_clauses.rs"]
pub mod object_action_clauses;
#[path = "../../ironsmith-compiler/src/model/permission_clauses.rs"]
pub mod permission_clauses;
#[path = "../../ironsmith-compiler/src/model/reference.rs"]
pub mod reference;
#[path = "../../ironsmith-compiler/src/model/reference_state.rs"]
pub mod reference_state;
#[path = "../../ironsmith-compiler/src/model/resource_choice_clauses.rs"]
pub mod resource_choice_clauses;
#[path = "../../ironsmith-compiler/src/model/selections.rs"]
pub mod selections;
#[path = "../../ironsmith-compiler/src/model/semantic.rs"]
pub mod semantic;
#[path = "../../ironsmith-compiler/src/model/static_abilities.rs"]
pub mod static_abilities;
#[path = "../../ironsmith-compiler/src/model/structured_abilities.rs"]
pub mod structured_abilities;
#[path = "../../ironsmith-compiler/src/model/token_definition.rs"]
pub mod token_definition;
#[path = "../../ironsmith-compiler/src/model/triggered_abilities.rs"]
pub mod triggered_abilities;
#[path = "../../ironsmith-compiler/src/model/visit.rs"]
pub mod visit;

pub use ironsmith_compiler_ast::{parse_types, provenance, restrictions, symbols};

pub use crate::payload::IfResultPredicate;
pub use ast::{
    CompilerAbility, CompilerAbilityKind, CompilerAbilityPayload, CompilerActivatedAbility,
    CompilerDocument, CompilerDocumentItem, CompilerTriggeredAbility,
};
pub use costs::{
    CastingConditionAst, CompilerAlternativeCastingMethod, CompilerCost, CompilerOptionalCost,
    CompilerTotalCost, CostRelationship,
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

pub use activated_abilities::CompilerActivatedAbilityAst;
pub use clauses::{ClauseActorAst, ClauseVerbAst};
pub use compiler_semantic::{
    CompilerAbilityCore, CompilerAbilityKindCore, CompilerActivatedAbilityCore,
    CompilerManaUsageRestriction, CompilerTriggeredAbilityCore,
};
pub use control_flow::*;
pub use coordination::*;
pub use legality::*;
pub use selections::CompilerSelectionAst;
pub use static_abilities::*;
pub use structured_abilities::*;
pub use triggered_abilities::*;
