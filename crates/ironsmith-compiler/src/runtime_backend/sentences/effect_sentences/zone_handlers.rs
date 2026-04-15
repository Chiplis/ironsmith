#![allow(dead_code)]

#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExchangeValueAst, ExchangeValueKindAst, IT_TAG, ObjectRefAst,
    OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SharedTypeConstraintAst,
    SubjectAst, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::Subtype;
use crate::zone::Zone;

use super::super::activation_and_restrictions::{
    contains_word_sequence, controller_filter_for_token_player, find_word_sequence_start,
    parse_devotion_value_from_add_clause,
};
pub(crate) use super::super::activation_helpers::{
    parse_land_could_produce_filter, trim_leading_commas,
};
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    ConditionalPredicateTailSpec, parse_conditional_predicate_tail_lexed,
    parse_trailing_instead_if_predicate_lexed, split_trailing_if_clause_lexed,
};
use super::super::keyword_static::parse_where_x_is_number_of_filter_value;
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_pt_modifier, parse_pt_modifier_values,
};
use super::super::lexer::{LexStream, TokenKind};
use super::super::object_filters::{
    find_word_slice_phrase_start, parse_object_filter, parse_object_filter_lexed,
};
use super::super::token_primitives::{
    find_index, find_window_by, rfind_index, slice_contains, slice_ends_with, slice_starts_with,
    str_strip_suffix,
};
use super::super::util::{
    helper_tag_for_tokens, intern_counter_name, is_article, mana_pips_from_token, parse_color,
    parse_counter_type_word, parse_mana_symbol, parse_number, parse_target_phrase, parse_value,
    parse_zone_word, parser_trace_stack, span_from_tokens, token_index_for_word_index, trim_commas,
    words,
};
use super::super::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_counters_on_reference_value,
    parse_equal_to_number_of_filter_plus_or_minus_fixed_value,
    parse_equal_to_number_of_filter_value, parse_equal_to_number_of_opponents_you_have_value,
    parse_filter_comparison_tokens,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::conditionals::{parse_mana_symbol_group, parse_subtype_word};
use super::dispatch_inner::trim_edge_punctuation;
pub(crate) use super::zone_counter_helpers::{
    apply_exile_subject_hand_owner_context, apply_exile_subject_owner_context,
    apply_shuffle_subject_graveyard_owner_context, exile_subject_owner_filter,
    merge_it_match_filter_into_target, parse_convert, parse_half_starting_life_total_value,
    parse_transform, split_until_source_leaves_tail, target_object_filter_mut,
};

type ZoneHandlerNormalizedWords<'a> = TokenWordView<'a>;
use super::for_each_helpers::{
    self, parse_get_for_each_count_value, parse_get_modifier_values_with_tail,
};
use super::search_library::parse_restriction_duration;
use super::sentence_primitives::find_color_choice_phrase;

const SHARE_REL_PREFIXES: &[&[&str]] = &[&["that", "share"], &["that", "shares"]];
const POWER_OF_PREFIXES: &[&[&str]] = &[&["the", "power", "of"], &["power", "of"]];
const TOUGHNESS_OF_PREFIXES: &[&[&str]] = &[&["the", "toughness", "of"], &["toughness", "of"]];
const TEXT_BOXES_OF_PREFIXES: &[&[&str]] =
    &[&["the", "text", "boxes", "of"], &["text", "boxes", "of"]];
const YOUR_PREFIXES: &[&[&str]] = &[&["your"]];
const THEIR_PREFIXES: &[&[&str]] = &[&["their"]];
const THAT_PLAYER_PREFIXES: &[&[&str]] = &[
    &["that", "player"],
    &["that", "players"],
    &["his", "or", "her"],
];
const TARGET_PLAYER_PREFIXES: &[&[&str]] = &[&["target", "player"], &["target", "players"]];
const TARGET_OPPONENT_PREFIXES: &[&[&str]] = &[&["target", "opponent"], &["target", "opponents"]];
const TURN_PREFIXES: &[&[&str]] = &[&["that", "turn"], &["turn"]];
const EMBLEM_WITH_PREFIXES: &[&[&str]] = &[&["an", "emblem", "with"], &["emblem", "with"]];
const ADDITIONAL_PREFIXES: &[&[&str]] = &[&["an", "additional"], &["additional"]];
const ATTACHED_REFERENCE_PREFIXES: &[&[&str]] = &[
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "land"],
    &["that", "artifact"],
    &["that", "enchantment"],
];
const ALL_CARD_PREFIXES: &[&[&str]] = &[&["all", "cards"], &["all", "card"]];
const UP_TO_PREFIXES: &[&[&str]] = &[&["up", "to"]];
const THAT_MANY_PREFIXES: &[&[&str]] = &[&["that", "many"]];
const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"]];
const TARGET_BLOCKED_PREFIXES: &[&[&str]] = &[&["target", "blocked"]];
const ANY_AMOUNT_OF_PREFIXES: &[&[&str]] = &[&["any", "amount", "of"]];
const LIFE_TOTALS_PREFIXES: &[&[&str]] = &[&["life", "totals"]];

#[path = "exile_actions.rs"]
mod exile_actions;
#[path = "mana_actions.rs"]
mod mana_actions;
#[path = "misc_actions.rs"]
mod misc_actions;
#[path = "remove_destroy.rs"]
mod remove_destroy;
#[path = "return_exchange.rs"]
mod return_exchange;
#[path = "sacrifice_discard.rs"]
mod sacrifice_discard;
#[path = "tap_actions.rs"]
mod tap_actions;

pub(crate) use exile_actions::*;
pub(crate) use mana_actions::*;
pub(crate) use misc_actions::*;
pub(crate) use remove_destroy::*;
pub(crate) use return_exchange::*;
pub(crate) use sacrifice_discard::*;
pub(crate) use tap_actions::*;

#[cfg(test)]
#[path = "zone_handlers_tests.rs"]
mod tests;
