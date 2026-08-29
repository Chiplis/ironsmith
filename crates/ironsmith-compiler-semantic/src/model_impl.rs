//! Canonical compiler AST and semantic facts.

pub mod activated_abilities;
pub mod ast;
pub mod canonical_references;
pub mod clauses;
pub mod compiler_semantic;
pub mod control_flow;
pub mod coordination;
pub mod costs;
pub mod document_program;
pub mod facts;
pub mod interaction_clauses;
pub mod legality;
pub mod library_clauses;
pub mod object_action_clauses;
pub mod permission_clauses;
pub mod reference;
pub mod reference_state;
pub mod resource_choice_clauses;
pub mod selections;
pub mod static_abilities;
pub mod structured_abilities;
pub mod token_definition;
pub mod triggered_abilities;
pub mod visit;

pub use ironsmith_compiler_ast::{parse_types, provenance, restrictions, symbols};

pub use crate::payload::IfResultPredicate;
pub use ast::{
    CompilerAbility, CompilerAbilityKind, CompilerAbilityPayload, CompilerActivatedAbility,
    CompilerDocument, CompilerDocumentItem, CompilerTriggeredAbility,
};
pub use compiler_semantic::{
    ActivationRestrictionNormalizationFact, AdditionalCostChoiceOptionAst, GiftTimingAst, LineAst,
    ParsedAbility, ParsedActivationRestriction, ParsedAlternativeCastingMethodAst, ParsedCardItem,
    ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLevelActivatedAbilityAst,
    ParsedLineAst, ParsedManaRestriction, ParsedModalActivatedHeader, ParsedModalAst,
    ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedOptionalCostAst,
    ParsedTriggerRestriction,
};
pub use costs::{
    CastingConditionAst, CompilerAlternativeCastingMethod, CompilerCost, CompilerOptionalCost,
    CompilerTotalCost, CostRelationship,
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
