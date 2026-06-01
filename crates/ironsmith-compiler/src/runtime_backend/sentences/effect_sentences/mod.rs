#![allow(unused_imports)]

#[allow(unused_imports)]
use self::sentence_helpers::*;
#[allow(unused_imports)]
#[cfg(test)]
use super::keyword_static::parse_value_binding_clause;
#[allow(unused_imports)]
use super::object_filters::parse_object_filter;
#[allow(unused_imports)]
use super::util::{
    is_source_reference_words, parse_choice_count_before_target_prefix,
    parse_counter_type_from_tokens, parse_filter_counter_constraint_words, parse_subject,
    parse_target_phrase, parse_value, span_from_tokens,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, OwnedLexToken, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst,
    TextSpan,
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

mod bundle_rules;
mod chain_carry;
mod clause_dispatch;
pub(crate) mod clause_pattern_helpers;
mod clause_primitives;
pub(crate) mod conditionals;
mod consult_family;
mod creation_handlers;
mod dispatch_entry;
mod dispatch_inner;
mod divvy;
mod fanout_family;
mod for_each_helpers;
mod gain_ability;
mod lex_chain_helpers;
mod looked_cards_family;
mod next_spell_family;
mod search_library;
mod sentence_helpers;
mod sentence_registry;
mod sentence_unsupported;
mod sequence_rules;
mod subject_verb_primitives;
mod subject_verb_special_recognizers;
mod verb_dispatch;
mod verb_handlers;
mod zone_counter_helpers;
mod zone_handlers;

pub(crate) use super::grammar::effects::parse_cant_effect_sentence;
pub(crate) use super::grammar::effects::parse_cant_effect_sentence_with_grammar_entrypoint_lexed as parse_cant_effect_sentence_lexed;
pub(crate) use chain_carry::parse_effect_chain_with_subject_verb_primitives_lexed;
pub(crate) use chain_carry::*;
pub(crate) use chain_carry::{
    collapse_token_copy_end_of_combat_exile_followup,
    collapse_token_copy_next_end_step_exile_followup,
    collapse_token_copy_next_end_step_sacrifice_followup, find_verb,
    maybe_apply_carried_player_with_clause, parse_effect_chain, parse_effect_chain_inner,
    parse_effect_chain_with_subject_verb_primitives, parse_effect_clause_with_trailing_if,
    parse_leading_player_may, parse_or_action_clause, remove_first_word, remove_through_first_word,
};
pub(crate) use clause_dispatch::parse_effect_clause_lexed;
pub(crate) use clause_dispatch::*;
pub(crate) use clause_primitives::{
    parse_attack_or_block_this_turn_if_able_clause, parse_attack_this_turn_if_able_clause,
    parse_must_be_blocked_if_able_clause, parse_must_block_if_able_clause, run_clause_primitives,
};
#[cfg(test)]
pub(crate) use conditionals::parse_conditional_sentence_lexed;
pub(crate) use conditionals::*;
pub(crate) use dispatch_entry::SentenceInput;
pub(crate) use dispatch_entry::*;
pub(crate) use dispatch_inner::*;
pub(crate) use fanout_family::{
    parse_compound_damage_fanout_sentence, parse_same_name_gets_fanout_sentence,
    parse_same_name_target_fanout_sentence, parse_shared_color_target_fanout_sentence,
};
pub(crate) use gain_ability::*;
pub(crate) use lex_chain_helpers::find_verb_lexed;
pub(crate) use search_library::parse_search_library_sentence;
pub(crate) use search_library::parse_search_library_sentence as parse_search_library_sentence_lexed;
pub(crate) use search_library::*;
#[cfg(test)]
pub(crate) use sentence_helpers::{
    parse_half_starting_life_total_value, parse_sentence_put_multiple_counters_on_target,
};
#[cfg(test)]
pub(crate) use sequence_rules::try_parse_subject_verb_sequence_rule;
pub(crate) use subject_verb_primitives::*;

fn subject_pronoun_player_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject? {
        SubjectAst::Player(PlayerAst::You) => Some(PlayerFilter::You),
        SubjectAst::Player(PlayerAst::Any) => Some(PlayerFilter::Any),
        SubjectAst::Player(PlayerAst::Target) => Some(PlayerFilter::target_player()),
        SubjectAst::Player(PlayerAst::TargetOpponent) => Some(PlayerFilter::target_opponent()),
        SubjectAst::Player(PlayerAst::Opponent) => Some(PlayerFilter::Opponent),
        SubjectAst::Player(PlayerAst::Defending) => Some(PlayerFilter::Defending),
        SubjectAst::Player(PlayerAst::Attacking) => Some(PlayerFilter::Attacking),
        SubjectAst::Player(PlayerAst::MostCardsInHand) => Some(PlayerFilter::MostCardsInHand),
        SubjectAst::Player(PlayerAst::MostLifeTied) => Some(PlayerFilter::MostLifeTied),
        SubjectAst::Player(PlayerAst::LowestLifeTied) => Some(PlayerFilter::LowestLifeTied),
        _ => None,
    }
}

pub(super) fn bind_iterated_player_pronouns_to_subject(
    filter: &mut ObjectFilter,
    subject: Option<SubjectAst>,
) {
    let Some(replacement) = subject_pronoun_player_filter(subject) else {
        return;
    };
    bind_iterated_player_pronouns_in_filter(filter, &replacement);
}

fn bind_iterated_player_pronouns_in_filter(filter: &mut ObjectFilter, replacement: &PlayerFilter) {
    if let Some(controller) = &mut filter.controller {
        bind_iterated_player_pronouns_in_player_filter(controller, replacement);
    }
    if let Some(owner) = &mut filter.owner {
        bind_iterated_player_pronouns_in_player_filter(owner, replacement);
    }
    if let Some(cast_by) = &mut filter.cast_by {
        bind_iterated_player_pronouns_in_player_filter(cast_by, replacement);
    }
    if let Some(targets_player) = &mut filter.targets_player {
        bind_iterated_player_pronouns_in_player_filter(targets_player, replacement);
    }
    if let Some(targets_only_player) = &mut filter.targets_only_player {
        bind_iterated_player_pronouns_in_player_filter(targets_only_player, replacement);
    }
    if let Some(attacking_player) = &mut filter.attacking_player_or_planeswalker_controlled_by {
        bind_iterated_player_pronouns_in_player_filter(attacking_player, replacement);
    }
    if let Some(attached_to_player) = &mut filter.attached_to_player {
        bind_iterated_player_pronouns_in_player_filter(attached_to_player, replacement);
    }
    if let Some(entered_controller) = &mut filter.entered_battlefield_controller {
        bind_iterated_player_pronouns_in_player_filter(entered_controller, replacement);
    }
    if let Some(damaged_player) = &mut filter.dealt_damage_to_player_this_turn {
        bind_iterated_player_pronouns_in_player_filter(damaged_player, replacement);
    }
    if let Some(targets_object) = &mut filter.targets_object {
        bind_iterated_player_pronouns_in_filter(targets_object, replacement);
    }
    if let Some(targets_only_object) = &mut filter.targets_only_object {
        bind_iterated_player_pronouns_in_filter(targets_only_object, replacement);
    }
    for branch in &mut filter.any_of {
        bind_iterated_player_pronouns_in_filter(branch, replacement);
    }
}

fn bind_iterated_player_pronouns_in_player_filter(
    filter: &mut PlayerFilter,
    replacement: &PlayerFilter,
) {
    match filter {
        PlayerFilter::IteratedPlayer => *filter = replacement.clone(),
        PlayerFilter::Target(inner)
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { base: inner, .. }
        | PlayerFilter::HasMoreLifeThanYou { base: inner }
        | PlayerFilter::MaxSpeed { base: inner, .. } => {
            bind_iterated_player_pronouns_in_player_filter(inner, replacement);
        }
        PlayerFilter::Excluding { base, excluded } => {
            bind_iterated_player_pronouns_in_player_filter(base, replacement);
            bind_iterated_player_pronouns_in_player_filter(excluded, replacement);
        }
        _ => {}
    }
}
