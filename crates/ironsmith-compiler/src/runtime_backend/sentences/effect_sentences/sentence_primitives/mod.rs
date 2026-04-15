use super::super::activation_and_restrictions::activated_line_core::{
    contains_word_sequence, find_word_sequence_start,
};
use super::super::activation_and_restrictions::choice_object_clauses::{
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause,
};
use super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed;
use super::super::grammar::primitives::{
    self as grammar, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_period,
};
use super::super::keyword_static::parse_where_x_value_clause;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::parse_object_filter;
use super::super::rule_engine::{
    LexClauseView, LexRuleDef, LexRuleHeadHint, LexRuleHintIndex, LexRuleIndex,
    build_lex_rule_hint_index,
};
use super::super::token_primitives::{
    find_index, find_window_by, iter_contains, lexed_head_words, rfind_index, slice_contains,
    slice_ends_with, slice_starts_with, split_lexed_once_on_comma_then, str_strip_suffix,
};
use super::super::util::{
    is_article, is_source_reference_words, mana_pips_from_token, parse_card_type, parse_color,
    parse_counter_type_from_tokens, parse_subject, token_index_for_word_index, words,
};
use super::super::util::{parse_target_phrase, parse_value, span_from_tokens};
use super::dispatch_inner::merge_filters;
use super::search_library::parse_restriction_duration;
use super::sentence_helpers::*;
use super::verb_handlers::parse_half_rounded_down_draw_count_words;
use super::zone_counter_helpers::parse_convert;
#[allow(unused_imports)]
use super::{
    bind_implicit_player_context, parse_after_turn_sentence, parse_become_clause,
    parse_cant_effect_sentence, parse_delayed_until_next_end_step_sentence,
    parse_delayed_when_that_dies_this_turn_sentence, parse_destroy_or_exile_all_split_sentence,
    parse_each_player_choose_and_sacrifice_rest,
    parse_each_player_put_permanent_cards_exiled_with_source_sentence, parse_earthbend_sentence,
    parse_effect_chain, parse_effect_chain_inner, parse_effect_chain_lexed, parse_effect_clause,
    parse_effect_sentence_lexed, parse_enchant_sentence,
    parse_exile_hand_and_graveyard_bundle_sentence, parse_exile_instead_of_graveyard_sentence,
    parse_exile_then_return_same_object_sentence, parse_exile_up_to_one_each_target_type_sentence,
    parse_for_each_counter_removed_sentence, parse_for_each_destroyed_this_way_sentence,
    parse_for_each_exiled_this_way_sentence, parse_for_each_opponent_doesnt,
    parse_for_each_player_doesnt, parse_for_each_put_into_graveyard_this_way_sentence,
    parse_for_each_vote_clause, parse_gain_ability_sentence, parse_gain_ability_to_source_sentence,
    parse_gain_life_equal_to_age_sentence, parse_gain_life_equal_to_power_sentence,
    parse_gain_x_plus_life_sentence, parse_look_at_hand_sentence,
    parse_look_at_top_then_exile_one_sentence, parse_mana_symbol, parse_monstrosity_sentence,
    parse_play_from_graveyard_sentence, parse_prevent_damage_sentence,
    parse_same_name_gets_fanout_sentence, parse_same_name_target_fanout_sentence,
    parse_search_library_sentence, parse_sentence_counter_target_spell_if_it_was_kicked,
    parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    parse_sentence_delayed_trigger_this_turn,
    parse_sentence_exile_target_creature_with_greatest_power,
    parse_shared_color_target_fanout_sentence, parse_shuffle_graveyard_into_library_sentence,
    parse_shuffle_object_into_library_sentence, parse_subtype_word, parse_take_extra_turn_sentence,
    parse_target_player_exiles_creature_and_graveyard_sentence, parse_vote_extra_sentence,
    parse_vote_start_sentence, parse_you_and_each_opponent_voted_with_you_sentence, trim_commas,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, TagKey, TargetAst, TextSpan,
};
#[allow(unused_imports)]
use crate::effect::{ChoiceCount, Until, Value};
use crate::mana::ManaSymbol;
#[allow(unused_imports)]
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
#[allow(unused_imports)]
use crate::types::{CardType, Subtype};
#[allow(unused_imports)]
use crate::zone::Zone;
use std::cell::OnceCell;
use std::sync::LazyLock;
use winnow::Parser as _;

#[path = "choice_damage_family.rs"]
mod choice_damage_family;
mod combat_and_damage_family;
mod counter_marker_family;
mod delayed_step_family;
mod mechanic_marker_family;
mod registry;
mod token_copy_control_family;

pub(crate) use choice_damage_family::*;
pub(crate) use combat_and_damage_family::*;
pub(crate) use counter_marker_family::*;
pub(crate) use delayed_step_family::*;
pub(crate) use mechanic_marker_family::*;
pub(crate) use registry::*;
pub(crate) use token_copy_control_family::*;
