#![allow(unused_imports)]

#[allow(unused_imports)]
use self::sentence_helpers::*;
#[allow(unused_imports)]
use super::keyword_static::parse_where_x_value_clause;
#[allow(unused_imports)]
use super::object_filters::parse_object_filter;
#[allow(unused_imports)]
use super::util::{
    is_source_reference_words, parse_counter_type_from_tokens, parse_subject, parse_target_phrase,
    parse_value, span_from_tokens,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, OwnedLexToken, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, TagKey, TargetAst, TextSpan,
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
    EnterTappedAndAttacking,
    SacrificeAtNextEndStep,
    ExileAtNextEndStep,
    ExileAtEndOfCombat,
    SacrificeAtEndOfCombat,
}

mod chain_carry;
mod clause_dispatch;
pub(crate) mod clause_pattern_helpers;
mod clause_primitives;
pub(crate) mod conditionals;
mod creation_handlers;
mod dispatch_entry;
mod dispatch_inner;
mod divvy;
mod for_each_helpers;
mod gain_ability;
mod lex_chain_helpers;
mod search_library;
mod sentence_helpers;
mod sentence_primitives;
mod verb_dispatch;
mod verb_handlers;
mod zone_counter_helpers;
mod zone_handlers;

pub(crate) use super::grammar::effects::parse_cant_effect_sentence;
pub(crate) use super::grammar::effects::parse_cant_effect_sentence_with_grammar_entrypoint_lexed as parse_cant_effect_sentence_lexed;
pub(crate) use super::grammar::effects::parse_conditional_sentence_lexed;
pub(crate) use super::grammar::effects::parse_search_library_sentence;
pub(crate) use super::grammar::effects::parse_search_library_sentence_with_grammar_entrypoint_lexed as parse_search_library_sentence_lexed;
pub(crate) use chain_carry::parse_effect_chain_with_sentence_primitives_lexed;
pub(crate) use chain_carry::*;
pub(crate) use chain_carry::{
    collapse_token_copy_end_of_combat_exile_followup,
    collapse_token_copy_next_end_step_exile_followup,
    collapse_token_copy_next_end_step_sacrifice_followup, find_verb,
    maybe_apply_carried_player_with_clause, parse_effect_chain, parse_effect_chain_inner,
    parse_effect_chain_with_sentence_primitives, parse_effect_clause_with_trailing_if,
    parse_leading_player_may, parse_or_action_clause, remove_first_word, remove_through_first_word,
};
pub(crate) use clause_dispatch::parse_effect_clause_lexed;
pub(crate) use clause_dispatch::*;
pub(crate) use clause_primitives::{
    parse_attack_or_block_this_turn_if_able_clause, parse_attack_this_turn_if_able_clause,
    parse_must_be_blocked_if_able_clause, parse_must_block_if_able_clause, run_clause_primitives,
};
pub(crate) use conditionals::*;
pub(crate) use dispatch_entry::*;
pub(crate) use dispatch_inner::*;
pub(crate) use gain_ability::*;
pub(crate) use lex_chain_helpers::find_verb_lexed;
pub(crate) use search_library::*;
#[cfg(test)]
pub(crate) use sentence_helpers::{
    parse_half_starting_life_total_value, parse_sentence_put_multiple_counters_on_target,
};
pub(crate) use sentence_primitives::*;
