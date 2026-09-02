#![recursion_limit = "256"]
#![expect(clippy::type_complexity, clippy::too_many_arguments)]
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]
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

//! Lowering: the compiler-owned card AST becomes a runtime card definition.
//!
//! Everything upstream of this crate recognizes and resolves text. Nothing here
//! reads Oracle text or re-parses it: the input is the typed AST the front end
//! produced, and the output is the definition the engine runs. The phase DAG
//! makes that one-way — this crate cannot see the grammar crate at all.

pub mod model {
    pub use ironsmith_compiler_semantic::card_document::{
        ParsedCardAst, ParsedCleaveBranch, ParsedOverloadBranch,
    };
    pub use ironsmith_compiler_semantic::model::{
        ActivationRestrictionNormalizationFact, AdditionalCostChoiceOptionAst, AnnotatedEffect,
        AnnotatedEffectSequence, Cardinality, CarriedFactAst, CarryKindAst, CastingConditionAst,
        CharacteristicChangeAst, CharacteristicValueAst, ClashOpponentAst, ClauseActorAst,
        ClauseVerbAst, CompilerAbility, CompilerAbilityCore, CompilerAbilityKind,
        CompilerAbilityKindCore, CompilerAbilityPayload, CompilerActivatedAbility,
        CompilerActivatedAbilityAst, CompilerActivatedAbilityCore, CompilerActivationLegalityAst,
        CompilerAlternativeCastingMethod, CompilerAnthem, CompilerAttachedAbilityGrantCore,
        CompilerCastingLegalityAst, CompilerClassAbilityAst, CompilerClassLevelAst,
        CompilerControlFlowAst, CompilerCost, CompilerCostIncrease, CompilerCostIncreaseManaCost,
        CompilerCostReduction, CompilerCostReductionManaCost, CompilerDocument,
        CompilerDocumentItem, CompilerDurationAst, CompilerEnterAsCopyAsEntersSpecCore,
        CompilerGrantAbilityCore, CompilerGrantObjectAbilityForFilterCore, CompilerGrantSpecCore,
        CompilerGrantableCore, CompilerGrantedAbilityAst, CompilerKeywordAbilityAst,
        CompilerKeywordIdentityAst, CompilerKeywordPayloadAst, CompilerLevelAbilityAst,
        CompilerLevelBandAst, CompilerManaUsageRestriction, CompilerModalAbilityAst,
        CompilerModalModeAst, CompilerModalSelectionAst, CompilerOptionalCost,
        CompilerPermissionAst, CompilerPowerToughnessChoiceOptionCore,
        CompilerRemoveCardTypesForFilter, CompilerSagaAbilityAst, CompilerSagaChapterAst,
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
pub use alternative_cast::TrapCondition;
pub use card::{PowerToughness, PtValue};
pub use color::{Color, ColorSet};
pub use cost::{OptionalCost, TotalCost};
pub use diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use effect::{ChoiceCount, DelayedTriggerSpec, Effect, EffectId, Until, Value};
pub use ids::{CardId, ObjectId, PlayerId, StableId};
pub use ironsmith_compiler_api::parse_loss;
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
pub use ironsmith_compiler_syntax::{slice_primitives, string_primitives, word_primitives};
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

pub use ironsmith_compiler_resolve::condition_antecedents;
pub use ironsmith_compiler_resolve::effect_ast_normalization;
pub use ironsmith_compiler_resolve::effect_ast_traversal;
pub use ironsmith_compiler_resolve::predicate_conditions as reference_resolution_support;
pub use ironsmith_compiler_resolve::reference_helpers;
pub use ironsmith_compiler_resolve::reference_resolution;
pub use ironsmith_compiler_resolve::tag_support;
pub use ironsmith_compiler_resolve::trigger_players;

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub mod front_end {
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

    pub mod grammar {}
}

pub mod card_builders;
pub mod card_tokens;
pub mod keyword_actions;

#[path = "lowering_impl/battlefield_entry_counter_fusion.rs"]
pub mod battlefield_entry_counter_fusion;
#[path = "lowering_impl/compile_support.rs"]
pub mod compile_support;
#[path = "lowering_impl/condition_antecedent.rs"]
pub mod condition_antecedent;
#[path = "lowering_impl/effect_pipeline.rs"]
pub mod effect_pipeline;
#[path = "lowering_impl/lower/mod.rs"]
pub mod lower;
#[path = "lowering_impl/mod.rs"]
pub mod lowering;
#[path = "lowering_impl/lowering_support.rs"]
pub mod lowering_support;
#[path = "lowering_impl/runtime_static_ability_helpers.rs"]
pub mod runtime_static_ability_helpers;

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
