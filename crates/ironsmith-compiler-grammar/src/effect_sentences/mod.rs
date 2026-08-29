use self::sentence_helpers::*;
use super::object_filters::parse_object_filter;
use super::util::{parse_target_phrase, span_from_tokens};
use crate::cards::builders::OwnedLexToken;
use crate::target::ObjectFilter;

pub fn parse_artifact_enchantment_or_token_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let words = crate::lexer::token_word_refs(tokens);
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

pub(crate) fn parse_complete_each_player_return_with_additional_counter(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::cards::builders::EffectAst>>, crate::cards::builders::CardTextError> {
    subject_verb_primitives::parse_sentence_each_player_return_with_additional_counter(
        subject_verb_primitives::SubjectVerbPrimitiveClause::new(tokens),
    )
}

pub(crate) fn parse_complete_each_player_reveal_partition(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::cards::builders::EffectAst>>, crate::cards::builders::CardTextError> {
    subject_verb_primitives::parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
        subject_verb_primitives::SubjectVerbPrimitiveClause::new(tokens),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCopyFollowup {
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
pub(crate) use clause_dispatch::parse_get_pump_clause;
pub mod clause_pattern_helpers;
mod clause_primitives;
pub(crate) use clause_primitives::{
    parse_anaphoric_object_deals_damage_clause, parse_deal_damage_equal_to_power_clause,
};
pub mod conditionals;
mod consult_family;
mod creation_handlers;
mod delegated_partition_programs;
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

pub use super::grammar::effects::parse_cant_effect_sentence;
pub use super::grammar::effects::parse_cant_effect_sentence_with_grammar_entrypoint_lexed as parse_cant_effect_sentence_lexed;
pub(crate) use bundle_rules::parse_complete_kicked_search_replacement_bundle;
#[cfg(test)]
pub use bundle_rules::parse_typed_effect_bundle_lexed;
pub use chain_carry::parse_effect_chain_with_subject_verb_primitives_lexed;
pub use chain_carry::*;
pub use chain_carry::{
    find_verb, parse_effect_chain, parse_effect_chain_inner,
    parse_effect_chain_with_subject_verb_primitives, parse_effect_clause_with_trailing_if,
};
pub use clause_dispatch::parse_effect_clause_lexed;
pub use clause_dispatch::*;
pub use clause_pattern_helpers::parse_choose_target_prelude_sentence;
#[cfg(test)]
pub use conditionals::parse_conditional_sentence_lexed;
pub use conditionals::*;
pub use creation_handlers::{
    attach_inline_token_granted_abilities_to_last_create,
    attach_mixed_pronoun_token_rules_to_last_create, lower_complete_simple_create_shape,
    mixed_pronoun_token_rule_list, parse_create, reconcile_inline_copy_self_replacement_grants,
};
pub use dispatch_entry::SentenceInput;
pub(crate) use dispatch_entry::parse_complete_simple_subject_verb_sentence;
pub use dispatch_entry::*;
pub use dispatch_inner::*;
pub use fanout_family::{
    parse_compound_damage_fanout_sentence, parse_same_name_gets_fanout_sentence,
    parse_same_name_target_fanout_sentence, parse_serial_target_pt_modifiers_sentence,
    parse_shared_color_target_fanout_sentence,
};
pub use for_each_helpers::parse_get_for_each_count_value;
pub use gain_ability::*;
pub use search_library::parse_search_library_sentence;
pub use search_library::parse_search_library_sentence as parse_search_library_sentence_lexed;
pub use search_library::*;
pub use sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_optional_battlefield_then_conditional_remainder;
pub use sequence_rules::generic_subject_verb_sequences::exile_permission_followups::parse_dynamic_exile_top_then_play_for_as_long_as_exiled;
pub use sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_partition_face_down_then_filtered_permission;
pub use sequence_rules::generic_subject_verb_sequences::parse_destroy_then_no_regeneration_sequence;
pub use sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_look_at_players_hand_then_may_cast_from_those_cards;
pub use sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_tempting_offer_copy_spell_sequence;
pub use sequence_rules::try_parse_document_program;
pub use subject_verb_primitives::*;
pub use verb_handlers::parse_exiled_with_source_move_surface;
pub use verb_handlers::{
    damage_clause_has_terminal_unpreventable_rider, mark_damage_ast_unpreventable,
};
pub use zone_counter_helpers::target_object_filter_mut;
#[cfg(test)]
pub use zone_counter_helpers::{
    parse_half_starting_life_total_value, parse_sentence_put_multiple_counters_on_target,
};
pub use zone_handlers::parse_destroy;
