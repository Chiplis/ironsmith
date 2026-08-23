//! Compiler-owned front-end utilities.
//!
//! This is the start of the parser front-end extraction. The full grammar and
//! lowering pipeline still lives elsewhere, but lexing and token-view utilities
//! now have a real home in `ironsmith-compiler`.

pub use ironsmith_compiler_source::cst_primitives;
pub use ironsmith_compiler_source::document_cst;
pub use ironsmith_compiler_source::document_structure;
pub use ironsmith_compiler_syntax::lexer;
pub mod parser_support;
pub use ironsmith_compiler_source::preprocess;
pub use ironsmith_compiler_source::source_model;
pub use ironsmith_compiler_syntax::token_utils;

pub(crate) use crate::grammar;
pub(crate) mod semantic_domain_migration;

pub use cst_primitives::{
    KeywordLineCst, KeywordLineKindCst, MetadataLineCst, StatementLineCst, StaticLineCst,
    TriggerIntroCst, UnsupportedLineCst,
};
pub use document_cst::{
    ActivatedLineCst, LevelHeaderCst, LevelItemCst, LevelItemKindCst, ModalBlockCst, ModalModeCst,
    RewriteDocumentCst, RewriteLineCst, SagaChapterLineCst, TriggeredLineCst,
};
pub use document_structure::{
    ClassifiedFace, ClassifiedLine, DocumentStructure, ModeMarker, SelfReferenceSurface,
    StructuralLineKind, StructuralNode, StructuralNodeKind, classify_document_structure,
};
pub use lexer::{
    LexCursor, LexStream, LexToken, LexerError, OwnedLexToken, TokenKind, TokenWordPiece,
    TokenWordView, contains_token_any_word, contains_token_kind, contains_token_word,
    contains_token_word_sequence, find_any_token_word_sequence_span, find_token_any_word,
    find_token_kind, find_token_word, find_token_word_sequence, find_token_word_sequence_span,
    find_token_word_sequence_value, lex_line, parser_token_word_positions, parser_token_word_refs,
    render_token_slice, rfind_token_word, split_lexed_sentences, token_word_pieces_for_token,
    token_word_refs, trim_lexed_commas, word_slice_at_is, word_slice_at_is_any,
    word_slice_contains_all_words, word_slice_contains_any_phrase,
    word_slice_contains_any_phrase_or_empty, word_slice_contains_any_word,
    word_slice_contains_no_words, word_slice_contains_phrase, word_slice_contains_phrase_or_empty,
    word_slice_contains_window_by, word_slice_contains_word, word_slice_ends_with,
    word_slice_ends_with_any, word_slice_eq, word_slice_eq_any, word_slice_find_any_phrase_span,
    word_slice_find_any_phrase_start, word_slice_find_any_phrase_start_or_zero,
    word_slice_find_any_word, word_slice_find_phrase_start, word_slice_find_phrase_start_or_zero,
    word_slice_find_phrase_value, word_slice_find_window_by, word_slice_find_word,
    word_slice_find_word_where, word_slice_first_is, word_slice_first_is_any, word_slice_last_is,
    word_slice_last_is_any, word_slice_matching_phrase, word_slice_matching_value,
    word_slice_rfind_word_where, word_slice_starts_with, word_slice_starts_with_any,
    word_slice_strip_any_prefix, word_slice_strip_any_suffix, word_slice_strip_first_word,
    word_slice_strip_first_word_value, word_slice_strip_prefix, word_slice_strip_prefix_value,
    word_slice_strip_suffix, word_slice_strip_suffix_value,
};
pub use parser_support::{
    SentenceSplitResult, extract_parenthetical_sentences, is_at_trigger_intro_lexed,
    looks_like_reflexive_followup_intro_lexed, looks_like_spell_resolution_followup_intro_lexed,
    normalize_restriction_text, split_sentences_for_parse, split_sentences_for_parse_fallback,
    split_text_for_parse, split_text_for_parse_with_restrictions,
};
pub use preprocess::{make_line_info, normalize_trimmed_line, parse_metadata_line};
pub use source_model::{
    LineInfo, MetadataLine, NormalizedLine, NormalizedSourceMap, NormalizedSourceSegment,
};
pub use token_utils::{
    CommonSentenceHead, LeadingMayActionMatch, LeadingMayActor, TurnDurationPhrase,
    clone_sentence_chunk_tokens, contains_sequence, contains_window, find_index, find_window_by,
    find_window_index, iter_contains, iter_eq, lexed_head_words,
    lexed_tokens_contain_non_prefix_instead, parse_common_sentence_head,
    parse_leading_may_action_lexed, parse_turn_duration_prefix, parse_turn_duration_suffix,
    remove_copy_exception_type_removal_lexed, rewrite_followup_intro_to_if_lexed, rfind_index,
    slice_contains, slice_contains_all, slice_contains_any, slice_ends_with, slice_eq_any,
    slice_starts_with, slice_starts_with_any, slice_strip_prefix, slice_strip_suffix,
    split_em_dash_label_prefix, split_em_dash_label_prefix_tokens, split_lexed_once_on_comma,
    split_lexed_once_on_comma_then, split_lexed_once_on_delimiter, split_lexed_once_on_period,
    str_contains, str_contains_char, str_ends_with, str_ends_with_any_char, str_ends_with_char,
    str_find, str_find_char, str_rfind, str_rfind_char, str_split_once, str_split_once_char,
    str_starts_with, str_starts_with_char, str_strip_prefix, str_strip_suffix,
    str_strip_suffix_char, strip_leading_if_you_do_lexed, word_view_has_any_prefix,
    word_view_has_prefix,
};
