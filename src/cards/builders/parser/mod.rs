#![allow(unused_imports)]

mod activation_and_restrictions;
mod activation_helpers;
mod clause_support;
mod compile_support;
mod cst;
mod cst_lowering;
mod document_parser;
mod effect_ast_normalization;
mod effect_ast_traversal;
mod effect_pipeline;
mod effect_sentences;
mod grammar;
mod ir;
mod keyword_families;
mod keyword_registry;
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
mod pipeline;
mod preprocess;
mod reference_helpers;
mod reference_model;
mod reference_resolution;
mod rewrite_exceptions;
mod restriction_support;
mod rule_engine;
mod shared_types;
mod static_ability_helpers;
mod token_primitives;
mod util;
mod value_helpers;

// Keep the non-test parser surface intentionally small. Parser internals should
// prefer importing concrete submodules directly instead of extending this list.
pub(crate) use activation_and_restrictions::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
#[cfg(test)]
pub(crate) use activation_and_restrictions::{
    parse_activate_only_timing_lexed, parse_activated_line, parse_activation_condition_lexed,
    parse_activation_cost, parse_cost_reduction_line, parse_cycling_line_lexed,
    parse_mana_usage_restriction_sentence_lexed, parse_trigger_clause_lexed,
    parse_triggered_times_each_turn_lexed, parse_you_choose_objects_clause,
};
#[cfg(test)]
pub(crate) use clause_support::parse_static_ability_ast_line_lexed;
#[cfg(test)]
pub(crate) use effect_sentences::clause_pattern_helpers;
pub(crate) use effect_sentences::{CarryContext, TokenCopyFollowup, Verb, parse_type_line};
#[cfg(test)]
pub(crate) use effect_sentences::{
    SearchLibraryManaConstraint, extract_search_library_mana_constraint, find_verb,
    parse_cant_effect_sentence, parse_cant_effect_sentence_lexed, parse_choice_of_abilities,
    parse_effect_clause_lexed, parse_effect_sentence_lexed, parse_half_starting_life_total_value,
    parse_restriction_duration, parse_restriction_duration_lexed,
    parse_search_library_sentence_lexed, parse_sentence_choose_then_do_same_for_filter,
    parse_sentence_delayed_next_step_unless_pays, parse_sentence_put_multiple_counters_on_target,
    parse_shared_color_target_fanout_sentence, split_choose_list,
    split_search_same_name_reference_filter,
};
pub(crate) use grammar::filters::parse_object_filter_with_grammar_entrypoint as parse_object_filter;
pub(crate) use grammar::filters::parse_spell_filter_with_grammar_entrypoint as parse_spell_filter;
pub(crate) use grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed as parse_spell_filter_lexed;
pub(crate) use grammar::structure::parse_predicate_with_grammar_entrypoint_lexed as parse_predicate_lexed;
#[cfg(test)]
pub(crate) use grammar::values::{
    parse_count_word_rewrite, parse_mana_cost_rewrite, parse_mana_symbol_group_rewrite,
    parse_type_line_rewrite,
};
#[cfg(test)]
pub(crate) use ir::{RewriteKeywordLineKind, RewriteSemanticItem};
#[cfg(test)]
pub(crate) use keyword_static::{
    parse_add_mana_equal_amount_value, parse_combined_pregame_choose_color_line,
    parse_enters_with_counters_line,
};
#[cfg(test)]
pub(crate) use leaf::{
    ActivationCostSegmentCst, lower_activation_cost_cst, parse_activation_cost_rewrite,
    parse_activation_cost_tokens_rewrite,
};
pub(crate) use lexer::{OwnedLexToken, token_word_refs};
#[cfg(test)]
pub(crate) use lexer::{TokenWordView, lex_line, split_lexed_sentences};
#[cfg(test)]
pub(crate) use lower::{
    lower_rewrite_keyword_to_chunk, lower_rewrite_statement_token_groups_to_chunks,
    lower_rewrite_static_to_chunk, lower_rewrite_triggered_to_chunk,
};
pub(crate) use object_filters::{
    is_comparison_or_delimiter, merge_spell_filters, parse_object_filter_lexed,
    spell_filter_has_identity,
};
#[cfg(test)]
pub(crate) use parser_support::{
    is_at_trigger_intro, looks_like_reflexive_followup_intro_lexed,
    looks_like_spell_resolution_followup_intro_lexed,
};
pub(crate) use permission_helpers::{PermissionClauseSpec, PermissionLifetime};
pub(crate) use pipeline::parse_text_with_annotations;
#[cfg(test)]
pub(crate) use pipeline::{parse_text_to_semantic_document, parse_text_with_annotations_lowered};
#[cfg(test)]
pub(crate) use reference_model::RefState;
pub(crate) use reference_model::{ReferenceEnv, ReferenceExports, ReferenceImports};
#[cfg(test)]
pub(crate) use rule_engine::{LexClauseView, RULE_SHAPE_HAS_COMMA, RULE_SHAPE_STARTS_WHENEVER};
pub(crate) use shared_types::{
    CompileContext, EffectLoweringContext, IdGenContext, LineInfo, LoweringFrame, MetadataLine,
    NormalizedLine,
};
#[cfg(test)]
pub(crate) use util::tokenize_line;
pub(crate) use util::{
    SubjectAst, contains_until_end_of_turn, find_activation_cost_start, is_basic_color_word,
    is_sentence_helper_tag, parse_counter_type_from_tokens, parse_counter_type_word, parse_number,
    parse_number_or_x_value, parse_power_toughness, parse_scryfall_mana_cost, parse_target_phrase,
    replace_unbound_x_with_value, span_from_tokens, starts_with_activation_cost,
    token_index_for_word_index, value_contains_unbound_x, words,
};

#[cfg(test)]
mod tests;
