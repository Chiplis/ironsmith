use self::sentence_helpers::*;
use super::object_filters::parse_object_filter;
use super::util::{parse_target_phrase, span_from_tokens};
use crate::cards::builders::OwnedLexToken;
use crate::target::ObjectFilter;

pub(crate) fn parse_artifact_enchantment_or_token_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words
        .iter()
        .any(|word| *word == "token" || *word == "tokens")
        || !words
            .iter()
            .any(|word| *word == "artifact" || *word == "artifacts")
        || !words
            .iter()
            .any(|word| *word == "enchantment" || *word == "enchantments")
    {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        ObjectFilter::artifact(),
        ObjectFilter::enchantment(),
        ObjectFilter::default().token(),
    ];
    Some(filter)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCopyFollowup {
    HasHaste(crate::effect::TokenCopyReferenceSurface),
    GainHasteUntilEndOfTurn(crate::effect::TokenCopyReferenceSurface),
    EnterTappedAndAttacking,
    EnterTappedAndAttackingThatPlayer,
    SacrificeAtNextEndStep(crate::effect::TokenCopyReferenceSurface),
    SacrificeAtNextUpkeep,
    ExileAtNextEndStep(crate::effect::TokenCopyReferenceSurface),
    ExileAtEndOfCombat(crate::effect::TokenCopyReferenceSurface),
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
mod optional_companion_fanout;
mod player_subject_sequences;
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
pub(crate) use bundle_rules::parse_typed_effect_bundle_lexed;
pub(crate) use chain_carry::parse_effect_chain_with_subject_verb_primitives_lexed;
pub(crate) use chain_carry::*;
pub(crate) use chain_carry::{
    find_verb, parse_effect_chain, parse_effect_chain_inner,
    parse_effect_chain_with_subject_verb_primitives, parse_effect_clause_with_trailing_if,
};
pub(crate) use clause_dispatch::parse_effect_clause_lexed;
pub(crate) use clause_dispatch::*;
#[cfg(test)]
pub(crate) use conditionals::parse_conditional_sentence_lexed;
pub(crate) use conditionals::*;
pub(crate) use dispatch_entry::SentenceInput;
pub(crate) use dispatch_entry::*;
pub(crate) use dispatch_inner::*;
pub(crate) use fanout_family::{
    parse_compound_damage_fanout_sentence, parse_same_name_gets_fanout_sentence,
    parse_same_name_target_fanout_sentence, parse_serial_target_pt_modifiers_sentence,
    parse_shared_color_target_fanout_sentence,
};
pub(crate) use gain_ability::*;
pub(crate) use search_library::parse_search_library_sentence;
pub(crate) use search_library::parse_search_library_sentence as parse_search_library_sentence_lexed;
pub(crate) use search_library::*;
#[cfg(test)]
pub(crate) use sequence_rules::try_parse_subject_verb_sequence_rule;
pub(crate) use subject_verb_primitives::*;
#[cfg(test)]
pub(crate) use zone_counter_helpers::{
    parse_half_starting_life_total_value, parse_sentence_put_multiple_counters_on_target,
};
