{
    if let Some(compact) = describe_target_player_draw_exile_then_copy_result(effects) {
        return compact;
    }

    if let Some(compact) = describe_face_down_pile_then_manifest(effects) {
        return compact;
    }

    if let Some(compact) = describe_player_chosen_attachment(effects) {
        return compact;
    }

    if let Some(compact) = describe_may_choose_graveyard_then_return(effects) {
        return compact;
    }

    if let Some(compact) = describe_search_reveal_conditional_battlefield_or_hand(effects) {
        return compact;
    }
    if let Some(compact) = describe_turn_source_exiled_face_up_then_lose_mana_value(effects) {
        return compact;
    }
    if let Some(compact) = describe_source_exiled_creature_may_battlefield_else_hand(effects) {
        return compact;
    }

    if let Some(compact) = describe_targeted_card_set_total_mana_value_then_return(effects) {
        return compact;
    }
    if let Some(compact) = describe_may_exile_one_from_triggered_set_then_cast(effects) {
        return compact;
    }

    if let Some(compact) =
        describe_consult_reveal_put_battlefield_then_shuffle_effects(effects)
    {
        return compact;
    }
    if let Some(compact) = describe_pay_life_reveal_hand_choose_exile_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_pre_clause_structural_effect_list(effects) {
        return compact;
    }
    if let Some(compact) =
        describe_target_player_cast_and_creatures_attack_restrictions(effects)
    {
        return compact;
    }
    if let Some(compact) =
        describe_tagged_continuous_then_counter_conditional_draw(&raw_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_attach_all_enchanting_target_to_same_controller(effects) {
        return compact;
    }
    if let Some(compact) = describe_redundant_target_only_pair(effects) {
        return compact;
    }
    if effects.len() > 2
        && let Some(prefix) = describe_redundant_target_only_pair(&effects[..2])
    {
        let suffix = describe_effect_list(&effects[2..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some(compact) = describe_exile_then_choose_delayed_leaves(effects) {
        return compact;
    }
    if let Some(compact) = describe_triggering_object_coordinated_damage(effects) {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_target_continuous_fanout_pair(first, second)
            .or_else(|| describe_target_prevention_fanout_pair(first, second))
    {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_linked_same_source_damage_pair(first, second)
    {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_target_creature_damage_fanout_pair(first, second)
    {
        return compact;
    }
    if effects.len() >= 2
        && let Some(compact) =
            describe_target_same_name_action_fanout_pair(&effects[0], &effects[1])
    {
        if effects.len() == 2 {
            return compact;
        }
        let suffix = describe_effect_list(&effects[2..]);
        return format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if effects.len() >= 2
        && let Some(prefix) = describe_gain_control_then_untap_structural(&effects[..2])
    {
        if effects.len() == 2 {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[2..]);
        let suffix = normalize_imperative_you_clause(suffix.trim_end_matches('.'));
        return format!("{prefix}. {}", capitalize_first(&suffix));
    }







    if let Some(compact) =
        describe_consult_choose_any_number_to_battlefield_rest_bottom(&raw_effects)
    {
        return compact;
    }
    if let Some(compact) =
        describe_shuffle_reveal_repeated_permanent_groups_rest_bottom(&raw_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_target_modifications_then_exile_top_play(effects) {
        return compact;
    }
    if let Some(compact) = describe_optional_sticker_aura_return_attach_sequence(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_attached_to_source_sacrifice_sequence(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_tagged_token_copy_then_sacrifice(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_triggering_control_draw_else_lose(effects) {
        return compact;
    }
    if let [may_effect, shuffle_effect] = effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
        && may.decider.is_none()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_may_search_choose_for_each_with_shuffle(may, shuffle)
    {
        return compact;
    }
    if let Some(compact) = describe_return_as_aura_with_granted_abilities(effects) {
        return compact;
    }
    if let Some(compact) = describe_mixed_move_to_exile_then_exile_all_list(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_source_and_target(effects) {
        return compact;
    }
    if let Some(compact) = describe_target_permanent_shuffle_reveal_permanent_card(effects) {
        return compact;
    }
    if effects.len() > 3
        && let Some(prefix) = describe_choose_top_exile_then_play_structural(&effects[..3])
    {
        let rest = lowercase_first(describe_effect_list(&effects[3..]).trim_end_matches('.'));
        return format!("{prefix}. Then {rest}");
    }
    if let Some(compact) = describe_choose_top_exile_then_play_structural(effects) {
        return compact;
    }
    if effects.len() >= 3
        && let Some(compact) =
            describe_consult_may_cast_remainder_bottom_sequence(&raw_effects[..3])
    {
        if effects.len() == 3 {
            return compact;
        }
        let rest = lowercase_first(describe_effect_list(&effects[3..]).trim_end_matches('.'));
        return format!("{compact}. {rest}");
    }
    if effects.len() > 3
        && let Some(suffix) =
            describe_choose_top_exile_then_play_structural(&effects[effects.len() - 3..])
    {
        let prefix = describe_effect_list(&effects[..effects.len() - 3]);
        return format!("{}. {suffix}", prefix.trim_end_matches('.'));
    }
    if let Some(compact) = describe_chosen_creatures_blessing_additional_combat_clause(effects) {
        return compact;
    }
    if let Some(compact) = describe_gain_life_shuffle_source_and_graveyard(effects) {
        return compact;
    }
    if let Some(compact) = describe_untap_triggering_then_remove_from_combat(effects) {
        return compact;
    }
    if let Some(compact) = describe_remove_counter_then_no_counters_conditional(effects) {
        return compact;
    }
    if let [first, second] = raw_effects.as_slice()
        && let Some(compact) = describe_put_counters_then_untap_them(first, second)
    {
        return compact;
    }
    if let Some(compact) = describe_return_from_graveyard_with_counters(effects) {
        return compact;
    }
    if let Some(compact) = describe_oath_of_ghouls_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_countered_spell_same_name_search_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_counter_and_damage_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_put_counters_and_add_mana_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_countered_spell_exile_with_counters_gain_suspend(effects) {
        return compact;
    }
    if let Some(compact) = describe_destroy_all_groups_then_draw_for_destroyed(effects) {
        return compact;
    }
    if let Some(compact) = describe_countered_spell_controller_consult_cast_shuffle(effects) {
        return compact;
    }
    if let Some(compact) = describe_damage_each_then_tap_damaged_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_source_and_attacking_nonflying_creature(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_source_and_target(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_two_tap_then_unattach_equipment_sequence(effects) {
        return compact;
    }
    if let [target_effect, unattach_effect] = raw_effects.as_slice()
        && let Some(compact) =
            describe_target_then_unattach_all_equipment(target_effect, unattach_effect)
    {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_random_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_sacrifice_then_reflexive_trigger_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_sacrifice_then_source_damage_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_sacrifice_then_sacrificed_conditional_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_gain_control_create_token_attach_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_create_token_then_grant_same_tag(effects) {
        return compact;
    }
    if let Some(compact) = describe_moved_object_haste_delayed_cleanup(effects) {
        return compact;
    }
    if let Some(compact) = describe_pump_all_then_grant_same_filter(effects) {
        return compact;
    }
    if let Some(compact) = describe_put_counters_then_grant_same_filter(effects) {
        return compact;
    }
    if let Some(compact) = describe_draw_count_then_grant_same_filter(effects) {
        return compact;
    }
    if let Some(compact) = describe_continuous_choose_attach_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_return_each_subtype_card_from_your_graveyard(effects) {
        return compact;
    }
    if let Some(compact) = describe_random_choose_then_destroy_rest(effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_two_filters_then_discard(&raw_effects) {
        return compact;
    }
    if raw_effects.len() == 4
        && let Some(target_only) = raw_effects[0]
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(look) = raw_effects[1].downcast_ref::<crate::effects::LookAtHandEffect>()
        && let Some(compact) = describe_reveal_hand_choose_discard(&raw_effects[1..])
    {
        let ordinary_subject = capitalize_first(&describe_choose_spec(&look.target));
        let target_subject = capitalize_first(&describe_choose_spec(&ChooseSpec::target(
            target_only.target.clone(),
        )));
        if let Some(rest) = compact.strip_prefix(&ordinary_subject) {
            return format!("{target_subject}{rest}");
        }
    }
    if raw_effects.len() == 4
        && raw_effects[0]
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some()
        && let Some(compact) =
            describe_reveal_hand_subset_choose_then_discard(&raw_effects[1..])
    {
        return compact;
    }
    if let Some(compact) = describe_discard_reveal_hand_choose_discard_chosen(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_color_reveal_hand_discard_that_color(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_target_player_choose_hand_top_library_any_order(effects) {
        return compact;
    }
    if let Some(compact) = describe_hand_choose_then_library_placement(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_graveyard_or_hand_exile(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_scry(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_adventure_move(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_gain_toughness_then_discard(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_look_hand_choose_then_discard_or_exile(&raw_effects) {
        return compact;
    }
    if let [create_effect, manifest_effect] = effects
        && let Some(create) =
            unwrap_tag_wrappers(create_effect).downcast_ref::<crate::effects::CreateTokenEffect>()
        && let Some(manifest) = unwrap_tag_wrappers(manifest_effect)
            .downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
        && let Some(compact) = describe_create_token_and_manifest_top_card(create, manifest)
    {
        return compact;
    }
    if let [exile_top_effect, grant_play_effect, grant_free_cast_effect] = effects
        && let Some(exile_top) = unwrap_tag_wrappers(exile_top_effect)
            .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && let Some(grant_play) = unwrap_tag_wrappers(grant_play_effect)
            .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(grant_free_cast) = unwrap_tag_wrappers(grant_free_cast_effect)
            .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
        )
        && let Some(compact) =
            describe_exile_top_then_play_without_paying_mana(exile_top, grant_play, grant_free_cast)
    {
        return compact;
    }

    if let Some(compact) = describe_gain_life_then_put_same_x_counters(effects) {
        return compact;
    }
    if let Some(compact) = describe_gain_life_then_distribute_creatures_died_counters(effects) {
        return compact;
    }
    if let Some(compact) = describe_power_damage_exchange_clause(effects) {
        return capitalize_first(&compact);
    }
    if let Some(compact) = describe_turn_start_hand_condition_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_draw_then_for_players_choose_exile(effects) {
        return compact;
    }
    if let Some(compact) = describe_lose_life_then_endure(effects) {
        return compact;
    }
    if let Some(compact) = describe_tagged_target_then_conditional_action(effects) {
        return compact;
    }
    if let Some(compact) = describe_tagged_for_each_then_apply_continuous(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_target_set_then_apply_continuous(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_target_set_then_return_to_hand(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_two_move_one_put_counters_on_other(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_same_controller_sacrifice_one_return_other(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_exiled_cards_exile_library_put_chosen_on_top(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_two_sacrifice_one_return_other(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_sacrifice_power_damage_each(effects) {
        return compact;
    }
    if let Some(compact) = describe_tag_attached_tap_then_become_monarch(effects) {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_destroy_then_color_conditional(first, second)
    {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_destroy_then_temporary_cant_attack_block(first, second)
    {
        return compact;
    }
    if let [destroy_effect, search_effect, shuffle_effect] = effects
        && let Some(compact) =
            describe_destroy_then_search_target_opponent_to_graveyard_then_shuffle(
                destroy_effect,
                search_effect,
                shuffle_effect,
            )
    {
        return compact;
    }
    if let [destroy_effect, search_effect, shuffle_effect] = effects
        && let Some(compact) = describe_destroyed_land_controller_basic_search_then_player_shuffle(
            destroy_effect,
            search_effect,
            shuffle_effect,
        )
    {
        return compact;
    }
    if let [search_effect, shuffle_effect] = effects
        && let Some(compact) =
            describe_destroyed_land_basic_search_then_player_shuffle(search_effect, shuffle_effect)
    {
        return compact;
    }
    if let [first, second] = effects
        && let Some(tagged) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(cant) = second.downcast_ref::<crate::effects::CantEffect>()
        && let Some(compact) = describe_tagged_target_then_cant_restriction(tagged, cant)
    {
        return compact;
    }
    if let Some(compact) = describe_vote_with_received_vote_followups(effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_top_to_hand_then_lose_mana_value_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_copy_tagged_then_may_cast_copy(effects) {
        return compact;
    }
    if let Some(compact) = describe_may_draw_then_source_enchanted_additional_draw(effects) {
        return compact;
    }
    if effects.len() > 2
        && let Some(compact_tail) =
            describe_copy_tagged_then_may_cast_copy(&effects[effects.len() - 2..])
    {
        let prefix = describe_effect_list(&effects[..effects.len() - 2]);
        if prefix.trim().is_empty() {
            return compact_tail;
        }
        return format!("{}. {compact_tail}", prefix.trim_end_matches('.'));
    }
    if let [for_players_effect, look_effect, grant_effect] = effects
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(look) = look_effect.downcast_ref::<crate::effects::LookAtObjectsEffect>()
        && let Some(grant) = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(compact) =
            describe_for_players_bottom_library_exile_then_look_cast(for_players, look, grant)
    {
        return compact;
    }
    if let Some(compact) = describe_each_opponent_exile_top_then_cast_until_eot_any_color(effects) {
        return compact;
    }
    if let Some(compact) = describe_id_backed_prior_action_count_consumer(effects) {
        return compact;
    }
    if let Some(compact) = describe_discard_then_draw_amount_sequence(effects) {
        return compact;
    }
    if let Some(compact) = describe_structural_multisentence_effect_list(effects) {
        return compact;
    }
    if let Some(compact) = describe_group_pump_then_conditional_untap(effects) {
        return compact;
    }
    if let Some(compact) = describe_roll_result_damage_then_random_source_attachment(effects) {
        return compact;
    }
    if let Some(compact) = describe_roll_die_then_scry_result(effects) {
        return compact;
    }
    if let Some(compact) = describe_roll_die_with_numeric_result_table(effects) {
        return compact;
    }

    if let Some(compact) = describe_choose_color_then_chosen_color_mana(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_revealed_cards_opponent_may_put_or_draw(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_kicked_additional_targets_put_counters(&raw_effects) {
        return compact;
    }




















    if let Some(compact) = describe_choose_then_mount_vehicle_become(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_tagged_effect_then_remove_all_counters(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_tagged_pump_then_conditional_keyword(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_return_all_face_down_then_become(&raw_effects) {
        return compact;
    }
    if let [first, rest @ ..] = raw_effects.as_slice()
        && let Some(compact) = describe_return_all_face_down_then_become(rest)
    {
        return format!(
            "{}. {compact}",
            describe_effect(first).trim_end_matches('.')
        );
    }
    if let Some(compact) = describe_return_then_conditional_animation(effects) {
        return compact;
    }
    if let Some(compact) =
        describe_immediate_life_gain_then_delayed_source_return(&raw_effects)
    {
        return compact;
    }


    if let Some(compact) = describe_tagged_counter_then_color_subtype_keyword(&raw_effects) {
        return compact;
    }
    if let Some(compact) =
        describe_return_with_inline_counter_and_static_followups(&raw_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_delayed_return_with_counter_and_static_followups(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_return_with_counter_and_static_followups(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_return_then_color_subtype_addition(&raw_effects) {
        return compact;
    }



    if let Some(compact) = describe_consult_reveal_put_battlefield_then_bottom(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_move_then_color_subtype_addition(&raw_effects) {
        return compact;
    }







    if let Some(compact) = describe_return_then_return_attached(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_put_onto_battlefield_attached(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_exile_then_incubate_count(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_clash_win_optional_top_replacement(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_energy_then_pay_any_then_destroy(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_energy_then_pay_any_then_create_paid_x_token(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_energy_then_pay_any_then_put_paid_counters(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_copy_then_may_cast_copy(&raw_effects) {
        return compact;
    }
    if raw_effects.len() == 3
        && let Some(copy_cast) = describe_copy_then_may_cast_copy(&raw_effects[1..])
    {
        return format!("{}. {copy_cast}", describe_effect(raw_effects[0]));
    }
    if let Some(compact) = describe_sequence_copy_then_may_cast(&raw_effects) {
        return compact;
    }
    if raw_effects.len() == 3
        && let Some(copy_cast) = describe_sequence_copy_then_may_cast(&raw_effects[1..])
    {
        return format!("{}. {copy_cast}", describe_effect(raw_effects[0]));
    }


    if let Some(compact) = describe_targeted_conditional_action_then_fight(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_two_distinct_targets_conditional_then_fight(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_two_distinct_targets_counter_then_fight(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_look_at_hand_top_and_face_down_creatures(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_reveal_hand_choose_move(&raw_effects) {
        return compact;
    }
    if raw_effects.first().is_some_and(|effect| {
        effect
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    }) && let Some(compact) = describe_reveal_hand_choose_move(&raw_effects[1..])
    {
        return compact;
    }






    if let Some(compact) = look_hand_choose_then_move_to_library(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_target_source_power_damage_to_controller(&raw_effects) {
        return compact;
    }

    if let Some(compact) =
        describe_choose_any_target_players_then_investigate_total_creatures(&raw_effects)
    {
        return compact;
    }





    if let Some(compact) = describe_target_only_then_exchange_control(&raw_effects) {
        return compact;
    }

    if let [
        draw_you_effect,
        draw_attacking_effect,
        lose_you_effect,
        lose_attacking_effect,
    ] = raw_effects.as_slice()
        && let Some(draw_you) = draw_you_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(draw_attacking) =
            draw_attacking_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(lose_you) = lose_you_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(lose_attacking) =
            lose_attacking_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(compact) = describe_you_and_attacking_player_draw_and_lose(
            draw_you,
            draw_attacking,
            lose_you,
            lose_attacking,
        )
    {
        return compact;
    }

    if let [draw_effect, target_effect, lose_effect] = raw_effects.as_slice()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(compact) = describe_target_player_draw_then_lose_life(draw, target_only, lose)
    {
        return compact;
    }
    if let [choose_effect, draw_effect, target_effect, lose_effect] = raw_effects.as_slice()
        && let Some(choose) =
            choose_effect.downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
        && choose.chooser == PlayerFilter::You
        && choose.excluded_subtypes.is_empty()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(compact) = describe_target_player_draw_then_lose_life(draw, target_only, lose)
    {
        return format!("Choose a creature type. {compact}");
    }

    if let [target_effect, lose_effect, gain_effect] = raw_effects.as_slice()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(gain) = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()
        && let Some(compact) =
            describe_target_player_lose_then_you_gain_life(target_only, lose, gain)
    {
        return compact;
    }

    if let Some(compact) = describe_damaged_player_reveal_choose_graveyard(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_target_opponent_create_tokens_with_count(&raw_effects) {
        return compact;
    }
    if let [sacrifice_effect, extra_turn_effect] = raw_effects.as_slice()
        && let Some(sacrifice) =
            sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()
        && matches!(sacrifice.target, ChooseSpec::Source)
        && let Some(extra_turn) =
            extra_turn_effect.downcast_ref::<crate::effects::ExtraTurnEffect>()
        && extra_turn.player == PlayerFilter::You
    {
        return format!(
            "Sacrifice {} and take an extra turn after this one",
            describe_choose_spec(&sacrifice.target)
        );
    }

}
