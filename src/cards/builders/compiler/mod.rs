#![allow(unused_imports)]

#[allow(unused_imports)]
pub(crate) use super::*;

pub(crate) mod facade;
pub(crate) mod families;
pub(crate) mod front_end;
pub(crate) mod lowering;
pub(crate) mod model;
pub(crate) mod postpasses;
pub(crate) mod references;
pub(crate) mod sentences;

#[path = "families/activation_and_restrictions/mod.rs"]
pub(crate) mod activation_and_restrictions;
#[path = "families/activation_helpers.rs"]
pub(crate) mod activation_helpers;
#[path = "model/ast.rs"]
pub(crate) mod ast;
#[path = "families/clause_support.rs"]
pub(crate) mod clause_support;
#[path = "lowering/compile_support.rs"]
pub(crate) mod compile_support;
#[path = "front_end/cst.rs"]
pub(crate) mod cst;
#[path = "front_end/cst_lowering.rs"]
pub(crate) mod cst_lowering;
#[path = "front_end/document/mod.rs"]
pub(crate) mod document_parser;
#[path = "model/effect_ast_normalization.rs"]
pub(crate) mod effect_ast_normalization;
#[path = "model/effect_ast_traversal.rs"]
pub(crate) mod effect_ast_traversal;
#[path = "lowering/effect_pipeline.rs"]
pub(crate) mod effect_pipeline;
#[path = "sentences/effect_sentences/mod.rs"]
pub(crate) mod effect_sentences;
#[path = "front_end/grammar/mod.rs"]
pub(crate) mod grammar;
#[path = "model/ir.rs"]
pub(crate) mod ir;
#[path = "families/keyword_families.rs"]
pub(crate) mod keyword_families;
#[path = "families/keyword_registry.rs"]
pub(crate) mod keyword_registry;
#[path = "families/keyword_static/mod.rs"]
pub(crate) mod keyword_static;
#[path = "families/keyword_static_helpers.rs"]
pub(crate) mod keyword_static_helpers;
#[path = "front_end/leaf.rs"]
pub(crate) mod leaf;
#[path = "front_end/lexer.rs"]
pub(crate) mod lexer;
#[path = "lowering/lower/mod.rs"]
pub(crate) mod lower;
#[path = "lowering/lowering_support.rs"]
pub(crate) mod lowering_support;
#[path = "families/modal_helpers.rs"]
pub(crate) mod modal_helpers;
#[path = "model/modal_support.rs"]
pub(crate) mod modal_support;
#[path = "families/object_filters.rs"]
pub(crate) mod object_filters;
#[path = "front_end/parser_support.rs"]
pub(crate) mod parser_support;
#[path = "families/permission_helpers.rs"]
pub(crate) mod permission_helpers;
#[path = "lowering/pipeline.rs"]
pub(crate) mod pipeline;
#[path = "front_end/preprocess.rs"]
pub(crate) mod preprocess;
#[path = "references/reference_helpers.rs"]
pub(crate) mod reference_helpers;
#[path = "references/reference_model.rs"]
pub(crate) mod reference_model;
#[path = "references/reference_resolution.rs"]
pub(crate) mod reference_resolution;
#[path = "families/restriction_support.rs"]
pub(crate) mod restriction_support;
#[path = "lowering/rewrite_exceptions.rs"]
pub(crate) mod rewrite_exceptions;
#[path = "front_end/rule_engine.rs"]
pub(crate) mod rule_engine;
#[path = "sentences/search_library_support.rs"]
pub(crate) mod search_library_support;
#[path = "model/semantic.rs"]
pub(crate) mod semantic;
#[path = "model/shared_types.rs"]
pub(crate) mod shared_types;
#[path = "families/static_ability_helpers.rs"]
pub(crate) mod static_ability_helpers;
#[path = "front_end/token_primitives.rs"]
pub(crate) mod token_primitives;
#[path = "front_end/shared/util.rs"]
pub(crate) mod util;
#[path = "front_end/shared/value_helpers.rs"]
pub(crate) mod value_helpers;

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
    find_verb, parse_cant_effect_sentence, parse_cant_effect_sentence_lexed,
    parse_choice_of_abilities, parse_effect_clause_lexed, parse_effect_sentence_lexed,
    parse_half_starting_life_total_value, parse_restriction_duration,
    parse_restriction_duration_lexed, parse_search_library_sentence_lexed,
    parse_sentence_choose_then_do_same_for_filter, parse_sentence_delayed_next_step_unless_pays,
    parse_sentence_put_multiple_counters_on_target, parse_shared_color_target_fanout_sentence,
    split_choose_list,
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
pub(crate) use pipeline::parse_text_to_semantic_document;
pub(crate) use pipeline::parse_text_with_annotations;
#[cfg(test)]
pub(crate) use pipeline::parse_text_with_annotations_lowered;
#[cfg(test)]
pub(crate) use reference_model::RefState;
pub(crate) use reference_model::{ReferenceEnv, ReferenceExports, ReferenceImports};
pub(crate) use ir::RewriteSemanticDocument as LegacySemanticDocument;
#[cfg(test)]
pub(crate) use rule_engine::{LexClauseView, RULE_SHAPE_HAS_COMMA, RULE_SHAPE_STARTS_WHENEVER};
#[cfg(test)]
pub(crate) use search_library_support::{
    SearchLibraryManaConstraint, extract_search_library_mana_constraint,
    split_search_same_name_reference_filter,
};
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

#[allow(unused_imports)]
pub(crate) use facade::{CardTextCompiler, CompilePolicy, CompiledCardText};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
