#![recursion_limit = "256"]
#![expect(clippy::type_complexity, clippy::too_many_arguments)]
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]
// Grammar facts are assembled clause-by-clause and intentionally retain complete semantic
// subtrees. These implementation shapes favor auditable recognition over allocation or builder
// churn that would not change the canonical AST.
#![allow(
    clippy::clone_on_copy,
    clippy::collapsible_match,
    clippy::drop_non_drop,
    clippy::enum_variant_names,
    clippy::field_reassign_with_default,
    clippy::large_enum_variant,
    clippy::len_without_is_empty,
    clippy::let_and_return,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::redundant_closure_call,
    clippy::result_large_err,
    clippy::result_unit_err
)]

//! Oracle grammar implementation layer.
//!
//! This crate owns the parser grammar and its local recognition helpers.  The
//! public `ironsmith-compiler` package remains the document/lowering facade.

pub mod card_document;

pub mod model {
    pub use crate::card_document::{ParsedCardAst, ParsedCleaveBranch, ParsedOverloadBranch};
    pub use ironsmith_compiler_semantic::model::{
        ActivationRestrictionNormalizationFact, AdditionalCostChoiceOptionAst, AnnotatedEffect,
        AnnotatedEffectSequence, Cardinality, CarriedFactAst, CarryKindAst, CastingConditionAst,
        CharacteristicChangeAst, CharacteristicValueAst, ClashOpponentAst, ClauseActorAst,
        ClauseVerbAst, CompilerAbility, CompilerAbilityCore, CompilerAbilityKind,
        CompilerAbilityKindCore, CompilerAbilityPayload, CompilerActivatedAbility,
        CompilerActivatedAbilityAst, CompilerActivatedAbilityCore, CompilerActivationLegalityAst,
        CompilerAlternativeCastingMethod, CompilerAttachedAbilityGrantCore,
        CompilerCastingLegalityAst, CompilerClassAbilityAst, CompilerClassLevelAst,
        CompilerControlFlowAst, CompilerCost, CompilerDocument, CompilerDocumentItem,
        CompilerDurationAst, CompilerEnterAsCopyAsEntersSpecCore, CompilerGrantAbilityCore,
        CompilerGrantObjectAbilityForFilterCore, CompilerGrantSpecCore, CompilerGrantableCore,
        CompilerGrantedAbilityAst, CompilerKeywordAbilityAst, CompilerKeywordIdentityAst,
        CompilerKeywordPayloadAst, CompilerLevelAbilityAst, CompilerLevelBandAst,
        CompilerManaUsageRestriction, CompilerModalAbilityAst, CompilerModalModeAst,
        CompilerModalSelectionAst, CompilerOptionalCost, CompilerPermissionAst,
        CompilerPowerToughnessChoiceOptionCore, CompilerSagaAbilityAst, CompilerSagaChapterAst,
        CompilerSelectionAst, CompilerStaticAbilityAst, CompilerStaticAbilityCore,
        CompilerStaticAbilityPayloadCore, CompilerStructuredAbilityAst, CompilerTotalCost,
        CompilerTriggerEventAst, CompilerTriggerLegalityAst, CompilerTriggeredAbility,
        CompilerTriggeredAbilityAst, CompilerTriggeredAbilityCore, ConditionPositionAst,
        ContinuousLayerAst, ControlConditionAst, ControlDurationAst, ControlFlowError,
        ControlFlowNodeAst, ControlFlowReferenceEnvironmentAst, ControlFlowScopeAst,
        ControlFlowSemanticAst, ControlPredicateAst, CoordinationAst, CoordinationBoundaryAst,
        CoordinationCarryAst, CoordinationError, CoordinationKindAst, CoordinationMemberAst,
        CoordinationOperatorAst, CostRelationship, DamageBySpec, DashStyle, DelayedScheduleAst,
        EffectDependencyAst, EffectOrderingAst, ExchangeValueAst, ExchangeValueKindAst,
        ExtraTurnAnchorAst, FutureZoneReplacementCausePolicyAst, GiftTimingAst, IfResultPredicate,
        LegalityFrequencyAst, LegalityPeriodAst, LegalityRelationshipAst, LevelBandAst,
        LibraryBottomOrderAst, LibraryConsultModeAst, LibraryConsultStopRuleAst, LineAst,
        LinkedTriggerEffectAst, LoweredEffects, ManaUseConstraintAst, ModalSelectionModifierAst,
        NestedProgramAst, NestedProgramKindAst, ObjectDomain, ObjectRefAst, ParsedAbility,
        ParsedActivationRestriction, ParsedAlternativeCastingMethodAst, ParsedCardItem,
        ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLevelActivatedAbilityAst,
        ParsedLineAst, ParsedManaRestriction, ParsedModalActivatedHeader, ParsedModalAst,
        ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedOptionalCostAst,
        ParsedRestrictions, ParsedTriggerRestriction, PermissionKindAst, PermissionRelationshipAst,
        PhaseStepAst, PlayerAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
        PreventionRelationshipAst, ProvenanceId, ProvenanceRecord, ProvenanceStore, ProvenanceView,
        Provenanced, PunctuationKind, QuoteStyle, RedirectNextTimeDamageDestinationAst, RefState,
        ReferenceEnv, ReferenceExports, ReferenceFrame, ReferenceImports, ReferenceQuery,
        ReferenceRole, ReminderTextDecision, RenderingHint, ReplacedEventAst, ReplacementKindAst,
        ReplacementRelationshipAst, RestrictionBucket, RetargetModeAst, ReturnControllerAst,
        SearchLibrarySlotAst, SemanticProvenance, SharedTypeConstraintAst, SourcePosition,
        SourceSliceKind, SourceSpan, SourceUnit, SourceUnitId, StaticOperationAst,
        StaticRestrictionAst, StaticScopeAst, StaticSubjectAst, SymbolBinding, SymbolId,
        SymbolReference, SymbolResolutionError, SymbolScope, SymbolScopeId, SymbolScopeKind,
        SymbolTable, TargetAst, TimingWindowAst, TriggerBindingsAst, TriggerFrequencyAst,
        TriggerKindAst, TriggerReferenceAst, TriggerReferenceSurfaceAst, TriggerSubjectAst,
        TriggerZoneTransitionAst, TurnOwnerAst, ZoneReplacementDurationAst, activated_abilities,
        ast, canonical_references, clauses, compiler_semantic, control_flow, coordination, costs,
        document_program, facts, interaction_clauses, legality, library_clauses,
        object_action_clauses, parse_types, permission_clauses, provenance, reference,
        reference_state, resource_choice_clauses, restrictions, selections, static_abilities,
        structured_abilities, symbols, token_definition, triggered_abilities, visit,
    };
}
pub use ironsmith_compiler_semantic::model::{
    canonical_references, compiler_semantic, provenance, symbols,
};
pub use ironsmith_compiler_semantic::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, IfResultPredicate, KeywordAction, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst, ability, alternative_cast, card, color, continuous, cost, costs,
    effect, effects, events, filter, game_state, grant, ids, mana, model_impl, object,
    parse_context, payload, resolution, static_abilities, tag, target, triggers, types, zone,
};

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub mod front_end {
    pub use crate::front_end_parser_support::*;
    pub use ironsmith_compiler_source::{
        CstFace, CstLine, CstLineKind, CstNode, CstNodeKind, DashStyle, DocumentCst, LineInfo,
        MetadataLine, ModeMarker, NormalizedLine, NormalizedSourceMap, NormalizedSourceSegment,
        ProvenanceId, ProvenanceRecord, ProvenanceStore, ProvenanceView, Provenanced,
        PunctuationKind, QuoteStyle, ReminderTextDecision, RenderingHint, SelfReferenceSurface,
        SemanticProvenance, SourcePosition, SourceSliceKind, SourceSpan, SourceUnit, SourceUnitId,
        make_line_info, normalize_trimmed_line, parse_document_cst, parse_metadata_line,
    };
    pub use ironsmith_compiler_syntax::{
        CommonSentenceHead, LeadingMayActionMatch, LeadingMayActor, LexCursor, LexStream, LexToken,
        LexerError, OwnedLexToken, TokenKind, TokenWordPiece, TokenWordView, TurnDurationPhrase,
        clone_sentence_chunk_tokens, contains_sequence, contains_token_any_word,
        contains_token_kind, contains_token_word, contains_token_word_sequence, contains_window,
        find_any_token_word_sequence_span, find_token_any_word, find_token_kind, find_token_word,
        find_token_word_sequence, find_token_word_sequence_span, find_token_word_sequence_value,
        find_window_by, is_authored_proper_name_phrase, is_bare_card_name_phrase, iter_contains,
        iterators_equal, lex_line, lexed_head_words, lexed_tokens_contain_non_prefix_instead,
        lexer, parse_common_sentence_head, parse_leading_may_action_lexed,
        parse_turn_duration_prefix, parse_turn_duration_suffix, parser_token_word_positions,
        parser_token_word_refs, parses_any_word_view_prefix, parses_word_view_prefix,
        remove_copy_exception_type_removal_lexed, render_bare_card_name_surface,
        render_token_slice, rewrite_followup_intro_to_if_lexed, rfind_token_word,
        select_last_position, select_position, select_sequence_position, slice_contains,
        slice_contains_all, slice_contains_any, slice_ends_with, slice_ends_with_any, slice_eq_any,
        slice_primitives, slice_starts_with, slice_starts_with_any, slice_strip_any_prefix,
        slice_strip_any_suffix, slice_strip_prefix, slice_strip_suffix, split_em_dash_label_prefix,
        split_em_dash_label_prefix_tokens, split_lexed_once_on_comma,
        split_lexed_once_on_comma_then, split_lexed_once_on_delimiter, split_lexed_once_on_period,
        split_lexed_sentences, str_ends_with_any_char, str_find_char, str_rfind, str_rfind_char,
        str_split_once, str_strip_prefix, str_strip_suffix, string_primitives,
        strip_leading_if_you_do_lexed, synthetic_word_tokens, token_slice_at_is,
        token_slice_at_is_any, token_slice_first_is, token_slice_first_is_any, token_utils,
        token_word_pieces_for_token, token_word_refs, trim_lexed_commas, word_primitives,
        word_slice_at_is, word_slice_at_is_any, word_slice_contains_all_words,
        word_slice_contains_any_phrase_or_empty, word_slice_contains_any_word,
        word_slice_contains_no_words, word_slice_contains_phrase_or_empty,
        word_slice_contains_window_by, word_slice_contains_word, word_slice_ends_with_any,
        word_slice_find_any_phrase_start, word_slice_find_any_phrase_start_or_zero,
        word_slice_find_phrase_start_or_zero, word_slice_find_phrase_value,
        word_slice_find_window_by, word_slice_first_is, word_slice_first_is_any,
        word_slice_last_is, word_slice_last_is_any, word_slice_matching_phrase,
        word_slice_matching_value, word_slice_starts_with_any, word_slice_starts_with_at,
        word_slice_strip_any_prefix, word_slice_strip_any_suffix, word_slice_strip_first_word,
        word_slice_strip_first_word_value, word_slice_strip_prefix, word_slice_strip_prefix_value,
        word_slice_strip_suffix, word_slice_strip_suffix_value,
    };

    pub mod grammar {
        pub use crate::grammar::*;
    }
}
pub use front_end::*;

pub mod lexer {
    pub use ironsmith_compiler_syntax::lexer::{
        LexCursor, LexStream, LexToken, LexerError, OwnedLexToken, TokenKind, TokenWordPiece,
        TokenWordView, contains_token_any_word, contains_token_kind, contains_token_word,
        contains_token_word_sequence, find_any_token_word_sequence_span, find_token_any_word,
        find_token_kind, find_token_word, find_token_word_sequence, find_token_word_sequence_span,
        find_token_word_sequence_value, is_authored_proper_name_phrase, is_bare_card_name_phrase,
        lex_line, parser_token_word_positions, parser_token_word_refs,
        render_bare_card_name_surface, render_token_slice, rfind_token_word, split_lexed_sentences,
        synthetic_word_tokens, token_slice_at_is, token_slice_at_is_any, token_slice_first_is,
        token_slice_first_is_any, token_word_pieces_for_token, token_word_refs, trim_lexed_commas,
        word_slice_at_is, word_slice_at_is_any, word_slice_contains_all_words,
        word_slice_contains_any_phrase_or_empty, word_slice_contains_any_word,
        word_slice_contains_no_words, word_slice_contains_phrase_or_empty,
        word_slice_contains_window_by, word_slice_contains_word, word_slice_ends_with_any,
        word_slice_find_any_phrase_start, word_slice_find_any_phrase_start_or_zero,
        word_slice_find_phrase_start_or_zero, word_slice_find_phrase_value,
        word_slice_find_window_by, word_slice_first_is, word_slice_first_is_any,
        word_slice_last_is, word_slice_last_is_any, word_slice_matching_phrase,
        word_slice_matching_value, word_slice_starts_with_any, word_slice_starts_with_at,
        word_slice_strip_any_prefix, word_slice_strip_any_suffix, word_slice_strip_first_word,
        word_slice_strip_first_word_value, word_slice_strip_prefix, word_slice_strip_prefix_value,
        word_slice_strip_suffix, word_slice_strip_suffix_value,
    };
    pub use ironsmith_grammar_common::lexical::{
        LexedClause, locate_token_kind, locate_token_word, locate_token_word_choice,
        token_slice_all_are_kind, token_slice_last_is,
    };
}

pub use ironsmith_compiler_resolve::effect_ast_normalization;
pub use ironsmith_compiler_resolve::effect_ast_traversal;
pub use ironsmith_compiler_resolve::reference_helpers;
pub use ironsmith_compiler_resolve::reference_resolution;
pub use ironsmith_grammar_common::recognition;

pub use alternative_cast::TrapCondition;
pub use card::{PowerToughness, PtValue};
pub use color::{Color, ColorSet};
pub use cost::{OptionalCost, TotalCost};
pub use diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use effect::{ChoiceCount, DelayedTriggerSpec, Effect, EffectId, Until, Value};
pub use ids::{CardId, ObjectId, PlayerId, StableId};
pub use ironsmith_core::{
    AttachmentConditionHost, Condition as ConditionExpr, PermanentLeftBattlefieldControlSurface,
    SourceCounterThresholdSurface, WorkspaceSplitMarker,
};
pub use object::{AuraAttachmentFilter, CounterType};
pub use tag::TagKey;
pub use target::{
    ChooseSpec, ObjectCharacteristic, ObjectCharacteristicRelation,
    ObjectCharacteristicRelationKind, ObjectFilter, ObjectRef, PlayerFilter,
    TaggedObjectConstraint, TaggedOpbjectRelation,
};
pub use types::{CardType, Subtype, Supertype};
pub use zone::Zone;

pub use ironsmith_compiler_syntax::{slice_primitives, string_primitives, word_primitives};
pub mod token_primitives;

pub mod facade;
pub mod grammar;
pub mod ir;
pub mod oracle_grammar;
pub mod parse_trace;
pub mod pipeline;
pub mod registry;
pub mod rule_engine;

pub mod effect_sentences;

pub mod activation_and_restrictions;
pub mod activation_helpers;
pub mod clause_support;
pub mod keyword_families;
pub mod keyword_payloads;
pub mod keyword_registry;
pub mod keyword_static;
pub mod keyword_static_helpers;
pub mod line_info;
pub mod modal_helpers;
pub mod object_filters;
pub mod permission_helpers;
pub mod restriction_support;
#[path = "lowering_impl/runtime_static_ability_helpers.rs"]
pub mod runtime_static_ability_helpers;
pub mod search_library_support;
pub mod static_ability_helpers;

pub mod front_end_parser_support;
pub mod parser_support;
pub mod preprocess;
pub mod recognized_document;
pub mod semantic_assembly;
pub mod semantic_line_parsing;
pub mod util;

#[path = "lowering_impl/battlefield_entry_counter_fusion.rs"]
pub mod battlefield_entry_counter_fusion;
pub mod canonical_pipeline;
pub mod card_builders;
#[path = "lowering_impl/compile_support.rs"]
pub mod compile_support;
#[path = "lowering_impl/pipeline.rs"]
pub mod compiler_pipeline;
#[path = "lowering_impl/condition_antecedent.rs"]
pub mod condition_antecedent;
pub mod document_parser;
#[path = "lowering_impl/effect_pipeline.rs"]
pub mod effect_pipeline;
#[path = "lowering_impl/lower/mod.rs"]
pub mod lower;
#[path = "lowering_impl/mod.rs"]
pub mod lowering;
#[path = "lowering_impl/lowering_support.rs"]
pub mod lowering_support;
#[path = "grammar/modal_support.rs"]
pub mod modal_support;
pub mod parse_loss;
pub mod semantic_document;

pub mod cards {
    pub use crate::card_builders::CardDefinitionBuilder;
    pub use ironsmith_compiler_semantic::cards::{CardDefinition, ParseAnnotations, TextSpan};

    pub mod builders {
        pub use crate::card_builders::*;
    }

    pub mod tokens {
        pub use crate::card_tokens::*;
    }
}

pub use card_builders::CardDefinitionBuilder;
pub use ironsmith_compiler_semantic::cards::CardDefinition;
pub use line_info::LineInfo;

pub mod card_tokens;

pub mod host {
    pub use crate::cards::builders::{
        CardTextError, EffectAst, OwnedLexToken, PlayerAst, PredicateAst, SubjectAst, TagKey,
        TargetAst, TriggerSpec,
    };

    pub const IT_TAG: &str = "__it__";
    pub const ADDITIONAL_COST_OBJECT_TAG: &str = "__additional_cost_object__";
    pub const CHOSEN_OBJECTS_TAG: &str = ironsmith_core::CHOSEN_OBJECTS_TAG;
    pub const COPIED_STACK_OBJECT_TAG: &str = "__copied_stack_object__";
    pub const THIS_WAY_SACRIFICED_TAG: &str = "__this_way_sacrificed__";
}

pub fn parse_context_for_builder(
    builder: &CardDefinitionBuilder,
    text: &str,
    allow_unsupported: bool,
) -> ironsmith_compiler_ast::ParseContext {
    use ironsmith_compiler_ast::{
        CardFaceMetadata, ParseContext, ParseFeatures, ProvenanceStore, SourceIdentity,
        SourceUnitId,
    };
    let card_name = builder.card_builder.name_ref().trim().to_string();
    let mut context = ParseContext::new(
        SourceIdentity {
            unit: SourceUnitId(0),
            card_name: card_name.clone(),
            face_index: 0,
            source_len: text.len(),
            source_line_count: text.lines().count(),
        },
        CardFaceMetadata {
            supertypes: builder.card_builder.supertypes_ref().to_vec(),
            card_types: builder.card_builder.card_types_ref().to_vec(),
            subtypes: builder.card_builder.subtypes_ref().to_vec(),
            other_face_name: None,
        },
        ParseFeatures {
            allow_unsupported,
            preserve_reminder_text: false,
            capture_trace: parse_trace::is_enabled(),
        },
    );
    context.replace_provenance(ProvenanceStore::capture(SourceUnitId(0), text, &card_name));
    context
}

pub fn compile_card_text_with_policy(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<facade::CompiledCardText<CardDefinition>, CardTextError> {
    let text = text.into();
    util::with_cached_parser_trace(move || {
        let mut context = parse_context_for_builder(&builder, &text, allow_unsupported);
        compiler_pipeline::parse_text_with_annotations_lowered_with_facts_context(
            &mut context,
            builder,
            text,
        )
        .map(|lowered| facade::CompiledCardText {
            definition: lowered.definition,
            annotations: lowered.annotations,
        })
    })
}

pub fn compile_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<facade::CompiledCardText<CardDefinition>, CardTextError> {
    compile_card_text_with_policy(builder, text, allow_unsupported)
}

pub fn parse_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    compile_card_text(builder, text, false).map(|compiled| compiled.definition)
}

pub fn parse_card_text_allow_unsupported(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    compile_card_text(builder, text, true).map(|compiled| compiled.definition)
}

pub use facade::{
    CompilePolicy, CompiledCardText, CompilerBackend, CompilerCompileRequest, CompilerFacade,
    CompilerSourceDocument,
};
pub use oracle_grammar::{
    OracleGrammarDocument, OracleGrammarLevelItem, OracleGrammarLine, OracleGrammarLineInfo,
    OracleGrammarMode, parse_oracle_grammar_document,
};
pub use pipeline::{LoweringPipeline, PostpassProcessor};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
