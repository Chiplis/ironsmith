#![allow(unused_imports)]

pub(crate) use super::super::activation_and_restrictions::{
    append_token_reminder_to_last_create_effect, build_may_cast_tagged_effect,
    effect_creates_any_token, effect_creates_eldrazi_spawn_or_scion, find_negation_span,
    is_activate_only_restriction_sentence, is_generic_token_reminder_sentence,
    is_round_up_each_time_sentence, is_simple_copy_reference_sentence,
    is_spawn_scion_token_mana_reminder, is_trigger_only_restriction_sentence, normalize_cant_words,
    parse_ability_phrase, parse_activated_line, parse_cant_restrictions,
    parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
    parse_choose_creature_type_then_become_type, parse_may_cast_it_sentence,
    parse_sentence_exile_that_token_when_source_leaves,
    parse_sentence_sacrifice_source_when_that_token_leaves,
    parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    parse_subject_object_filter, parse_target_player_chooses_then_other_cant_block,
    parse_you_choose_player_clause, starts_with_target_indicator, strip_embedded_token_rules_text,
    target_ast_to_object_filter,
};
pub(crate) use super::super::keyword_static::{
    parse_ability_line, parse_pt_modifier_values, reject_unimplemented_keyword_actions,
};
pub(crate) use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
    parse_cast_spells_as_though_they_had_flash_clause, parse_permission_clause_spec,
    parse_unsupported_play_cast_permission_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
pub(crate) use super::super::rule_engine::{
    ClauseView, RULE_SHAPE_STARTS_IF, RuleDef, RuleIndex, UnsupportedDiagnoser, UnsupportedRuleDef,
};
pub(crate) use super::super::util::{
    classify_instead_followup_text, helper_tag_for_tokens, is_until_end_of_turn,
    parse_mana_symbol_word_flexible, parse_number, parser_trace, parser_trace_enabled,
    replace_unbound_x_with_value, split_on_comma, split_on_comma_or_semicolon, split_on_period,
    starts_with_until_end_of_turn, value_contains_unbound_x,
};
pub(crate) use super::chain_carry::{
    collapse_token_copy_end_of_combat_exile_followup,
    collapse_token_copy_next_end_step_exile_followup, explicit_player_for_carry, find_verb,
    maybe_apply_carried_player, maybe_apply_carried_player_with_clause, parse_effect_chain,
    parse_effect_chain_inner,
};
pub(crate) use super::clause_pattern_helpers::{
    extract_subject_player, parse_can_attack_as_though_no_defender_clause,
    parse_can_block_additional_creature_this_turn_clause, parse_choose_target_and_verb_clause,
    parse_choose_target_prelude_sentence, parse_connive_clause, parse_copy_spell_clause,
    parse_distribute_counters_clause, parse_double_counters_clause, parse_keyword_mechanic_clause,
    parse_prevent_all_damage_clause, parse_prevent_next_damage_clause,
    parse_prevent_next_time_damage_sentence, parse_redirect_next_damage_sentence,
    parse_verb_first_clause, parse_win_the_game_clause,
};
pub(crate) use super::conditionals::{parse_conditional_sentence_lexed, parse_predicate};
pub(crate) use super::conditionals::{parse_predicate_lexed, split_leading_result_prefix_lexed};
pub(crate) use super::dispatch_entry::{
    apply_where_x_to_damage_amounts, parse_token_copy_followup_sentence_lexed,
    replace_unbound_x_in_effects_anywhere,
};
pub(crate) use super::dispatch_inner::{
    is_exile_that_token_at_end_of_combat, is_exile_that_token_at_end_of_combat_lexed,
    is_sacrifice_that_token_at_end_of_combat, is_sacrifice_that_token_at_end_of_combat_lexed,
    parse_token_copy_modifier_sentence_lexed,
};
pub(crate) use super::for_each_helpers::{
    has_demonstrative_object_reference, is_mana_replacement_clause_words,
    is_mana_trigger_additional_clause_words, is_target_player_dealt_damage_by_this_turn_subject,
    parse_for_each_object_subject, parse_for_each_opponent_clause, parse_for_each_player_clause,
    parse_for_each_target_players_clause, parse_for_each_targeted_object_subject,
    parse_get_for_each_count_value, parse_get_modifier_values_with_tail,
    parse_has_base_power_clause, parse_has_base_power_toughness_clause,
};
pub(crate) use super::lex_chain_helpers::{
    has_effect_head_without_verb, is_token_creation_context, segment_has_effect_head,
    split_effect_chain_on_and, split_segments_on_comma_effect_head, split_segments_on_comma_then,
    starts_with_inline_token_rules_tail, strip_leading_instead_prefix,
};
pub(crate) use super::search_library::{
    normalize_search_library_filter, parse_cant_effect_sentence_lexed, parse_restriction_duration,
    parse_search_library_sentence_lexed,
};
pub(crate) use super::sentence_primitives::{
    POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX, POST_CONDITIONAL_SENTENCE_PRIMITIVES,
    PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX, PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
    parse_sentence_exile_source_with_counters, parse_sentence_exile_source_with_counters_lexed,
    parse_sentence_put_onto_battlefield_with_counters_on_it,
    parse_sentence_put_onto_battlefield_with_counters_on_it_lexed,
    parse_sentence_return_with_counters_on_it, parse_sentence_return_with_counters_on_it_lexed,
    run_sentence_primitives_lexed, try_build_unless,
};
pub(crate) use super::zone_counter_helpers::{
    apply_exile_subject_hand_owner_context, apply_shuffle_subject_graveyard_owner_context,
    parse_counter_descriptor, parse_counter_target_count_prefix,
    parse_half_starting_life_total_value, parse_put_counters,
    parse_sentence_put_multiple_counters_on_target, parse_transform,
    split_until_source_leaves_tail, target_object_filter_mut,
};
