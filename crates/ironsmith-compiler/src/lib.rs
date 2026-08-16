#![expect(
    dead_code,
    reason = "the canonical compiler AST intentionally models grammar vocabulary beyond the currently exercised corpus"
)]
#![expect(
    clippy::large_enum_variant,
    reason = "compiler AST nodes remain value-semantic until the single lowering boundary"
)]
#![expect(
    clippy::type_complexity,
    reason = "typed grammar recognizers expose complete compositional match facts"
)]
#![expect(
    clippy::too_many_arguments,
    reason = "parser boundaries carry explicit context, provenance, scope, and authored token slices"
)]
#![expect(
    clippy::field_reassign_with_default,
    reason = "grammar filters are assembled incrementally from independently recognized clauses"
)]
#![expect(
    clippy::enum_variant_names,
    reason = "grammar fact enums repeat their semantic family name to stay unambiguous at use sites"
)]
#![expect(
    clippy::wrong_self_convention,
    reason = "recognizer method names describe authored-token provenance rather than conversion ownership"
)]
#![expect(
    clippy::result_large_err,
    reason = "structured parse diagnostics retain committed spans, rule paths, and source context"
)]
#![expect(
    clippy::vec_box,
    reason = "recursive nested-ability nodes use stable indirection at each child boundary"
)]

//! Compiler crate for parser/front-end ownership in the split workspace.
//!
//! The compiler crate owns oracle-text recognition, canonical compiler ASTs,
//! explicit reference resolution, and the single lowering boundary consumed by
//! runtime crates.

pub mod ability;
pub mod alternative_cast;
pub mod card;
pub mod cards;
pub mod color;
pub mod continuous;
pub mod cost;
pub mod costs;
pub mod diagnostics;
pub mod effect;
pub mod effects;
pub mod events;
pub mod facade;
pub mod filter;
pub mod front_end;
pub mod game_state;
pub mod grant;
pub mod host;
pub mod ids;
pub(crate) mod lowering;
pub mod mana;
pub mod model;
pub mod object;
pub mod oracle_grammar;
pub mod parse_context;
pub mod parse_loss;
pub mod parse_trace;
pub mod payload;
pub mod pipeline;
pub mod recognition;
pub mod registry;
pub mod resolution;
mod slice_primitives;
pub mod static_abilities;
mod string_primitives;
pub mod tag;
pub mod target;
pub mod triggers;
pub mod types;
mod word_primitives;
pub mod zone;

#[path = "front_end/grammar/ability_rules/activation_and_restrictions/mod.rs"]
pub(crate) mod activation_and_restrictions;
#[path = "front_end/grammar/ability_rules/activation_helpers.rs"]
pub(crate) mod activation_helpers;
#[path = "lowering/battlefield_entry_counter_fusion.rs"]
pub(crate) mod battlefield_entry_counter_fusion;
#[path = "front_end/canonical_pipeline.rs"]
pub(crate) mod canonical_pipeline;
#[path = "front_end/grammar/ability_rules/clause_support.rs"]
pub(crate) mod clause_support;
#[path = "lowering/compile_support.rs"]
pub(crate) mod compile_support;
#[path = "lowering/condition_antecedent.rs"]
pub(crate) mod condition_antecedent;
#[path = "front_end/cst.rs"]
pub(crate) mod cst;
#[path = "front_end/cst_lowering.rs"]
pub(crate) mod cst_lowering;
#[path = "front_end/document/mod.rs"]
pub(crate) mod document_parser;
#[path = "model/effect_ast_normalization.rs"]
pub(crate) mod effect_ast_normalization;
#[path = "lowering/effect_pipeline.rs"]
pub(crate) mod effect_pipeline;
pub(crate) use model::visit as effect_ast_traversal;
#[path = "lowering/pipeline.rs"]
pub(crate) mod compiler_pipeline;
#[path = "front_end/grammar/effect_clauses/effect_sentences/mod.rs"]
pub(crate) mod effect_sentences;
#[path = "front_end/grammar/mod.rs"]
pub(crate) mod grammar;
#[path = "model/semantic_document.rs"]
pub(crate) mod ir;
#[path = "front_end/grammar/ability_rules/keyword_families.rs"]
pub(crate) mod keyword_families;
#[path = "front_end/grammar/ability_rules/keyword_payloads.rs"]
pub(crate) mod keyword_payloads;
#[path = "front_end/grammar/ability_rules/keyword_registry.rs"]
pub(crate) mod keyword_registry;
#[path = "front_end/grammar/ability_rules/keyword_static/mod.rs"]
pub(crate) mod keyword_static;
#[path = "front_end/grammar/ability_rules/keyword_static_helpers.rs"]
pub(crate) mod keyword_static_helpers;
#[path = "lowering/lower/mod.rs"]
pub(crate) mod lower;
#[path = "lowering/lowering_support.rs"]
pub(crate) mod lowering_support;
#[path = "front_end/grammar/ability_rules/modal_helpers.rs"]
pub(crate) mod modal_helpers;
#[path = "model/modal_support.rs"]
pub(crate) mod modal_support;
#[path = "front_end/grammar/ability_rules/object_filters.rs"]
pub(crate) mod object_filters;
#[path = "front_end/semantic_parser_support.rs"]
pub(crate) mod parser_support;
#[path = "front_end/grammar/ability_rules/permission_helpers.rs"]
pub(crate) mod permission_helpers;
#[path = "front_end/semantic_preprocess.rs"]
pub(crate) mod preprocess;
#[path = "model/reference_helpers.rs"]
pub(crate) mod reference_helpers;
#[path = "model/reference_resolution.rs"]
pub(crate) mod reference_resolution;
#[path = "front_end/grammar/ability_rules/restriction_support.rs"]
pub(crate) mod restriction_support;
#[path = "front_end/rule_engine.rs"]
pub(crate) mod rule_engine;
#[path = "front_end/grammar/effect_clauses/search_library_support.rs"]
pub(crate) mod search_library_support;
#[path = "front_end/semantic_document.rs"]
pub(crate) mod semantic_document;
#[path = "front_end/semantic_line_parsing/mod.rs"]
pub(crate) mod semantic_line_parsing;
#[path = "front_end/grammar/ability_rules/static_ability_helpers.rs"]
pub(crate) mod static_ability_helpers;
#[path = "front_end/token_primitives.rs"]
pub(crate) mod token_primitives;
#[path = "front_end/shared/util.rs"]
pub(crate) mod util;

pub(crate) use front_end::lexer;

pub(crate) fn compile_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<facade::CompiledCardText<CardDefinition>, CardTextError> {
    let text = text.into();
    let mut builder = builder;
    for raw_line in text.lines() {
        let Some(MetadataLine::TypeLine(raw_type_line)) = parse_metadata_line(raw_line)? else {
            continue;
        };
        builder = builder.apply_metadata(MetadataLine::TypeLine(raw_type_line))?;
    }
    let mut context = ParseContext::for_builder(&builder, &text, allow_unsupported);
    compiler_pipeline::parse_text_with_annotations_lowered_with_facts_context(
        &mut context,
        builder,
        text,
    )
    .map(|lowered| facade::CompiledCardText {
        definition: lowered.definition,
        annotations: lowered.annotations,
    })
}

pub(crate) fn parse_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    compile_card_text(builder, text, false).map(|compiled| compiled.definition)
}

pub(crate) fn parse_card_text_allow_unsupported(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    compile_card_text(builder, text, true).map(|compiled| compiled.definition)
}

#[cfg(test)]
mod tests;

pub use alternative_cast::TrapCondition;
pub use card::PowerToughness;
pub use card::PtValue;
pub use cards::{CardDefinition, CardDefinitionBuilder};
pub use color::{Color, ColorSet};
pub use cost::{OptionalCost, TotalCost};
pub use diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use effect::{ChoiceCount, DelayedTriggerSpec, Effect, EffectId, Until, Value};
pub use facade::{
    CompilePolicy, CompiledCardText, CompilerBackend, CompilerCompileRequest, CompilerFacade,
    CompilerSourceDocument,
};
pub use front_end::{
    ActivatedLineCst, ClassifiedFace, ClassifiedLine, CommonSentenceHead, DocumentStructure,
    KeywordLineCst, KeywordLineKindCst, LeadingMayActionMatch, LeadingMayActor, LevelHeaderCst,
    LevelItemCst, LevelItemKindCst, LexCursor, LexStream, LexToken, LexerError, LineInfo,
    MetadataLine, MetadataLineCst, ModalBlockCst, ModalModeCst, ModeMarker, NormalizedLine,
    NormalizedSourceMap, NormalizedSourceSegment, OwnedLexToken, RewriteDocumentCst,
    RewriteLineCst, SagaChapterLineCst, SelfReferenceSurface, SentenceSplitResult,
    StatementLineCst, StaticLineCst, StructuralLineKind, StructuralNode, StructuralNodeKind,
    TokenKind, TokenWordPiece, TokenWordView, TriggerIntroCst, TriggeredLineCst,
    TurnDurationPhrase, UnsupportedLineCst, classify_document_structure,
    clone_sentence_chunk_tokens, contains_sequence, contains_token_word,
    contains_token_word_sequence, contains_window, extract_parenthetical_sentences,
    find_any_token_word_sequence_span, find_index, find_token_any_word, find_token_word,
    find_token_word_sequence, find_token_word_sequence_span, find_token_word_sequence_value,
    find_window_by, find_window_index, is_at_trigger_intro_lexed, iter_contains, lex_line,
    lexed_head_words, lexed_tokens_contain_non_prefix_instead,
    looks_like_reflexive_followup_intro_lexed, looks_like_spell_resolution_followup_intro_lexed,
    make_line_info, normalize_restriction_text, normalize_trimmed_line, parse_common_sentence_head,
    parse_leading_may_action_lexed, parse_metadata_line, parse_turn_duration_prefix,
    parse_turn_duration_suffix, parser_token_word_positions, parser_token_word_refs,
    remove_copy_exception_type_removal_lexed, render_token_slice,
    rewrite_followup_intro_to_if_lexed, rfind_index, rfind_token_word, slice_contains,
    slice_contains_all, slice_contains_any, slice_ends_with, slice_eq_any, slice_starts_with,
    slice_starts_with_any, slice_strip_prefix, slice_strip_suffix, split_em_dash_label_prefix,
    split_em_dash_label_prefix_tokens, split_lexed_once_on_comma, split_lexed_once_on_comma_then,
    split_lexed_once_on_delimiter, split_lexed_once_on_period, split_lexed_sentences,
    split_sentences_for_parse, split_sentences_for_parse_fallback, split_text_for_parse,
    split_text_for_parse_with_restrictions, str_contains, str_contains_char, str_ends_with,
    str_ends_with_char, str_find, str_find_char, str_split_once, str_split_once_char,
    str_starts_with, str_starts_with_char, str_strip_prefix, str_strip_suffix,
    strip_leading_if_you_do_lexed, token_word_pieces_for_token, token_word_refs, trim_lexed_commas,
    word_slice_at_is, word_slice_at_is_any, word_slice_contains_all_words,
    word_slice_contains_any_phrase, word_slice_contains_any_phrase_or_empty,
    word_slice_contains_any_word, word_slice_contains_no_words, word_slice_contains_phrase,
    word_slice_contains_phrase_or_empty, word_slice_contains_word, word_slice_ends_with,
    word_slice_find_any_phrase_span, word_slice_find_any_word, word_slice_find_phrase_start,
    word_slice_find_phrase_start_or_zero, word_slice_find_phrase_value, word_slice_find_word,
    word_slice_find_word_where, word_slice_first_is, word_slice_first_is_any, word_slice_last_is,
    word_slice_last_is_any, word_slice_matching_phrase, word_slice_matching_value,
    word_slice_rfind_word_where, word_slice_starts_with, word_slice_starts_with_any,
    word_slice_strip_first_word_value, word_slice_strip_prefix_value,
    word_slice_strip_suffix_value, word_view_has_any_prefix, word_view_has_prefix,
};
pub use ids::{CardId, ObjectId, PlayerId, StableId};
pub use ironsmith_core::{
    AttachmentConditionHost, CompanionDeckCardFacts, CompanionDeckCondition,
    Condition as ConditionExpr, PermanentLeftBattlefieldControlSurface,
    SourceCounterThresholdSurface, WorkspaceSplitMarker,
};
pub use model::provenance::{
    DashStyle, ProvenanceId, ProvenanceRecord, ProvenanceStore, ProvenanceView, Provenanced,
    PunctuationKind, QuoteStyle, ReminderTextDecision, RenderingHint, SemanticProvenance,
    SourcePosition, SourceSliceKind, SourceSpan, SourceUnit,
};
pub use model::symbols::{
    Cardinality, ObjectDomain, ReferenceQuery, ReferenceRole, SymbolBinding, SymbolId,
    SymbolReference, SymbolResolutionError, SymbolScope, SymbolScopeId, SymbolScopeKind,
    SymbolTable,
};
pub use model::{
    AdditionalCostChoiceOptionAst, AnnotatedEffect, AnnotatedEffectSequence, ClashOpponentAst,
    CompilerAbility, CompilerAbilityKind, CompilerAbilityPayload, CompilerActivatedAbility,
    CompilerAlternativeCastingMethod, CompilerCost, CompilerDocument, CompilerDocumentItem,
    CompilerOptionalCost, CompilerTotalCost, CompilerTriggeredAbility, ControlDurationAst,
    CostRelationship, DamageBySpec, ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, GiftTimingAst, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, LineAst, LoweredEffects, ObjectRefAst,
    ParsedAbility, ParsedCardItem, ParsedCardItemKind, ParsedLevelAbilityAst,
    ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalActivatedHeader, ParsedModalAst,
    ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
    RedirectNextTimeDamageDestinationAst, RefState, ReferenceEnv, ReferenceExports, ReferenceFrame,
    ReferenceImports, RestrictionBucket, RetargetModeAst, ReturnControllerAst,
    RewriteActivatedLine, RewriteKeywordLine, RewriteLevelHeader, RewriteLevelItem,
    RewriteLevelItemKind, RewriteModalBlock, RewriteModalMode, RewriteSagaChapterLine,
    RewriteSemanticDocument, RewriteSemanticItem, RewriteStatementLine, RewriteStaticLine,
    RewriteTriggeredLine, RewriteUnsupportedLine, SearchLibrarySlotAst, SharedTypeConstraintAst,
    TargetAst, ZoneReplacementDurationAst,
};
pub use object::{AuraAttachmentFilter, CounterType};
pub use oracle_grammar::{
    OracleGrammarDocument, OracleGrammarLevelItem, OracleGrammarLine, OracleGrammarLineInfo,
    OracleGrammarMode, parse_oracle_grammar_document,
};
pub use parse_context::{
    CardFaceMetadata, ContextDiagnostic, ParseArenaId, ParseArenas, ParseContext, ParseContextView,
    ParseDiagnosticSink, ParseFeatures, ParseScopeId, ParseScopeKind, SourceIdentity, SourceUnitId,
};
pub use payload::{IfResultPredicate, KeywordAction};
pub use pipeline::{LoweringPipeline, PostpassProcessor};
pub use recognition::{
    ParseDiagnostic, ParseDiagnosticKind, ParseExpectation, ParseMatch, ParseOutcome, RuleId,
    RuleMatch, UnsupportedReason,
};
pub use registry::{
    HeadDiscriminator, LegacyCompatibilityRule, LegacyOrderRank, RegistryCandidate,
    RegistryRuleMetadata, SemanticEquivalenceKey, SourceSpanPolicy, furthest_committed_diagnostic,
    resolve_registry_candidates,
};
pub use tag::TagKey;
pub use target::{
    ChooseSpec, ObjectCharacteristic, ObjectCharacteristicRelation,
    ObjectCharacteristicRelationKind, ObjectFilter, ObjectRef, PlayerFilter,
    TaggedObjectConstraint, TaggedOpbjectRelation,
};
pub use types::{CardType, Subtype, Supertype};
pub use zone::Zone;
