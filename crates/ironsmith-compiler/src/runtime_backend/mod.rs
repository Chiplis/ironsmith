pub(crate) use super::*;
pub(crate) use crate::cards::builders::GrantedAbilityAst;

pub(crate) mod facade;
pub(crate) mod families;
pub(crate) mod front_end;
pub(crate) mod lowering;
pub(crate) mod model;
pub(crate) mod references;
pub(crate) mod sentences;

#[path = "families/activation_and_restrictions/mod.rs"]
pub(crate) mod activation_and_restrictions;
#[path = "families/activation_helpers.rs"]
pub(crate) mod activation_helpers;
#[path = "model/ast.rs"]
pub(crate) mod ast;
#[path = "lowering/battlefield_entry_counter_fusion.rs"]
pub(crate) mod battlefield_entry_counter_fusion;
#[path = "families/clause_support.rs"]
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
#[path = "families/keyword_payloads.rs"]
pub(crate) mod keyword_payloads;
#[path = "families/keyword_registry.rs"]
pub(crate) mod keyword_registry;
#[path = "families/keyword_static/mod.rs"]
pub(crate) mod keyword_static;
#[path = "families/keyword_static_helpers.rs"]
pub(crate) mod keyword_static_helpers;
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
#[path = "front_end/rule_engine.rs"]
pub(crate) mod rule_engine;
#[path = "sentences/search_library_support.rs"]
pub(crate) mod search_library_support;
#[path = "model/semantic.rs"]
pub(crate) mod semantic;
#[path = "front_end/semantic_document.rs"]
pub(crate) mod semantic_document;
#[path = "front_end/semantic_line_parsing/mod.rs"]
pub(crate) mod semantic_line_parsing;
#[path = "model/shared_types.rs"]
pub(crate) mod shared_types;
#[path = "families/static_ability_helpers.rs"]
pub(crate) mod static_ability_helpers;
#[path = "model/token_definition.rs"]
pub(crate) mod token_definition;
#[path = "front_end/token_primitives.rs"]
pub(crate) mod token_primitives;
#[path = "front_end/shared/util.rs"]
pub(crate) mod util;

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
pub(crate) use cst_lowering::lower_activation_cost_cst;
#[cfg(test)]
pub(crate) use effect_sentences::clause_pattern_helpers;
pub(crate) use effect_sentences::{CarryContext, TokenCopyFollowup, Verb, parse_type_line};
#[cfg(test)]
pub(crate) use effect_sentences::{
    find_verb, parse_cant_effect_sentence_lexed, parse_choice_of_abilities,
    parse_effect_clause_lexed, parse_effect_sentence_lexed, parse_half_starting_life_total_value,
    parse_restriction_duration_lexed, parse_search_library_sentence_lexed,
    parse_sentence_choose_then_do_same_for_filter, parse_sentence_delayed_next_step_unless_pays,
    parse_sentence_put_multiple_counters_on_target, parse_shared_color_target_fanout_sentence,
    split_choose_list,
};
#[cfg(test)]
pub(crate) use grammar::activation_costs::{
    ActivationCostSegmentCst, parse_activation_cost_rewrite, parse_activation_cost_tokens_rewrite,
};
pub(crate) use grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed as parse_spell_filter_lexed;
#[cfg(test)]
pub(crate) use grammar::filters::{
    parse_object_filter_with_grammar_entrypoint as parse_object_filter,
    parse_spell_filter_with_grammar_entrypoint as parse_spell_filter,
};
#[cfg(test)]
pub(crate) use grammar::structure::parse_predicate_with_grammar_entrypoint_lexed as parse_predicate_lexed;
#[cfg(test)]
pub(crate) use grammar::values::{
    parse_mana_cost_rewrite, parse_mana_symbol_group_rewrite, parse_type_line_rewrite,
};
pub(crate) use ir::RewriteSemanticDocument as LegacySemanticDocument;
#[cfg(test)]
pub(crate) use ir::{RewriteKeywordLineKind, RewriteSemanticItem};
#[cfg(test)]
pub(crate) use keyword_static::{
    parse_add_mana_equal_amount_value, parse_combined_pregame_choose_color_line,
};
pub(crate) use lexer::{OwnedLexToken, token_word_refs};
#[cfg(test)]
pub(crate) use lexer::{TokenWordView, lex_line, split_lexed_sentences};
#[cfg(test)]
pub(crate) use object_filters::parse_object_filter_lexed;
#[cfg(test)]
pub(crate) use parser_support::{
    looks_like_reflexive_followup_intro_lexed, looks_like_spell_resolution_followup_intro_lexed,
};
pub(crate) use permission_helpers::{PermissionClauseSpec, PermissionLifetime};
#[cfg(test)]
pub(crate) use pipeline::parse_text_to_semantic_document;
#[cfg(test)]
pub(crate) use pipeline::parse_text_with_annotations_lowered;
#[cfg(test)]
pub(crate) use reference_model::RefState;
pub(crate) use reference_model::{ReferenceEnv, ReferenceExports, ReferenceImports};
#[cfg(test)]
pub(crate) use rule_engine::{LexClauseView, RULE_SHAPE_HAS_COMMA, RULE_SHAPE_STARTS_WHENEVER};
#[cfg(test)]
pub(crate) use search_library_support::{
    SearchLibraryManaConstraint, extract_search_library_mana_constraint,
    split_search_different_name_reference_filter, split_search_same_name_reference_filter,
};
#[cfg(test)]
pub(crate) use semantic_line_parsing::{
    parse_keyword_line_for_test, parse_keyword_line_with_full_tokens_for_test,
    parse_statement_token_groups_to_chunks, parse_static_line, parse_triggered_line,
};
pub(crate) use shared_types::{
    CompileContext, EffectLoweringContext, IdGenContext, LineInfo, LoweringFrame, MetadataLine,
    NormalizedLine,
};
pub(crate) use util::{
    SubjectAst, parse_counter_type_from_tokens, parse_power_toughness, parse_scryfall_mana_cost,
    span_from_tokens,
};

pub(crate) use facade::{CardTextCompiler, CompilePolicy, CompiledCardText};

pub(crate) fn compile_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<CompiledCardText, CardTextError> {
    stacker::grow(16 * 1024 * 1024, || {
        let text = text.into();
        let mut builder = builder;
        let card_name = builder.card_builder.name_ref().to_string();
        // Payload-backed callers put card identity in `Type:` metadata instead
        // of pre-seeding the builder. Install that identity before source-aware
        // grammar resolves phrases such as "this enchantment's".
        for raw_line in text.lines() {
            let Some(crate::front_end::MetadataLine::TypeLine(raw_type_line)) =
                crate::front_end::parse_metadata_line(raw_line)?
            else {
                continue;
            };
            builder =
                builder.apply_metadata(crate::front_end::MetadataLine::TypeLine(raw_type_line))?;
        }
        let card_types = builder.card_builder.card_types_ref().to_vec();
        let subtypes = builder.card_builder.subtypes_ref().to_vec();
        util::with_card_source_reference_context(card_name.as_str(), &card_types, &subtypes, || {
            CardTextCompiler::compile(builder, text, CompilePolicy { allow_unsupported })
        })
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
