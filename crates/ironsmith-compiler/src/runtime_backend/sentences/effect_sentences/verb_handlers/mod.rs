#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, ControlDurationAst, EffectAst, EventValueSpec, IT_TAG, ObjectRefAst,
    OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst, TagKey, TargetAst,
    TextSpan, Verb,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::mana::ManaSymbol;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::super::activation_and_restrictions::{
    find_word_sequence_start, parse_devotion_value_from_add_clause,
};
use super::super::activation_helpers::parse_add_mana;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    parse_trailing_if_predicate_lexed, parse_trailing_instead_if_predicate_lexed,
    parse_who_player_predicate_lexed, split_trailing_if_clause_lexed,
    split_trailing_unless_clause_lexed,
};
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
    parse_where_x_value_clause,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index, find_window_by, rfind_index, slice_contains, slice_starts_with, str_strip_suffix,
};
use super::super::util::{
    is_article, is_source_reference_words, mana_pips_from_token, parse_card_type,
    parse_mana_symbol, parse_number, parse_number_word_u32, parse_target_count_range_prefix,
    parse_target_phrase, parse_value, parse_value_expr_words, replace_unbound_x_with_value,
    span_from_tokens, token_index_for_word_index, trim_commas, value_contains_unbound_x, words,
    wrap_target_count,
};
use super::super::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_counters_on_reference_value,
    parse_equal_to_number_of_filter_plus_or_minus_fixed_value,
    parse_equal_to_number_of_filter_value, parse_equal_to_number_of_opponents_you_have_value,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::creation_handlers::{parse_create, parse_investigate};
use super::for_each_helpers::parse_who_did_this_way_predicate;
use super::sentence_primitives::try_build_unless;
use super::zone_counter_helpers::{parse_convert, parse_put_counters, parse_transform};
use super::zone_handlers::{
    DelayedReturnTimingAst, parse_become, parse_delayed_return_timing_words, parse_destroy,
    parse_discard, parse_exchange, parse_exile, parse_flip, parse_get,
    parse_graveyard_owner_prefix, parse_mill, parse_pay, parse_regenerate, parse_remove,
    parse_return, parse_roll, parse_sacrifice, parse_scry, parse_skip, parse_surveil, parse_switch,
    parse_tap, parse_untap, wrap_return_with_delayed_timing,
};
include!("resource_verbs.rs");
include!("combat_verbs.rs");
include!("zone_move_verbs.rs");
include!("counter_stat_verbs.rs");
include!("control_copy_attach_verbs.rs");
