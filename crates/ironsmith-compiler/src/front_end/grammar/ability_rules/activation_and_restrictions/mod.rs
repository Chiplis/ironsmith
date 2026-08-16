use super::activation_helpers::{
    find_activation_cost_start, join_sentences_with_period, non_article_word_refs, parse_add_mana,
    parse_filter_comparison_tokens, parse_subtype_flexible, strip_leading_article_tokens,
    trim_edge_punctuation_tokens, value_contains_unbound_x,
};
use super::cst_lowering::lower_activation_cost_cst;
use super::effect_ast_traversal::{for_each_nested_effects, for_each_nested_effects_mut};
use super::effect_sentences::{
    parse_effect_sentence_lexed, parse_effect_sentences_lexed, parse_restriction_duration,
    parse_scryfall_mana_cost, replace_unbound_x_in_effect_anywhere, strip_leading_articles,
    trim_edge_punctuation,
};
use super::grammar::activation_costs::parse_activation_cost_tokens;
use super::grammar::primitives as grammar;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_cost_modifier_amount, parse_cost_modifier_mana_cost,
    parse_dynamic_cost_modifier_value, parse_static_condition_clause, parse_value_binding_clause,
    parse_value_binding_clause_lexed,
};
use super::lexer::{OwnedLexToken, TokenKind};
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::token_primitives::{contains_window, lexed_head_words};
use super::util::{
    current_source_reference_name, is_source_reference_words, parse_card_type, parse_color,
    parse_counter_type_from_tokens, parse_greater_than_or_equal_quantity_prefix, parse_non_type,
    parse_number, parse_number_word_u32, parse_subject, parse_target_phrase,
    parse_value_expr_words, source_reference_surface_for_span, source_reference_surface_for_words,
    span_from_tokens, this_source_surface_for_words, trim_commas, words,
};
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardTextError, DamageBySpec, EffectAst, IT_TAG, KeywordAction, ParsedAbility, PlayerAst,
    PredicateAst, ReferenceImports, ReturnControllerAst, StaticAbilityAst, TagKey, TargetAst,
    TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::effect::{ChoiceCount, Effect, Until, Value};
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::ast::TriggerIntroSurfaceAst;
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) mod activated_line_core;
mod activated_sentence_parsers;
pub(crate) mod activation_costs;
pub(crate) mod activation_restriction_clauses;
pub(crate) mod choice_object_clauses;
pub(crate) mod keyword_action_costs;
#[path = "keyword_activated_lines.rs"]
pub(crate) mod keyword_activated_lines;
#[path = "../../trigger_clauses/semantic.rs"]
pub(crate) mod trigger_clause_core;
pub(crate) mod trigger_subject_filters;

use activated_line_core::*;
pub(crate) use activated_line_core::{
    color_from_color_set, combine_mana_activation_condition, is_activate_only_restriction_sentence,
    is_any_player_may_activate_sentence_lexed, is_trigger_only_restriction_sentence,
    parse_activated_line, parse_activation_cost, parse_all_creatures_able_to_block_source_line,
    parse_cost_reduction_line, parse_devotion_value_from_add_clause, parse_enters_tapped_line,
    parse_mana_usage_restriction_sentence_lexed, parse_named_number,
    parse_source_must_be_blocked_if_able_line, parse_triggered_times_each_turn_lexed,
    scale_dynamic_cost_modifier_value,
};
#[cfg(test)]
pub(crate) use activated_line_core::{
    parse_activate_only_timing_lexed, parse_activation_condition_lexed,
};
use activated_sentence_parsers::collect_activated_sentence_modifiers;
pub(crate) use activation_costs::parse_cant_clauses;
use activation_restriction_clauses::*;
pub(crate) use activation_restriction_clauses::{
    find_negation_span, parse_cant_restriction_clause, parse_cant_restrictions,
    parse_subject_object_filter, parse_type_adjective_conjunction_filter,
    starts_with_target_indicator,
};
#[cfg(test)]
pub(crate) use choice_object_clauses::parse_you_choose_objects_clause;
pub(crate) use choice_object_clauses::{
    parse_choose_basic_land_type_phrase_words, parse_choose_card_type_phrase_words,
    parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand, parse_choose_color_phrase_words,
    parse_choose_creature_type_phrase_words, parse_choose_creature_type_then_become_type,
    parse_choose_land_type_phrase_words, parse_choose_player_phrase_words,
    parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    parse_target_player_choose_objects_clause_with_count_value,
    parse_target_player_chooses_then_other_cant_block,
    parse_you_choose_objects_clause_with_count_value, parse_you_choose_player_clause,
};
use keyword_action_costs::*;
pub(crate) use keyword_action_costs::{
    find_payment_alternative_or, normalize_cant_words, parse_ability_phrase,
    parse_payment_clause_as_total_cost, parse_single_graveyard_bottom_library_payment,
    parse_single_word_keyword_action, target_ast_to_object_filter,
};
pub(crate) use keyword_activated_lines::{
    parse_channel_line_lexed, parse_craft_line_lexed, parse_cycling_line, parse_cycling_line_lexed,
    parse_equip_line_lexed, parse_reconfigure_line_lexed,
};
use trigger_clause_core::*;
pub(crate) use trigger_clause_core::{
    parse_leading_exactly_quantifier, parse_leading_or_more_quantifier, parse_trigger_clause_lexed,
};
use trigger_subject_filters::*;
pub(crate) use trigger_subject_filters::{
    append_token_reminder_to_last_create_effect, build_may_cast_tagged_effect,
    controller_filter_for_token_player, effect_creates_any_token,
    effect_creates_eldrazi_spawn_or_scion, is_generic_token_reminder_sentence,
    is_round_up_each_time_sentence, is_simple_copy_reference_sentence,
    is_spawn_scion_token_mana_reminder, last_created_token_info,
    parse_copy_reference_cost_reduction_sentence, parse_may_cast_it_sentence,
    parse_sentence_exile_that_token_when_source_leaves,
    parse_sentence_sacrifice_source_when_that_token_leaves, parse_trigger_subject_player_filter,
    strip_embedded_token_rules_text,
};
