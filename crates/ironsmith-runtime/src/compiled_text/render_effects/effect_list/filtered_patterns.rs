{

    if let Some(compact) = describe_source_counter_and_create(&filtered) {
        return compact;
    }
    if let [first, second] = filtered.as_slice()
        && let Some(compact) = describe_put_counters_then_untap_them(first, second)
    {
        return compact;
    }
    if let Some(compact) = describe_life_lock_and_protection_from_everything(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_player_protection_from_everything_pair(&filtered) {
        return compact;
    }

    if let Some(compact) = describe_discard_reveal_hand_choose_discard_chosen(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_two_filters_then_discard(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_move(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_choose_hand_then_reveal(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_sequence_copy_then_may_cast(&filtered) {
        return compact;
    }
    if filtered.len() == 3
        && let Some(copy_cast) = describe_sequence_copy_then_may_cast(&filtered[1..])
    {
        return format!("{}. {copy_cast}", describe_effect(filtered[0]));
    }
    if let Some(compact) = describe_reveal_hand_then_discard(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_exile_graveyard_reflexive_copy_artifact(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_may_choose_pay_for_each_then_untap_tagged(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_targeted_most_common_color_conditional_return_to_hand(&filtered)
    {
        return compact;
    }
    if let Some(compact) = describe_targeted_most_common_color_conditional_destroy(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_targeted_conditional_destroy(&filtered) {
        return compact;
    }


    if filtered.len() == 5
        && let Some(compact) = describe_search_two_split_hand_graveyard(&filtered)
    {
        return compact;
    }


    if let Some(compact) = describe_council_vote_winners_exile(&filtered) {
        return compact;
    }


    if let Some(compact) = describe_choose_player_add_mana(&filtered) {
        return compact;
    }










    if let Some(compact) = describe_put_counters_then_goad(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_put_counters_then_unblockable(&raw_effects) {
        return compact;
    }
    if let Some(compact) = describe_tap_defending_creature_then_goad(&filtered) {
        return compact;
    }



    if let Some(compact) = describe_targeted_haste_then_role(&filtered) {
        return compact;
    }


    if let Some(compact) = describe_tag_attached_sacrifice_then_create(&filtered) {
        return compact;
    }


    if let Some(compact) = describe_target_players_each_effects(&filtered) {
        return compact;
    }




    if let Some(compact) = describe_two_target_creature_exchange_or_fight(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_shape_anew_like_bundle(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_reveal_until_land_put_all_graveyard(&raw_effects) {
        return compact;
    }


    if let Some(compact) = describe_tap_for_mana_player_bonus_and_damage(&filtered) {
        return compact;
    }


    if let Some(compact) = describe_creature_secret_vote_with_default_draw(&filtered) {
        return compact;
    }


    if let Some(compact) = describe_each_player_choose_creature_destroy_others(&filtered) {
        return compact;
    }

    if filtered.len() == 2
        && let Some(compact) = describe_source_exile_with_counters_pair(filtered[0], filtered[1])
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) =
            describe_exile_it_then_return_all_to_battlefield(filtered[0], filtered[1])
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) =
            describe_source_sacrifice_then_return_source_exiled(filtered[0], filtered[1])
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) =
            describe_player_or_planeswalker_damage_then_controlled_creature_damage(
                filtered[0],
                filtered[1],
            )
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) = describe_phase_in_out_pair(filtered[0], filtered[1])
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(split_for_players) =
            filtered[0].downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(choice_for_players) =
            filtered[1].downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(compact) = describe_for_players_split_piles_then_choose_sacrifice_pair(
            split_for_players,
            choice_for_players,
        )
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) =
            describe_split_piles_then_choose_attack_or_block_restriction(filtered[0], filtered[1])
    {
        return compact;
    }
    if filtered.len() == 3
        && let Some(compact) = describe_look_hand_choose_then_discard_or_exile(&filtered)
    {
        return compact;
    }
    if filtered.len() == 4
        && let Some(compact) = describe_reveal_hand_choose_two_filters_then_discard(&filtered)
    {
        return compact;
    }
    if filtered.len() == 3
        && let Some(compact) = describe_reveal_hand_subset_choose_then_discard(&filtered)
    {
        return compact;
    }
    if filtered.len() == 4
        && (filtered[0]
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
            || filtered[0]
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some())
        && let Some(compact) = describe_reveal_hand_subset_choose_then_discard(&filtered[1..])
    {
        return compact;
    }
    if filtered.len() == 7
        && let Some(compact) = describe_reveal_top_two_optional_picks_rest_bottom(&filtered)
    {
        return compact;
    }
    if filtered.len() == 4
        && let Some(compact) = describe_hideaway_effects(&filtered)
    {
        return compact;
    }
    if filtered.len() == 6
        && let Some(compact) = describe_look_exile_one_rest_bottom_cast_else_hand(&filtered)
    {
        return compact;
    }
    if filtered.len() == 5
        && let Some(compact) = describe_exile_targets_opponent_piles_return_chosen(&filtered)
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) = describe_choose_x_permanents_create_x_copies(&filtered)
    {
        return compact;
    }
    if let Some(compact) = describe_counter_artifact_ability_destroy_source(&filtered)
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(choose) = filtered[0].downcast_ref::<crate::effects::ChooseCardTypeEffect>()
        && let Some(phase_out) = filtered[1].downcast_ref::<crate::effects::PhaseOutEffect>()
        && let Some(compact) = describe_choose_type_then_phase_out(choose, phase_out)
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(compact) =
            describe_damaged_player_gain_control_then_rewards(filtered[0], filtered[1])
    {
        return compact;
    }

    let visible_effects = filtered
        .iter()
        .copied()
        .filter(|effect| {
            let implicit_triggering_object = effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some_and(|tag| is_implicit_reference_tag(tag.tag.as_str()));
            let implicit_triggering_source = effect
                .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                .is_some_and(|tag| is_implicit_reference_tag(tag.tag.as_str()));
            !implicit_triggering_object && !implicit_triggering_source
        })
        .collect::<Vec<_>>();

    if visible_effects.len() == 2
        && let Some(compact) =
            describe_damaged_player_gain_control_then_rewards(visible_effects[0], visible_effects[1])
    {
        return compact;
    }

    if let Some(compact) = describe_choose_each_basic_land_type_then_destroy(&visible_effects) {
        return compact;
    }
    if let Some(compact) =
        describe_source_owner_shuffle_then_reveal_named_to_battlefield(&visible_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_add_mana_then_conditional_consult_hand_bottom(&visible_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_search_name_conditional_put_then_shuffle(&visible_effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_phase_then_skip_chosen_this_turn(&visible_effects) {
        return compact;
    }
    if let Some(compact) =
        describe_may_cast_target_graveyard_spell_then_exile_replacement(&visible_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_simple_create_token_bundle(&visible_effects) {
        return compact;
    }
    if let Some(compact) =
        describe_exile_creatures_consult_that_many_battlefield_shuffle(&visible_effects)
    {
        return compact;
    }
    if let Some(compact) =
        describe_choose_any_target_players_then_investigate_total_creatures(&visible_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_each_player_gain_life_and_draw_pair(&visible_effects) {
        return compact;
    }
    if let Some(compact) = describe_player_loses_life_and_discards_pair(&visible_effects) {
        return compact;
    }
    if let Some(compact) = describe_sacrifice_source_then_return_with_counters(&visible_effects) {
        return compact;
    }
    if let Some(compact) = describe_spell_mastery_reanimation_effects(&visible_effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_then_put_counter_on_each(&visible_effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_copy_spell_and_retarget_copy_to_chosen(&visible_effects)
    {
        return compact;
    }
    if let Some(compact) = describe_choose_same_controller_targets_then_sacrifice_one(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_exile_graveyard_then_same_name_library_exile_bundle(&filtered) {
        return compact;
    }

    if visible_effects.len() == 2
        && let Some(compact) = describe_for_players_choose_then_destroy_chosen_collection_pair(
            visible_effects[0],
            visible_effects[1],
        )
        .or_else(|| {
            let for_players = structural_unwrap_render_wrappers(visible_effects[0])
                .downcast_ref::<crate::effects::ForPlayersEffect>()?;
            let destroy = destroy_effect_for_choose_compaction(visible_effects[1])?;
            describe_for_players_may_choose_then_destroy_chosen(for_players, destroy)
        })
    {
        return compact;
    }
    if filtered.len() == 2
        && let Some(life_loss) = filtered[0].downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(return_to_hand) =
            filtered[1].downcast_ref::<crate::effects::ReturnToHandEffect>()
        && life_loss.player == ChooseSpec::Player(PlayerFilter::You)
        && return_to_hand.spec == ChooseSpec::Source
    {
        return format!(
            "You lose {} life and return this card to your hand",
            describe_value(&life_loss.amount)
        );
    }
    if filtered.len() == 2
        && let Some(choose) = filtered[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(conditional) = filtered[1].downcast_ref::<crate::effects::ConditionalEffect>()
        && let Some(compact) =
            describe_choose_then_color_matched_combat_prevention(choose, conditional)
    {
        return compact;
    }




    if let Some(compact) =
        describe_sacrifice_return_from_graveyard_then_exile_source_bundle(effects)
    {
        return compact;
    }


    if let Some(compact) = describe_dynamic_return_from_graveyard_bundle(effects, &filtered) {
        return compact;
    }












    if let Some(compact) = render_necromentia_shape(&raw_effects) {
        return compact;
    }
    if let Some(compact) = render_reveal_hand_choose_same_name_exile_shuffle(&raw_effects) {
        return compact;
    }
    if let Some(compact) = render_choose_name_search_same_name_exile_shuffle(&raw_effects) {
        return compact;
    }
    if let Some(compact) = render_optional_draw_then_sylvan_card_choice(&raw_effects) {
        return compact;
    }
}
