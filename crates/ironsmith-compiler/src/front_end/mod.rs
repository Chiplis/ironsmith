//! Compiler-owned front-end utilities.
//!
//! This is the start of the parser front-end extraction. The full grammar and
//! lowering pipeline still lives elsewhere, but lexing and token-view utilities
//! now have a real home in `ironsmith-compiler`.

pub mod cst_primitives;
pub mod document_cst;
pub mod lexer;
pub mod parser_support;
pub mod preprocess;
pub mod source_model;
pub mod token_utils;

pub use cst_primitives::{
    KeywordLineCst, KeywordLineKindCst, MetadataLineCst, StatementLineCst, StaticLineCst,
    TriggerIntroCst, UnsupportedLineCst,
};
pub use document_cst::{
    ActivatedLineCst, LevelHeaderCst, LevelItemCst, LevelItemKindCst, ModalBlockCst, ModalModeCst,
    RewriteDocumentCst, RewriteLineCst, SagaChapterLineCst, TriggeredLineCst,
};
pub use lexer::{
    LexCursor, LexStream, LexToken, LexerError, OwnedLexToken, TokenKind, TokenWordPiece,
    TokenWordView, lex_line, parser_token_word_positions, parser_token_word_refs,
    render_token_slice, split_lexed_sentences, token_word_pieces_for_token, token_word_refs,
    trim_lexed_commas,
};
pub use parser_support::{
    SentenceSplitResult, extract_parenthetical_sentences, is_at_trigger_intro_lexed,
    looks_like_reflexive_followup_intro_lexed, looks_like_spell_resolution_followup_intro_lexed,
    normalize_restriction_text, split_sentences_for_parse, split_sentences_for_parse_fallback,
    split_text_for_parse, split_text_for_parse_with_restrictions,
};
pub use preprocess::{make_line_info, normalize_trimmed_line, parse_metadata_line};
pub use source_model::{LineInfo, MetadataLine, NormalizedLine};
pub use token_utils::{
    CommonSentenceHead, LeadingMayActionMatch, LeadingMayActor, TurnDurationPhrase,
    clone_sentence_chunk_tokens, contains_sequence, contains_window, find_any_str_index,
    find_index, find_str_by, find_str_index, find_window_by, find_window_index, iter_contains,
    lexed_head_words, lexed_tokens_contain_non_prefix_instead, parse_common_sentence_head,
    parse_leading_may_action_lexed, parse_turn_duration_prefix, parse_turn_duration_suffix,
    remove_copy_exception_type_removal_lexed, rewrite_followup_intro_to_if_lexed, rfind_index,
    rfind_str_by, slice_contains, slice_contains_all, slice_contains_any, slice_contains_str,
    slice_ends_with, slice_eq_any, slice_starts_with, slice_starts_with_any, slice_strip_prefix,
    slice_strip_suffix, split_em_dash_label_prefix, split_em_dash_label_prefix_tokens,
    split_lexed_once_on_comma, split_lexed_once_on_comma_then, split_lexed_once_on_delimiter,
    split_lexed_once_on_period, str_contains, str_ends_with, str_ends_with_char, str_find,
    str_find_char, str_split_once, str_split_once_char, str_starts_with, str_starts_with_char,
    str_strip_prefix, str_strip_suffix, strip_leading_if_you_do_lexed, word_view_has_any_prefix,
    word_view_has_prefix,
};
