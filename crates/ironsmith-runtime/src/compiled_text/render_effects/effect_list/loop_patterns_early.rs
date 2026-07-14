{
        if filtered[idx]
            .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
            .is_some_and(|tag| is_implicit_reference_tag(tag.tag.as_str()))
        {
            idx += 1;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_exile_with_counters_then_gain_suspend(&[
                filtered[idx].clone(),
                filtered[idx + 1].clone(),
                filtered[idx + 2].clone(),
            ])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_source_exile_with_counters_pair(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_put_counters_then_gain_suspend(&[
                filtered[idx].clone(),
                filtered[idx + 1].clone(),
            ])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_return_from_graveyard_with_counters(&[
                filtered[idx].clone(),
                filtered[idx + 1].clone(),
            ])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(exiled_tag) = tagged_exile_effect_tag(filtered[idx])
            && copied_spell_targets_tag(filtered[idx + 1], exiled_tag)
            && may_cast_copy_targets_tag(filtered[idx + 2], exiled_tag)
        {
            let exile_text = describe_effect(filtered[idx])
                .replace(" in your graveyard", " from your graveyard");
            parts.push(format!("{exile_text}. Copy it. You may cast the copy"));
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_look_at_hand_top_and_face_down_creatures(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_exile_source_and_unless_pays_target(&[filtered[idx], filtered[idx + 1]])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_target_groups_then_random_destroy(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(compact) = describe_untap_gain_control_then_haste(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(compact) = describe_reveal_hand_choose_move(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(compact) = describe_reveal_hand_choose_discard(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(choose_player) =
                filtered[idx].downcast_ref::<crate::effects::ChoosePlayerEffect>()
            && let Some(add_mana) =
                filtered[idx + 1].downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
            && choose_player.chooser == PlayerFilter::You
            && choose_player.filter == PlayerFilter::Any
            && matches!(add_mana.player, PlayerFilter::TaggedPlayer(_))
        {
            let amount = match add_mana.amount {
                Value::Fixed(2) => "two".to_string(),
                _ => describe_value(&add_mana.amount),
            };
            parts.push(format!(
                "choose a player. That player adds {amount} mana of any one color they choose"
            ));
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(ticket) =
                filtered[idx].downcast_ref::<crate::effects::TicketCountersEffect>()
            && let Some(may) = filtered[idx + 1].downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) = describe_ticket_then_may_put_sticker(ticket, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(discard) = filtered[idx].downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachObject>()
            && let Some(compact) = describe_discard_then_for_each_discarded(discard, for_each)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }






































        // Look at top N + optional single exile pick from the looked set +
        // remainder to library bottom + cast window for the exiled card.












        if let Some((rendered, consumed)) =
            render_look_reveal_choice_to_hand_rest_graveyard(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_self_look_reorder_then_may_shuffle(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) = render_look_reveal_repeated_choices(&filtered[idx..]) {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_look_choose_reveal_to_hand_rest_bottom(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let [look_effect, choose_effect, move_effect, remainder_effect, ..] = &filtered[idx..]
            && let Some(look_at_top) =
                look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(remainder) = remainder_effect
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
            && let Some(rendered) = describe_looked_up_to_one_top_rest_bottom(
                look_at_top,
                choose,
                move_effect,
                remainder,
            )
        {
            parts.push(rendered);
            idx += 4;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_three_way_looked_card_partition(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_conditional_looked_hand_partition(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_look_may_sacrifice_select_battlefield_rest_bottom(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_look_exile_face_down_rest_graveyard_then_cast(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_looked_card_selected_partition(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_look_may_exile_from_among_rest_bottom_cast(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) = describe_look_may_move_one_rest_bottom(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) = describe_look_move_counted_rest_bottom(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            render_look_at_top_count_override_with_conditional(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) = render_reveal_top_choose_to_hand(&filtered[idx..]) {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = describe_consult_may_cast_remainder_bottom_sequence(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = describe_consult_exile_may_cast_rest_bottom_sequence(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = describe_consult_reveal_put_battlefield_then_bottom(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = render_consult_reveal_put_hand_then_bottom(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }

        if idx + 4 < filtered.len()
            && let Some(rendered) = describe_choose_name_exile_top_consult_hand_rest_exile(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
                filtered[idx + 3],
                filtered[idx + 4],
            ])
        {
            parts.push(rendered);
            idx += 5;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = render_consult_reveal_put_hand_rest_exile(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = render_consult_reveal_put_hand_rest_graveyard(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            ])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(apply) = apply_continuous_for_compaction(filtered[idx])
            && let Some(destroy) = downcast_destroy(filtered[idx + 1])
            && let Some(rendered) =
                render_remove_abilities_then_destroy_matching_creatures(apply, destroy)
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(rendered) =
                describe_exile_split_pile_opponent_choice_bundle(&filtered[idx..idx + 4])
        {
            parts.push(rendered);
            idx += 4;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) = describe_creature_pile_destroy_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_graveyard_creature_pile_exile_return_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) = describe_player_exile_controlled_creature_and_graveyard_bundle(
                &filtered[idx..idx + 3],
            )
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
}
