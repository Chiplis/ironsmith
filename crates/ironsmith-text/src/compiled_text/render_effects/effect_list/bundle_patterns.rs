{



















































































    if let Some(compact) = describe_filtered_future_exile_delayed_return_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) =
        describe_mass_creature_change_graveyard_exile_future_replacement_bundle(&filtered)
    {
        return compact;
    }
    if let Some(compact) = describe_exile_split_pile_opponent_choice_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_top_opponent_split_you_choose_pile_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) =
        describe_exiled_collection_opponent_split_you_choose_pile_bundle(&filtered)
    {
        return compact;
    }
    if let Some(compact) = describe_creature_pile_destroy_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_graveyard_creature_pile_exile_return_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_player_exile_controlled_creature_and_graveyard_bundle(&filtered)
    {
        return compact;
    }
    if let Some(compact) = describe_filtered_mill_then_draw_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_choose_tap_conditional_freeze_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_countered_spell_exile_replacement_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_damage_and_die_replacement_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_compound_damage_regeneration_exile_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_tagged_die_exile_replacement_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_exile_target_search_same_name_exile_shuffle_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_graveyard_exile_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_choose_shuffle_into_library_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_exile_same_name_search_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_tempting_offer_creature_return_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_mill_return_land_else_counter_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_dynamic_pt_token_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_power_cards_for_mana_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_top_hand_or_graveyard_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_each_player_choose_unselected_bounce_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_grant_keyword_and_unblockable_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_return_creature_mana_value_scry_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_exchange_control_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_graveyard_mana_ladder_return_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) =
        describe_effect_list_linked_graveyard_choices_then_may_return_bundle(&filtered)
    {
        return compact;
    }
    if let Some(compact) = describe_random_hand_reveal_damage_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_random_hand_reveal_life_loss_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_random_hand_reveal_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_self_unblockable_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_source_pump_unblockable_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_target_pump_unblockable_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_declared_target_for_each_pump_unblockable_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_tap_freeze_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_target_freeze_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = describe_reveal_top_to_hand_bundle(&filtered) {
        return compact;
    }
    if let Some(compact) = render_search_reveal_opponent_choose_rest_bundle(&filtered) {
        return compact;
    }

    if effects.len() == 2
        && let Some(target_only) = effects[0].downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(damage) =
            unwrap_tag_wrappers(&effects[1]).downcast_ref::<crate::effects::DealDamageEffect>()
        && let Some(rendered) = describe_target_only_then_damage_that_player(target_only, damage)
    {
        return rendered;
    }

    if effects.len() == 2
        && let Some(target_only) = effects[0].downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(create_token) =
            unwrap_tag_wrappers(&effects[1]).downcast_ref::<crate::effects::CreateTokenEffect>()
        && let Some(rendered) =
            describe_target_only_then_create_token_count(target_only, create_token)
    {
        return rendered;
    }







}
