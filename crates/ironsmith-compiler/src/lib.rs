//! Compiler crate for parser/front-end ownership in the split workspace.
//!
//! The full parser and lowering pipeline has not been extracted yet, but this
//! crate now owns the compiler-facing diagnostics and source-preparation
//! surface instead of acting as a pure marker package.

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
pub mod mana;
pub mod model;
pub mod object;
pub mod oracle_grammar;
pub mod parse_loss;
pub mod parse_trace;
pub mod payload;
pub mod pipeline;
pub mod resolution;
mod runtime_backend;
mod slice_primitives;
pub mod static_abilities;
mod string_primitives;
pub mod tag;
pub mod target;
pub mod triggers;
pub mod types;
mod word_primitives;
pub mod zone;

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
    CompilerSourceDocument, ParseCacheKey,
};
pub use front_end::{
    ActivatedLineCst, CommonSentenceHead, KeywordLineCst, KeywordLineKindCst,
    LeadingMayActionMatch, LeadingMayActor, LevelHeaderCst, LevelItemCst, LevelItemKindCst,
    LexCursor, LexStream, LexToken, LexerError, LineInfo, MetadataLine, MetadataLineCst,
    ModalBlockCst, ModalModeCst, NormalizedLine, OwnedLexToken, RewriteDocumentCst, RewriteLineCst,
    SagaChapterLineCst, SentenceSplitResult, StatementLineCst, StaticLineCst, TokenKind,
    TokenWordPiece, TokenWordView, TriggerIntroCst, TriggeredLineCst, TurnDurationPhrase,
    UnsupportedLineCst, clone_sentence_chunk_tokens, contains_sequence, contains_token_word,
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
pub use ironsmith_core::Condition as ConditionExpr;
pub use ironsmith_core::WorkspaceSplitMarker;
pub use model::{
    AdditionalCostChoiceOptionAst, AnnotatedEffect, AnnotatedEffectSequence, ClashOpponentAst,
    ControlDurationAst, DamageBySpec, ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst,
    GiftTimingAst, LibraryBottomOrderAst, LibraryConsultModeAst, LibraryConsultStopRuleAst,
    LineAst, LoweredEffects, ObjectRefAst, ParsedAbility, ParsedCardItem, ParsedCardItemKind,
    ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalActivatedHeader,
    ParsedModalAst, ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions,
    PlayerAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RefState,
    ReferenceEnv, ReferenceExports, ReferenceFrame, ReferenceImports, RestrictionBucket,
    RetargetModeAst, ReturnControllerAst, RewriteActivatedLine, RewriteKeywordLine,
    RewriteLevelHeader, RewriteLevelItem, RewriteLevelItemKind, RewriteModalBlock,
    RewriteModalMode, RewriteSagaChapterLine, RewriteSemanticDocument, RewriteSemanticItem,
    RewriteStatementLine, RewriteStaticLine, RewriteTriggeredLine, RewriteUnsupportedLine,
    SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst, ZoneReplacementDurationAst,
};
pub use object::{AuraAttachmentFilter, CounterType};
pub use oracle_grammar::{
    OracleGrammarDocument, OracleGrammarLevelItem, OracleGrammarLine, OracleGrammarLineInfo,
    OracleGrammarMode, parse_oracle_grammar_document,
};
pub use payload::{IfResultPredicate, KeywordAction};
pub use pipeline::{LoweringPipeline, PostpassProcessor};
pub use tag::TagKey;
pub use target::{
    ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
pub use types::{CardType, Subtype, Supertype};
pub use zone::Zone;
