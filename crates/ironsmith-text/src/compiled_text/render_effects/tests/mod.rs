use super::*;
use crate::tag::TagKey;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

mod activation_restriction_surfaces;
mod anaphoric_opponent_actor_surfaces;
mod any_number_cast_surfaces;
mod attached_bundle_surfaces;
mod attached_conditional_otherwise_surfaces;
mod attached_must_block_registry_surfaces;
mod basic_land_type_surfaces;
mod bounded_x_payment_surfaces;
mod branch_scoped_collection_unions;
mod chosen_card_type_phase_out;
mod chosen_type_consult_remainder;
mod coin_flip_target_backrefs;
mod comma_then_surfaces;
mod conditional_entry_counter_surfaces;
mod conditional_fight_surfaces;
mod conditional_looked_entry_counter_surfaces;
mod consult_battlefield_remainder_surfaces;
mod consult_conditional_destination;
mod consult_revealed_partition;
mod copy_activated_registry_surfaces;
mod copy_cast_surfaces;
mod copy_spell_modifiers;
mod correlated_color_type_choice_surfaces;
mod correlated_created_result;
mod correlated_delegated_collections;
mod cost_effect_surfaces;
mod count_backref_surfaces;
mod counter_cast_origin;
mod countered_spell_replacement_surfaces;
mod created_token_copy_retarget;
mod day_black_sun_surfaces;
mod delayed_destroy_surfaces;
mod delegated_search_partition_surfaces;
mod destroy_consult_collection;
mod destroy_unless_dynamic_life_surfaces;
mod distinct_player_choices;
mod draw_step_search_player_surfaces;
mod draw_then_discard_unless_surfaces;
mod duration_scoped_delayed_triggers;
mod dynamic_token_count_surfaces;
mod each_any_number_counter_surfaces;
mod each_player_coordinated_draw_life;
mod each_player_return_counter_surfaces;
mod each_player_reveal_partition;
mod exact_mana_cost_saga_surfaces;
mod exile_top_play_boundaries;
mod explicit_you_coordinated_actions;
mod failed_action_and_aura_attach_surfaces;
mod gain_life_x_plus_surfaces;
mod goad_surfaces;
mod graveyard_exception_same_name_exile_surfaces;
mod hand_choice_shuffle_surfaces;
mod historical_block_reanimation;
mod historical_combat_relation;
mod historical_damage_recipients;
mod history_draw_surface;
mod kang_dynasty_surfaces;
mod keyword_compaction_surfaces;
mod land_animation_surfaces;
mod linked_counter_unless_surfaces;
mod looked_cloak_partition;
mod looked_exile_cast_surfaces;
mod lowest_life_tie_surfaces;
mod mana_spent_surfaces;
mod may_pay_sacrifice_surfaces;
mod movement_surfaces;
mod multi_zone_search_slots;
mod named_source_exile_surfaces;
mod next_cast_entry_counter_surfaces;
mod next_cleanup_surfaces;
mod overencumbered_surfaces;
mod oversimplify_surfaces;
mod participant_choice_phase_out;
mod participant_graveyard_choice_complement;
mod participant_loot_extremum;
mod per_player_failure_surfaces;
mod per_player_unless_alternative_costs;
mod phase_out_source_duration_surfaces;
mod prime_value_condition;
mod qualified_villainous_choice_surfaces;
mod quantified_unless_sacrifice;
mod quantifier_residual_cards;
mod quoted_token_conditional_surfaces;
mod reflexive_producer_surfaces;
mod refreshed_attachment_and_phantom_surfaces;
mod roll_result_surfaces;
mod same_name_fanout_surfaces;
mod search_conditional_destination_surfaces;
mod search_conditional_without_reveal;
mod search_equipment_attachment_surfaces;
mod second_sunrise;
mod serial_type_choice_surfaces;
mod shard_00;
mod shard_01;
mod shard_02;
mod shard_03;
mod source_attacked_battle_condition_surfaces;
mod source_plus_chosen_sacrifice_cost;
mod special_triggered_program_surfaces;
mod surface_residual_nothic_will_heartless;
mod synthetic_target_folding;
mod target_player_draw_exile_copy_surfaces;
mod target_player_each_cardinality;
mod target_threshold_self_replacement_surfaces;
mod targeted_graveyard_cast;
mod teferis_protection_surfaces;
mod temporal_graveyard_history;
mod temporary_escape_grant_surfaces;
mod token_copy_followup_surfaces;
mod token_definition_cleanup;
mod trailing_unless;

mod counter_placement_condition;
mod copied_end_step_ability;
mod search_shuffle_event;

mod conditional_quoted_grant;

mod attached_mixed_grants;

mod searched_player_shuffle;

mod revealed_hand_cast;

mod same_name_exile_investigate;

mod aggregate_choice_complement;
mod leading_duration_quoted_grant;
mod extraction_hand_draw;

mod hand_choice_reveal;

mod independent_exile_targets;

mod conditional_attachment_attack;

mod max_speed_source_bonus;

mod source_combat_unless;

mod source_block_unless;

mod source_counter_combat;

mod trailing_if_combat;

mod opposing_player_block;

mod relative_opponent_choice;

mod conditional_copy_vanishing;

mod vote_outcome_pairs;

mod exiled_card_token_watcher;

mod filtered_blocking_life_cost;

mod chosen_hand_free_cast;

mod aggregate_discount_temporary_grant;

mod kicker_entry_activated_grant;

mod attached_token_attack_contract;

mod counter_threshold_transform;

mod madness_total_cost;

mod animation_granted_land_count;

mod ferocious_before_fight;

mod named_self_exile_transform;

mod hand_remainder_shuffle;

mod initiative_and_token_coordination;

mod aura_combat_transform_restriction;

mod attack_keyword_progression;

mod damaged_target_player_or_controller;

mod paid_color_hand_discard;

mod combat_face_up_or_counter;

mod manifest_dread_counter_bundle;

mod unbound_x_cost_trigger;

mod lavabrink_counter_sacrifice;

mod biting_palm_reflexive;

mod scaled_free_cast;

mod conditional_damage_replacement;

mod starting_life_anthem;

mod damage_emblem_recipients;

mod source_damage_transform;

mod kicked_damage_replacement;

mod opponent_target_copy;

mod per_opponent_mill_payment;
mod chosen_permanent_exile_search;

mod conditional_draw_fallback;

mod paid_color_hand_damage;

mod chosen_die_entry_counters;

mod source_pump_keyword_target_debuff;

mod revealed_entry_count;

mod next_turn_flash_entry;

mod chosen_added_combat;

mod chosen_power_difference;
mod conditional_attacker_untap;
mod destroyed_controller_damage;

mod tapped_additional_cost;

mod processed_entry_counters;

mod self_subtype_defender;

mod revealed_land_modifier;
mod returned_aura_ability;
mod shared_type_copy;
mod source_power_combat_gate;
mod greatest_power_discount;

mod sticker_target_modifier;

mod snow_cast_entry;

mod base_plus_entry_counters;

mod conditional_attacker_bonus;
mod counter_gated_combat_prevention;
mod milled_card_source_copy;
mod destroyed_artifact_damage;

mod subtype_attack_group;

mod chosen_pair_sacrifice;

mod first_x_counter_discount;

mod chosen_type_compound_pump;

mod enchanted_repeat_cast_copy;

mod damage_target_type_condition;

mod attached_source_counter_release;

mod graveyard_cast_entry_counters;

mod void_self_cost;

mod delayed_targeted_damage_source;

mod melira_watched_permanent;

mod cast_trigger_hand_choice;
