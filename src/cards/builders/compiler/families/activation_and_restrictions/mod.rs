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

mod activated_sentence_parsers;
#[path = "keyword_activated_lines.rs"]
mod keyword_activated_lines;

use activated_sentence_parsers::collect_activated_sentence_modifiers;
pub(crate) use keyword_activated_lines::*;

include!("activated_line_core.rs");
include!("activation_costs.rs");
include!("activation_restriction_clauses.rs");
include!("keyword_action_costs.rs");
include!("trigger_clause_core.rs");
include!("trigger_subject_filters.rs");
include!("choice_object_clauses.rs");
