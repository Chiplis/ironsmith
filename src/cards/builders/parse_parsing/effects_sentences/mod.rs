#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, TagKey, TargetAst, TextSpan, Token, is_article,
    is_source_reference_words, parse_color, parse_effect_clause, parse_keyword_mechanic_clause,
    parse_object_filter, parse_subject, parse_target_phrase, parse_value, span_from_tokens,
    token_index_for_word_index, words,
};
#[allow(unused_imports)]
use crate::cards::builders::parse_parsing::{
    apply_exile_subject_hand_owner_context, find_color_choice_phrase, parse_connive_clause,
    parse_counter_descriptor, parse_counter_target_count_prefix, parse_counter_type_from_tokens,
    parse_for_each_targeted_object_subject, parse_get_modifier_values_with_tail, parse_number,
    parse_pt_modifier_values, parse_put_counters, parse_sentence_put_multiple_counters_on_target,
    parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    parse_where_x_value_clause, parser_trace, parser_trace_enabled, split_on_and, split_on_comma,
};
#[allow(unused_imports)]
use crate::effect::{ChoiceCount, Value};
#[allow(unused_imports)]
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
#[allow(unused_imports)]
use crate::types::{CardType, Subtype};
#[allow(unused_imports)]
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCopyFollowup {
    HasHaste,
    GainHasteUntilEndOfTurn,
    SacrificeAtNextEndStep,
    ExileAtNextEndStep,
    ExileAtEndOfCombat,
    SacrificeAtEndOfCombat,
}

mod chain_carry;
mod conditionals;
mod dispatch_entry;
mod dispatch_inner;
mod gain_ability;
mod search_library;
mod sentence_primitives;

pub(crate) use chain_carry::*;
pub(crate) use conditionals::*;
pub(crate) use dispatch_entry::*;
pub(crate) use dispatch_inner::*;
pub(crate) use gain_ability::*;
pub(crate) use search_library::*;
pub(crate) use sentence_primitives::*;
