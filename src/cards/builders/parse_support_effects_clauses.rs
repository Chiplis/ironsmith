#[path = "parse_support_effects_clauses_clause_patterns.rs"]
mod clause_patterns;
#[path = "parse_support_effects_clauses_counters.rs"]
mod counters;
#[path = "parse_support_effects_clauses_creation.rs"]
mod creation;
#[path = "parse_support_effects_clauses_dispatch.rs"]
mod dispatch;
#[path = "parse_support_effects_clauses_for_each.rs"]
mod for_each;
#[path = "parse_support_effects_clauses_verb_handlers.rs"]
mod verb_handlers;
#[path = "parse_support_effects_clauses_zones.rs"]
mod zones;

pub(crate) use clause_patterns::{
    PermissionClauseSpec, PermissionLifetime, SubjectAst, extract_subject_player,
    parse_additional_land_plays_clause, parse_can_block_additional_creature_this_turn_clause,
    parse_cast_or_play_tagged_clause, parse_cast_spells_as_though_they_had_flash_clause,
    parse_choose_target_and_verb_clause, parse_choose_target_prelude_sentence,
    parse_connive_clause, parse_copy_spell_clause, parse_distribute_counters_clause,
    parse_double_counters_clause, parse_keyword_mechanic_clause, parse_permission_clause_spec,
    parse_prevent_next_time_damage_sentence, parse_redirect_next_damage_sentence, parse_subject,
    parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
    parse_unsupported_play_cast_permission_clause, parse_verb_first_clause,
    parse_win_the_game_clause,
};
pub(crate) use counters::{
    intern_counter_name, parse_counter_descriptor, parse_counter_target_count_prefix,
    parse_counter_type_from_tokens, parse_counter_type_word, parse_put_counters,
    parse_sentence_put_multiple_counters_on_target,
};
pub(crate) use creation::*;
pub(crate) use dispatch::*;
pub(crate) use for_each::{
    has_demonstrative_object_reference, is_mana_replacement_clause_words,
    is_mana_trigger_additional_clause_words,
    is_target_player_dealt_damage_by_this_turn_subject, parse_for_each_object_subject,
    parse_for_each_opponent_clause, parse_for_each_player_clause,
    parse_for_each_target_players_clause, parse_for_each_targeted_object_subject,
    parse_get_for_each_count_value, parse_get_modifier_values_with_tail,
    parse_has_base_power_clause, parse_has_base_power_toughness_clause,
    parse_who_did_this_way_predicate,
};
pub(crate) use verb_handlers::*;
pub(crate) use zones::*;
