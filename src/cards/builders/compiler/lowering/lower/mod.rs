use crate::Until;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardDefinition, CardDefinitionBuilder, CardTextError, ChoiceCount, EffectAst, GiftTimingAst,
    IT_TAG, InsteadSemantics, LineAst, LineInfo, OptionalCost, ParseAnnotations, ParsedAbility,
    ParsedCardItem, ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst,
    ParsedModalAst, ParsedModalModeAst, ParsedRestrictions, PlayerAst, PredicateAst,
    ReferenceImports, ReturnControllerAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod activated_lowering;
mod line_lowering;
mod normalization_support;
mod parser_semantic_lowering;
mod rewrite_support;

pub(crate) use activated_lowering::lower_rewrite_activated_to_chunk;
use activated_lowering::{LoweredRewriteActivatedLine, align_rewrite_activated_parse_sentences};
use normalization_support::{
    apply_chosen_option_to_triggered_chunk, apply_explicit_intervening_if_to_triggered_chunk,
};
pub(crate) use normalization_support::{
    prepare_parsed_card_ast_for_lowering, rewrite_document_to_normalized_card_ast,
    rewrite_document_to_parsed_card_ast,
};
#[cfg(test)]
pub(crate) use parser_semantic_lowering::lower_rewrite_keyword_to_chunk;
use parser_semantic_lowering::{
    infer_rewrite_triggered_functional_zones, lower_rewrite_modal_to_item,
};
pub(crate) use parser_semantic_lowering::{
    lower_exert_attack_keyword_line, lower_gift_keyword_line, lower_keyword_special_cases,
    lower_rewrite_statement_token_groups_to_chunks, lower_rewrite_static_to_chunk,
    lower_rewrite_triggered_to_chunk,
};
pub(crate) use parser_semantic_lowering::{
    lower_special_rewrite_triggered_chunk, try_lower_optional_behold_additional_cost,
    try_lower_optional_cost_with_cast_trigger,
};
#[cfg(test)]
use parser_semantic_lowering::{
    normalize_exert_followup_source_reference_tokens, parse_single_effect_lexed,
    strip_lexed_suffix_phrase,
};

use rewrite_support::{
    infer_static_ability_functional_zones, infer_triggered_ability_functional_zones,
    rewrite_finalize_lowered_card, rewrite_normalize_additional_cost_sacrifice_tags,
    runtime_effects_to_costs,
};

use super::activation_and_restrictions::{
    find_word_sequence_start, infer_activated_functional_zones_lexed,
    is_any_player_may_activate_sentence_lexed, parse_activation_cost,
    parse_mana_spend_bonus_sentence_lexed, parse_mana_usage_restriction_sentence_lexed,
};
use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_cycling_line_lexed, parse_equip_line_lexed,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::compile_support::{
    collect_tag_spans_from_effects_with_context, compile_condition_from_predicate_ast_with_env,
    materialize_prepared_effects_with_trigger_context,
    trigger_binds_player_reference_context as rewrite_trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst, NormalizedCardItem,
    NormalizedLineAst, NormalizedLineChunk, NormalizedModalAst, NormalizedModalModeAst,
    NormalizedParsedAbility, NormalizedPreparedAbility, ParsedCardAst,
};
use super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use super::ir::{
    RewriteKeywordLine, RewriteKeywordLineKind, RewriteLevelHeader, RewriteModalBlock,
    RewriteSagaChapterLine, RewriteSemanticDocument, RewriteSemanticItem, RewriteStatementLine,
    RewriteStaticLine, RewriteTriggeredLine,
};
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed;
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice, split_lexed_sentences,
    token_word_refs, trim_lexed_commas,
};
use super::lowering_support::{
    rewrite_apply_instead_followup_statement_to_last_ability, rewrite_lower_prepared_ability,
    rewrite_lower_prepared_additional_cost_choice_modes_with_exports,
    rewrite_lower_prepared_statement_effects, rewrite_lower_static_abilities_ast,
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability,
    rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
    rewrite_prepare_triggered_effects_for_lowering, rewrite_static_ability_for_keyword_action,
    rewrite_validate_iterated_player_bindings_in_lowered_effects,
};
use super::modal_support::{parse_modal_header, replace_modal_header_x_in_effects_ast};
use super::parser_support::split_text_for_parse;
use super::reference_model::LoweredEffects;
use super::reference_model::ReferenceEnv;
use super::reference_model::ReferenceExports;
use super::restriction_support::{
    apply_pending_mana_restriction, apply_pending_restrictions_to_ability, is_restrictable_ability,
};
use super::token_primitives::{
    find_index, iter_contains, lexed_tokens_contain_non_prefix_instead,
    remove_copy_exception_type_removal_lexed, rewrite_followup_intro_to_if_lexed, slice_contains,
    slice_ends_with, slice_starts_with, split_em_dash_label_prefix, str_contains, str_ends_with,
    str_find, str_split_once, str_split_once_char, str_starts_with, str_strip_prefix,
    str_strip_suffix, word_view_has_any_prefix, word_view_has_prefix,
};
use super::util::{
    classify_instead_followup_text, find_first_sacrifice_cost_choice_tag,
    find_last_exile_cost_choice_tag, join_sentences_with_period,
    parse_additional_cost_choice_options_lexed, parse_bargain_line_lexed, parse_bestow_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_escape_line_lexed, parse_flashback_line_lexed, parse_harmonize_line_lexed,
    parse_if_conditional_alternative_cost_line_lexed, parse_kicker_line_lexed,
    parse_level_up_line_lexed, parse_madness_line_lexed, parse_mana_symbol,
    parse_morph_keyword_line_lexed, parse_multikicker_line_lexed, parse_number_or_x_value_lexed,
    parse_offspring_line_lexed, parse_reinforce_line_lexed, parse_scryfall_mana_cost,
    parse_self_free_cast_alternative_cost_line_lexed, parse_squad_line_lexed,
    parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed, preserve_keyword_prefix_for_parse,
    token_index_for_word_index, trim_commas, words,
};
include!("rewrite_text_helpers.rs");
include!("rewrite_sentence_grouping.rs");
include!("damage_and_cost_rewrites.rs");
include!("modal_and_level_lowering.rs");
