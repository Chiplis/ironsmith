#![allow(unused_imports)]

mod activation_and_restrictions;
mod activation_helpers;
mod clause_support;
mod compile_support;
mod cst;
mod document_parser;
mod effect_ast_normalization;
mod effect_ast_traversal;
mod effect_pipeline;
mod effect_sentences;
mod grammar;
mod ir;
mod keyword_static;
mod keyword_static_helpers;
mod leaf;
mod lexer;
mod lower;
mod lowering_support;
mod modal_helpers;
mod modal_support;
mod object_filters;
mod parser_support;
mod permission_helpers;
mod preprocess;
mod reference_helpers;
mod reference_model;
mod reference_resolution;
mod restriction_support;
mod rule_engine;
mod shared_types;
mod static_ability_helpers;
mod token_primitives;
mod util;
mod value_helpers;
pub(crate) use activation_and_restrictions::*;
pub(crate) use document_parser::*;
pub(crate) use effect_pipeline::*;
pub(crate) use effect_sentences::*;
pub(crate) use grammar::filters::parse_object_filter_with_grammar_entrypoint as parse_object_filter;
pub(crate) use grammar::filters::parse_spell_filter_with_grammar_entrypoint as parse_spell_filter;
pub(crate) use grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed as parse_spell_filter_lexed;
pub(crate) use grammar::structure::parse_predicate_with_grammar_entrypoint_lexed as parse_predicate_lexed;
#[cfg(test)]
pub(crate) use grammar::values::{
    parse_count_word_rewrite, parse_mana_cost_rewrite, parse_mana_symbol_group_rewrite,
    parse_type_line_rewrite,
};
pub(crate) use ir::*;
pub(crate) use keyword_static::*;
#[cfg(test)]
pub(crate) use leaf::*;
pub(crate) use lexer::{
    OwnedLexToken, TokenWordView, lex_line, split_lexed_sentences, token_word_refs,
};
pub(crate) use lower::*;
pub(crate) use object_filters::{
    is_comparison_or_delimiter, merge_spell_filters, parse_object_filter_lexed,
    spell_filter_has_identity,
};
pub(crate) use parser_support::*;
pub(crate) use permission_helpers::{PermissionClauseSpec, PermissionLifetime};
pub(crate) use reference_model::*;
pub(crate) use rule_engine::*;
pub(crate) use shared_types::{
    CompileContext, EffectLoweringContext, IdGenContext, LineInfo, LoweringFrame, MetadataLine,
    NormalizedLine,
};
#[cfg(test)]
pub(crate) use util::tokenize_line;
pub(crate) use util::{
    SubjectAst, contains_until_end_of_turn, find_activation_cost_start, is_basic_color_word,
    is_sentence_helper_tag, parse_counter_type_from_tokens, parse_counter_type_word, parse_number,
    parse_number_or_x_value, parse_target_phrase, replace_unbound_x_with_value, span_from_tokens,
    starts_with_activation_cost, value_contains_unbound_x, words,
};

#[cfg(test)]
mod tests;
