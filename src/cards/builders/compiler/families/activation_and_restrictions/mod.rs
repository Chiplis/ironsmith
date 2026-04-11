#![allow(dead_code)]

use super::activation_helpers::{
    contains_discard_source_phrase, contains_from_command_zone_phrase,
    contains_source_from_your_graveyard_phrase, contains_source_from_your_hand_phrase,
    find_activation_cost_start, is_article, is_basic_color_word, is_comparison_or_delimiter,
    is_source_from_your_graveyard_words, join_sentences_with_period, parse_add_mana,
    parse_filter_comparison_tokens, parse_next_end_step_token_delay_flags, parse_subtype_flexible,
    split_cost_segments, value_contains_unbound_x,
};
use super::effect_ast_traversal::{for_each_nested_effects, for_each_nested_effects_mut};
use super::effect_sentences::find_verb;
use super::effect_sentences::{
    is_beginning_of_end_step_words, is_end_of_combat_words, is_negated_untap_clause,
    parse_effect_sentence_lexed, parse_effect_sentences_lexed, parse_mana_symbol,
    parse_mana_symbol_group, parse_restriction_duration, parse_scryfall_mana_cost,
    parse_subtype_word, parse_supertype_word, replace_unbound_x_in_effect_anywhere,
    strip_leading_articles, trim_edge_punctuation,
};
use super::grammar::primitives as grammar;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_cost_modifier_amount, parse_cost_modifier_mana_cost,
    parse_dynamic_cost_modifier_value, parse_static_condition_clause, parse_where_x_value_clause,
    parse_where_x_value_clause_lexed,
};
use super::leaf::{lower_activation_cost_cst, parse_activation_cost_tokens_rewrite};
use super::lexer::{OwnedLexToken, TokenKind};
use super::object_filters::{
    find_word_slice_phrase_start, parse_object_filter, parse_object_filter_lexed,
};
use super::token_primitives::{
    contains_window, find_index, find_window_by, find_window_index, lexed_head_words, rfind_index,
    slice_contains, slice_ends_with, slice_starts_with, slice_strip_prefix, slice_strip_suffix,
    str_strip_prefix, str_strip_suffix,
};
use super::util::{
    is_source_reference_words, mana_pips_from_token, parse_card_type, parse_color,
    parse_counter_type_from_tokens, parse_non_type, parse_number, parse_number_word_u32,
    parse_subject, parse_target_count_range_prefix, parse_target_phrase, span_from_tokens,
    token_index_for_word_index, trim_commas, words,
};
#[allow(unused_imports)]
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardTextError, DamageBySpec, EffectAst, IT_TAG, KeywordAction, LineAst, ParsedAbility,
    PlayerAst, PredicateAst, ReferenceImports, ReturnControllerAst, StaticAbilityAst, TagKey,
    TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::effect::{ChoiceCount, Effect, Until, Value};
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::mana::{ManaCost, ManaSymbol};
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
pub(crate) mod trigger_clause_core;
pub(crate) mod trigger_subject_filters;

use activated_line_core::*;
pub(crate) use activated_line_core::{
    color_from_color_set, combine_mana_activation_condition, contains_word_sequence,
    find_word_sequence_start, infer_activated_functional_zones_lexed,
    is_activate_only_restriction_sentence, is_activate_only_restriction_sentence_lexed,
    is_any_player_may_activate_sentence_lexed, is_trigger_only_restriction_sentence,
    is_trigger_only_restriction_sentence_lexed, parse_activate_only_timing_lexed,
    parse_activated_line, parse_activation_condition_lexed, parse_activation_cost,
    parse_all_creatures_able_to_block_source_line, parse_cost_reduction_line,
    parse_devotion_value_from_add_clause, parse_enters_tapped_line,
    parse_mana_spend_bonus_sentence_lexed, parse_mana_usage_restriction_sentence_lexed,
    parse_named_number, parse_source_must_be_blocked_if_able_line,
    parse_triggered_times_each_turn_lexed, scale_dynamic_cost_modifier_value,
};
use activated_sentence_parsers::collect_activated_sentence_modifiers;
pub(crate) use activation_costs::parse_cant_clauses;
use activation_costs::*;
use activation_restriction_clauses::*;
pub(crate) use activation_restriction_clauses::{
    find_negation_span, parse_cant_restriction_clause, parse_cant_restrictions,
    parse_subject_object_filter, starts_with_target_indicator,
};
use choice_object_clauses::*;
pub(crate) use choice_object_clauses::{
    parse_choose_basic_land_type_phrase_words, parse_choose_card_type_phrase_words,
    parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand, parse_choose_color_phrase_words,
    parse_choose_creature_type_phrase_words, parse_choose_creature_type_then_become_type,
    parse_choose_land_type_phrase_words, parse_choose_player_phrase_words,
    parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    parse_target_player_choose_objects_clause, parse_target_player_chooses_then_other_cant_block,
    parse_you_choose_objects_clause, parse_you_choose_player_clause,
};
use keyword_action_costs::*;
pub(crate) use keyword_action_costs::{
    normalize_cant_words, parse_ability_phrase, parse_single_word_keyword_action,
    target_ast_to_object_filter,
};
use keyword_activated_lines::*;
pub(crate) use keyword_activated_lines::{
    is_land_subtype, parse_channel_line_lexed, parse_cycling_line, parse_cycling_line_lexed,
    parse_equip_line_lexed,
};
use trigger_clause_core::*;
pub(crate) use trigger_clause_core::{
    parse_leading_or_more_quantifier, parse_trigger_clause_lexed,
};
use trigger_subject_filters::*;
pub(crate) use trigger_subject_filters::{
    MayCastTaggedSpec, append_token_reminder_to_last_create_effect, build_may_cast_tagged_effect,
    controller_filter_for_token_player, effect_creates_any_token,
    effect_creates_eldrazi_spawn_or_scion, is_generic_token_reminder_sentence,
    is_round_up_each_time_sentence, is_simple_copy_reference_sentence,
    is_spawn_scion_token_mana_reminder, parse_copy_reference_cost_reduction_sentence,
    parse_may_cast_it_sentence, parse_sentence_exile_that_token_when_source_leaves,
    parse_sentence_sacrifice_source_when_that_token_leaves, parse_trigger_subject_player_filter,
    strip_embedded_token_rules_text, title_case_token_word,
};
