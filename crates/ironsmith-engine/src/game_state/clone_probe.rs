//! Diagnostic: what one `GameState::clone` costs, field by field.
use super::*;

impl GameState {
    /// Nanoseconds per clone (clone + drop) of each field, most expensive first.
    pub fn clone_cost_report(&self, iterations: u32) -> Vec<(&'static str, u128)> {
        let mut out = Vec::new();
        macro_rules! time {
            ($name:expr, $field:expr) => {{
                let started = std::time::Instant::now();
                for _ in 0..iterations {
                    let copy = std::hint::black_box(&$field).clone();
                    drop(std::hint::black_box(copy));
                }
                out.push(($name, started.elapsed().as_nanos() / u128::from(iterations.max(1))));
            }};
        }
        time!("<whole GameState>", *self);
        time!("named_ability_counter_programs", self.named_ability_counter_programs);
        time!("object_groups", self.object_groups);
        time!("players", self.players);
        time!("object_store", self.object_store);
        time!("stack", self.stack);
        time!("turn", self.turn);
        time!("turn_store", self.turn_store);
        time!("effect_store", self.effect_store);
        time!("planechase", self.planechase);
        time!("vanguard", self.vanguard);
        time!("archenemy", self.archenemy);
        time!("attractions", self.attractions);
        time!("conspiracy", self.conspiracy);
        time!("free_for_all", self.free_for_all);
        time!("grand_melee", self.grand_melee);
        time!("emperor", self.emperor);
        time!("team_vs_team", self.team_vs_team);
        time!("two_headed_giant", self.two_headed_giant);
        time!("alternating_teams", self.alternating_teams);
        time!("range_of_influence", self.range_of_influence);
        time!("attack_direction", self.attack_direction);
        time!("chosen_defending_player", self.chosen_defending_player);
        time!("departed_battle_protectors", self.departed_battle_protectors);
        time!("fragmented_loop_history", self.fragmented_loop_history);
        time!("teams", self.teams);
        time!("shared_team_turns", self.shared_team_turns);
        time!("subgame_parent", self.subgame_parent);
        time!("initial_object_states", self.initial_object_states);
        time!("pending_restart_battlefield_entries", self.pending_restart_battlefield_entries);
        time!("choice_store", self.choice_store);
        time!("metadata", self.metadata);
        time!("zone_view_changes", self.zone_view_changes);
        time!("zone_revisions", self.zone_revisions);
        time!("combat", self.combat);
        time!("citys_blessing", self.citys_blessing);
        time!("named_player_designations", self.named_player_designations);
        time!("enduring_story", self.enduring_story);
        time!("battlefield_flags", self.battlefield_flags);
        time!("control_baseline", self.control_baseline);
        time!("combat_transients", self.combat_transients);
        time!("auxiliary_tracking", self.auxiliary_tracking);
        time!("object_annotations", self.object_annotations);
        time!("cast_permission_flags", self.cast_permission_flags);
        time!("commander_tracking", self.commander_tracking);
        time!("exile_tracking", self.exile_tracking);
        time!("runtime_cache", self.runtime_cache);
        time!("custom_subtypes", self.custom_subtypes);
        time!("effect_store.continuous_effects", self.effect_store.continuous_effects);
        time!("effect_store.replacement_effects", self.effect_store.replacement_effects);
        time!("effect_store.prevention_effects", self.effect_store.prevention_effects);
        time!("effect_store.cant_effects", self.effect_store.cant_effects);
        time!("effect_store.mana_spend_effects", self.effect_store.mana_spend_effects);
        time!("effect_store.activation_cost_alternatives", self.effect_store.activation_cost_alternatives);
        time!("effect_store.named_action_uses", self.effect_store.named_action_uses);
        time!("effect_store.source_object_collections", self.effect_store.source_object_collections);
        time!("effect_store.delayed_triggers", self.effect_store.delayed_triggers);
        time!("effect_store.pending_trigger_events", self.effect_store.pending_trigger_events);
        time!("effect_store.departure_hand_reveal_frames", self.effect_store.departure_hand_reveal_frames);
        time!("effect_store.pending_hand_reveal_windows", self.effect_store.pending_hand_reveal_windows);
        time!("effect_store.hand_reveal_events_captured", self.effect_store.hand_reveal_events_captured);
        time!("effect_store.instruction_result_records", self.effect_store.instruction_result_records);
        time!("effect_store.action_observation_records", self.effect_store.action_observation_records);
        time!("effect_store.retained_original_observation_scopes", self.effect_store.retained_original_observation_scopes);
        time!("effect_store.pending_trigger_entries", self.effect_store.pending_trigger_entries);
        time!("effect_store.pending_trigger_class_choices", self.effect_store.pending_trigger_class_choices);
        time!("effect_store.pending_reflexive_triggers", self.effect_store.pending_reflexive_triggers);
        time!("effect_store.matched_outcome_events", self.effect_store.matched_outcome_events);
        time!("effect_store.active_state_trigger_conditions", self.effect_store.active_state_trigger_conditions);
        time!("effect_store.pending_replacement_choice", self.effect_store.pending_replacement_choice);
        time!("effect_store.grant_registry", self.effect_store.grant_registry);
        time!("effect_store.library_top_revisions", self.effect_store.library_top_revisions);
        time!("effect_store.granted_mana_abilities", self.effect_store.granted_mana_abilities);
        time!("effect_store.temporary_spell_cost_reductions", self.effect_store.temporary_spell_cost_reductions);
        time!("effect_store.temporary_spell_ability_grants", self.effect_store.temporary_spell_ability_grants);
        time!("effect_store.repeatable_mana_payment_actions", self.effect_store.repeatable_mana_payment_actions);
        time!("effect_store.intrinsic_immediate_actions_used", self.effect_store.intrinsic_immediate_actions_used);
        time!("effect_store.restriction_effects", self.effect_store.restriction_effects);
        time!("effect_store.goad_effects", self.effect_store.goad_effects);
        time!("effect_store.attack_player_requirements", self.effect_store.attack_player_requirements);
        time!("effect_store.next_combat_attack_requirements", self.effect_store.next_combat_attack_requirements);
        time!("effect_store.goad_cleared_at", self.effect_store.goad_cleared_at);
        time!("effect_store.entry_spent_objects", self.effect_store.entry_spent_objects);
        time!("turn_store.resolved_spell_names", self.turn_store.resolved_spell_names);
        time!("turn_store.turn_order", self.turn_store.turn_order);
        time!("turn_store.turns_taken", self.turn_store.turns_taken);
        time!("turn_store.untapped_at_turn_start", self.turn_store.untapped_at_turn_start);
        time!("turn_store.skipped_steps", self.turn_store.skipped_steps);
        time!("turn_store.cards_drawn_this_draw_step", self.turn_store.cards_drawn_this_draw_step);
        time!("turn_store.turn_history", self.turn_store.turn_history);
        time!("turn_store.previous_turn_history", self.turn_store.previous_turn_history);
        time!("turn_store.last_turn_history_by_player", self.turn_store.last_turn_history_by_player);
        time!("turn_store.action_history", self.turn_store.action_history);
        time!("turn_store.creature_last_attacked_turn", self.turn_store.creature_last_attacked_turn);
        time!("turn_store.creature_last_blocked_turn", self.turn_store.creature_last_blocked_turn);
        time!("turn_store.departed_player_history", self.turn_store.departed_player_history);
        time!("turn_store.hand_sizes_at_turn_start", self.turn_store.hand_sizes_at_turn_start);
        time!("turn_store.entered_battlefield_last_turn", self.turn_store.entered_battlefield_last_turn);
        time!("turn_store.grant_cast_uses_this_turn", self.turn_store.grant_cast_uses_this_turn);
        time!("turn_store.cast_spell_lki", self.turn_store.cast_spell_lki);
        time!("turn_store.exhaust_abilities_activated", self.turn_store.exhaust_abilities_activated);
        time!("turn_store.ability_activations_per_object", self.turn_store.ability_activations_per_object);
        time!("turn_store.activation_mana_colors", self.turn_store.activation_mana_colors);
        time!("turn_store.ability_activations_by_actor", self.turn_store.ability_activations_by_actor);
        time!("turn_store.activation_group_receipts", self.turn_store.activation_group_receipts);
        time!("turn_store.triggers_fired_per_object", self.turn_store.triggers_fired_per_object);
        time!("turn_store.combat_damage_assignments", self.turn_store.combat_damage_assignments);
        time!("turn_store.came_under_control_since_last_upkeep", self.turn_store.came_under_control_since_last_upkeep);
        time!("turn_store.echo_eligible_this_upkeep", self.turn_store.echo_eligible_this_upkeep);
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out
    }
}
