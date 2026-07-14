{
        if idx + 6 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(hand_choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(matching_choose) =
                filtered[idx + 4].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(rest) =
                filtered[idx + 6].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_looked_one_hand_then_matching_to_zone_rest_graveyard(
                    look_at_top,
                    Some(reveal_top),
                    hand_choose,
                    filtered[idx + 3],
                    matching_choose,
                    filtered[idx + 5],
                    rest,
                )
        {
            parts.push(compact);
            idx += 7;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(hand_choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(matching_choose) =
                filtered[idx + 3].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(rest) =
                filtered[idx + 5].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_looked_one_hand_then_matching_to_zone_rest_graveyard(
                    look_at_top,
                    None,
                    hand_choose,
                    filtered[idx + 2],
                    matching_choose,
                    filtered[idx + 4],
                    rest,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(remainder) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
            && let Some(distribute) =
                filtered[idx + 4].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_looked_reveal_selection_rest_bottom_land_creature_split(
                    look_at_top,
                    choose,
                    reveal,
                    remainder,
                    distribute,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_damage_and_die_replacement_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(rendered) =
                describe_reveal_hand_choose_graveyard_exile_bundle(&filtered[idx..idx + 4])
        {
            parts.push(rendered);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_reveal_hand_choose_shuffle_into_library_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(rendered) =
                describe_reveal_hand_exile_same_name_search_bundle(&filtered[idx..idx + 6])
        {
            parts.push(rendered);
            idx += 6;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_tempting_offer_creature_return_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_mill_return_land_else_counter_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(rendered) =
                describe_mill_return_land_else_counter_bundle(&filtered[idx..idx + 4])
        {
            parts.push(rendered);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_prior_effect_dynamic_count_token_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) =
                describe_prior_effect_count_create_token_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) = describe_dynamic_pt_token_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) =
                describe_create_token_then_set_base_pt_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_reveal_power_cards_for_mana_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_reveal_top_hand_or_graveyard_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) =
                describe_each_player_choose_unselected_bounce_bundle(&filtered[idx..])
        {
            parts.push(rendered);
            break;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_grant_keyword_and_unblockable_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) =
                describe_return_creature_mana_value_scry_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(rendered) = describe_exchange_control_bundle(&filtered[idx..idx + 6])
        {
            parts.push(rendered);
            idx += 6;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(rendered) =
                describe_graveyard_mana_ladder_return_bundle(&filtered[idx..idx + 4])
        {
            parts.push(rendered);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_effect_list_linked_graveyard_choices_then_may_return_bundle(
                    &filtered[idx..idx + 3],
                )
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_random_hand_reveal_damage_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_random_hand_reveal_life_loss_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) = describe_random_hand_reveal_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) =
                describe_choose_reveal_from_hand_then_reflexive_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) =
                describe_choose_then_reveal_from_hand_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(rendered) = describe_self_unblockable_bundle(&filtered[idx..idx + 3])
        {
            parts.push(rendered);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len() {
            let create_and_grant = [(*filtered[idx]).clone(), (*filtered[idx + 1]).clone()];
            if let Some(rendered) = describe_create_token_then_grant_same_tag(&create_and_grant) {
                parts.push(rendered);
                idx += 2;
                continue;
            }
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) =
                describe_target_pump_unblockable_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) = describe_tap_freeze_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) = describe_target_freeze_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(rendered) = describe_reveal_top_to_hand_bundle(&filtered[idx..idx + 2])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = render_random_exile_choose_copy_then_cast_copy(&[
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
            && let Some(rendered) =
                render_shuffle_exile_top_then_cast_any_number_with_mana_value_cap(&[
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
            && let Some(rendered) = render_exile_top_then_put_from_among_onto_battlefield(&[
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
            && let Some(rendered) = render_exile_top_then_cast_any_number_with_mana_value_cap(&[
                filtered[idx],
                filtered[idx + 1],
            ])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(rendered) = render_each_player_exile_top_then_cast_any_number(&[
                filtered[idx],
                filtered[idx + 1],
            ])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(rendered) = render_consult_reveal_put_all_revealed_into_hand(&[
                filtered[idx],
                filtered[idx + 1],
            ])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(rendered) = render_consult_reveal_put_all_revealed_into_graveyard(&[
                filtered[idx],
                filtered[idx + 1],
            ])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }

        if idx + 2 < filtered.len()
            && let Some(rendered) = render_consult_reveal_put_battlefield_rest_graveyard(&[
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
            && let Some(rendered) =
                render_sacrifice_then_consult_reveal_put_battlefield_rest_bottom(&[
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

        if idx + 1 < filtered.len()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(rendered) = describe_destroy_for_each_destroyed_consult_exile_put_shuffle(
                filtered[idx],
                for_each,
            )
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }

        if let Some((rendered, consumed)) = render_repeated_named_searches_to_hand(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            render_single_named_search_to_hand_with_conditional_shuffle(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        // A run of independent optional single-target exiles reads as one
        // oracle sentence: "Exile up to one target artifact, up to one target
        // creature, ..., and/or up to one target land." — optionally followed
        // by a for-each-exiled reveal-until consult sentence.
        {
            fn up_to_one_target_exile_filter(effect: &Effect) -> Option<&ObjectFilter> {
                let exile =
                    unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::ExileEffect>()?;
                let ChooseSpec::WithCount(inner, count) = &exile.spec else {
                    return None;
                };
                if count.min != 0 || count.max != Some(1) {
                    return None;
                }
                let ChooseSpec::Target(target_inner) = inner.as_ref() else {
                    return None;
                };
                let ChooseSpec::Object(filter) = target_inner.as_ref() else {
                    return None;
                };
                Some(filter)
            }

            fn up_to_one_aggregate_choice_filter<'a>(
                effect: &'a Effect,
                expected_tag: Option<&str>,
            ) -> Option<(&'a ObjectFilter, &'a str)> {
                let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
                if choose.count.min != 0
                    || choose.count.max != Some(1)
                    || choose.chooser != PlayerFilter::You
                    || choose_primary_zone(choose) != Some(Zone::Battlefield)
                    || choose.is_search
                    || !crate::cards::is_sentence_helper_tag(choose.tag.as_str(), "exiled")
                    || expected_tag.is_some_and(|tag| tag != choose.tag.as_str())
                {
                    return None;
                }
                Some((&choose.filter, choose.tag.as_str()))
            }

            let mut run_filters = Vec::new();
            let mut lookahead = idx;
            let mut aggregate_tag = None;
            while lookahead < filtered.len() {
                let Some((filter, tag)) =
                    up_to_one_aggregate_choice_filter(filtered[lookahead], aggregate_tag)
                else {
                    break;
                };
                aggregate_tag = Some(tag);
                run_filters.push(filter);
                lookahead += 1;
            }
            if run_filters.len() >= 2 {
                let Some(tag) = aggregate_tag else {
                    unreachable!("aggregate choices always establish a tag")
                };
                if lookahead < filtered.len() && is_move_to_exile_of_tag(filtered[lookahead], tag) {
                    lookahead += 1;
                } else {
                    run_filters.clear();
                    aggregate_tag = None;
                    lookahead = idx;
                }
            }
            if run_filters.is_empty() {
                while lookahead < filtered.len()
                    && let Some(filter) = up_to_one_target_exile_filter(filtered[lookahead])
                {
                    run_filters.push(filter);
                    lookahead += 1;
                }
            }
            if run_filters.len() >= 2 {
                let items = run_filters
                    .iter()
                    .map(|filter| {
                        let mut display = (*filter).clone();
                        display.zone = None;
                        format!(
                            "up to one target {}",
                            strip_leading_article(&display.description())
                        )
                    })
                    .collect::<Vec<_>>();
                let (head_items, last_item) = items.split_at(items.len() - 1);
                let mut text = format!("Exile {}, and/or {}", head_items.join(", "), last_item[0]);
                let mut consumed = lookahead.saturating_sub(idx);
                let consult_selection = filtered.get(lookahead).and_then(|effect| {
                    if let Some(for_each) =
                        effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
                        && (for_each.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                            || aggregate_tag.is_some_and(|tag| for_each.tag.as_str() == tag))
                    {
                        return consult_reveal_put_battlefield_then_shuffle_selection(for_each);
                    }
                    let for_each = effect.downcast_ref::<crate::effects::ForEachObject>()?;
                    let iterates_aggregate =
                        filter_is_tagged_as(&for_each.filter, crate::tag::SOURCE_EXILED_TAG)
                            || aggregate_tag
                                .is_some_and(|tag| filter_is_tagged_as(&for_each.filter, tag));
                    iterates_aggregate.then(|| {
                        consult_reveal_put_battlefield_then_shuffle_effects(&for_each.effects)
                    })?
                });
                if let Some(selection) = consult_selection {
                    let permanent_types = [
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Planeswalker,
                        CardType::Battle,
                        CardType::Land,
                    ];
                    let all_permanents = run_filters.iter().all(|filter| {
                        !filter.card_types.is_empty()
                            && filter
                                .card_types
                                .iter()
                                .all(|card_type| permanent_types.contains(card_type))
                    });
                    let distinct_types = run_filters
                        .iter()
                        .flat_map(|filter| filter.card_types.iter())
                        .collect::<std::collections::HashSet<_>>();
                    let noun = if all_permanents && distinct_types.len() > 1 {
                        "permanent".to_string()
                    } else if let [only] = distinct_types.iter().collect::<Vec<_>>()[..] {
                        only.to_string().to_ascii_lowercase()
                    } else {
                        "permanent".to_string()
                    };
                    // Inside the for-each the shared-type constraint points at
                    // the iterated object, which the oracle names "it".
                    let selection = selection.replace(
                        "shares a card type with that object",
                        "shares a card type with it",
                    );
                    text.push_str(&format!(
                        ". For each {noun} exiled this way, its controller reveals cards from the top of their library until they reveal {}, puts that card onto the battlefield, then shuffles",
                        with_indefinite_article(&selection)
                    ));
                    consumed += 1;
                }
                parts.push(text);
                idx += consumed;
                continue;
            }
        }

        if idx + 2 < filtered.len()
            && let Some(exiled_tag) =
                optional_nonland_permanent_choice(filtered[idx], Zone::Battlefield, None)
            && optional_nonland_permanent_choice(
                filtered[idx + 1],
                Zone::Graveyard,
                Some(exiled_tag),
            ) == Some(exiled_tag)
            && is_move_to_exile_of_tag(filtered[idx + 2], exiled_tag)
        {
            parts.push("Exile up to one target nonland permanent and up to one target nonland permanent card from a graveyard".to_string());
            idx += 3;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(schedule) =
                filtered[idx + 1].downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        {
            let trigger_text = schedule.trigger.display().to_ascii_lowercase();
            if choose_targets_schedule_trigger(choose, schedule)
                && (trigger_text.contains("this creature attacks and isn't blocked")
                    || trigger_text.contains("this creature attacks and isnt blocked"))
            {
                let rendered = describe_effect(filtered[idx + 1]);
                if !rendered.is_empty() {
                    parts.push(rendered);
                }
                idx += 2;
                continue;
            }
        }

        if idx + 2 < filtered.len()
            && let Some(exiled_tag) =
                choose_exact_target_type(filtered[idx], crate::types::CardType::Creature, 2)
                    .or_else(|| {
                        tagged_exile_exact_target_type(
                            filtered[idx],
                            crate::types::CardType::Creature,
                            2,
                        )
                    })
            && is_move_to_exile_of_tag(filtered[idx + 1], exiled_tag)
            && let Some(for_each) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && for_each.tag.as_str() == exiled_tag
            && consult_reveal_put_battlefield_then_shuffle_selection(for_each).as_deref()
                == Some("creature")
        {
            parts.push("Exile two target creatures. For each of those creatures, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles the rest into their library".to_string());
            idx += 3;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(exiled_tag) =
                tagged_exile_exact_target_type(filtered[idx], crate::types::CardType::Creature, 2)
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachObject>()
            && (filter_is_tagged_as(&for_each.filter, exiled_tag)
                || filter_is_tagged_as(&for_each.filter, crate::tag::SOURCE_EXILED_TAG))
            && consult_reveal_put_battlefield_then_shuffle_effects(&for_each.effects).as_deref()
                == Some("creature")
        {
            parts.push("Exile two target creatures. For each of those creatures, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles the rest into their library".to_string());
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(exiled_tag) =
                tagged_exile_exact_target_type(filtered[idx], crate::types::CardType::Creature, 2)
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && (for_each.tag.as_str() == exiled_tag
                || for_each.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
            && consult_reveal_put_battlefield_then_shuffle_selection(for_each).as_deref()
                == Some("creature")
        {
            parts.push("Exile two target creatures. For each of those creatures, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles the rest into their library".to_string());
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && exile_exact_target_type(filtered[idx], crate::types::CardType::Creature, 2)
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && (for_each.tag.as_str().starts_with("exiled_")
                || crate::cards::is_sentence_helper_tag(for_each.tag.as_str(), "exiled"))
            && consult_reveal_put_battlefield_then_shuffle_selection(for_each).as_deref()
                == Some("creature")
        {
            parts.push("Exile two target creatures. For each of those creatures, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles the rest into their library".to_string());
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(exiled_tag) = tagged_exile_any_number_target_creatures(filtered[idx])
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && for_each.tag.as_str() == exiled_tag
            && consult_reveal_put_battlefield_then_bottom_selection(for_each).is_some_and(
                |selection| {
                    matches!(
                        selection.as_str(),
                        "creature" | "creature card" | "creature card in library"
                    )
                },
            )
        {
            parts.push("Exile any number of target creatures controlled by different players. For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order".to_string());
            idx += 2;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_compact_tagged_apply_continuous_pair(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) =
                describe_destroyed_land_controller_basic_search_then_player_shuffle(
                    filtered[idx],
                    filtered[idx + 1],
                    filtered[idx + 2],
                )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_destroyed_land_basic_search_then_player_shuffle(
                filtered[idx],
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_destroy_then_doubled_life_loss(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(first_apply) = apply_continuous_for_compaction(filtered[idx])
            && let Some(second_apply) = apply_continuous_for_compaction(filtered[idx + 1])
            && let Some(compact) = describe_compact_apply_continuous_pair(first_apply, second_apply)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose_creature_type) =
                filtered[idx].downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
            && let Some(compact) =
                describe_choose_creature_type_then_x_boost(choose_creature_type, filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose_creature_type) =
                filtered[idx].downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
            && let Some(compact) = describe_choose_creature_type_then_must_attack(
                choose_creature_type,
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(phase_out) =
                filtered[idx + 1].downcast_ref::<crate::effects::PhaseOutEffect>()
            && let Some(compact) = describe_choose_type_then_phase_out(choose, phase_out)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_put_onto_battlefield_attached(&filtered[idx..idx + 3])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_put_onto_battlefield_attached(&filtered[idx..idx + 2])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(sacrifice_choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice_with_id) =
                filtered[idx + 1].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(look_at_top) =
                filtered[idx + 2].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 3].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(split) =
                filtered[idx + 4].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(rest) = filtered[idx + 5]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_sacrifice_reveal_top_choose_land_nonland_split_rest_bottom(
                    sacrifice_choose,
                    sacrifice_with_id,
                    look_at_top,
                    choose,
                    split,
                    rest,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(put_sticker) =
                filtered[idx + 1].downcast_ref::<crate::effects::PutStickerEffect>()
            && let Some(compact) = describe_choose_then_put_sticker(choose, put_sticker)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tag_attached) =
                filtered[idx].downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            && let Some(compact) =
                describe_tag_attached_then_double_power(tag_attached, filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tag_attached) =
                filtered[idx].downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            && let Some(compact) =
                describe_tag_attached_then_tap_or_untap(tag_attached, filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tag_attached) =
                filtered[idx].downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            && let Some(compact) =
                describe_tag_attached_then_unattach(tag_attached, filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(compact) = describe_exile_then_return_transformed_with_counter(
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
                filtered[idx + 3],
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_exile_return_then_transform(
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_return_then_transform(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(move_back) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_exile_then_return(tagged, move_back)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_source_exile_then_return(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_create_attached_token_then_reflexive_fight(
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_create_token_attached_to_target(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if let Some((compact, consumed)) =
            describe_immediate_observation_conditionals(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(reveal_top) =
                filtered[idx].downcast_ref::<crate::effects::RevealTopEffect>()
            && let Some(conditional) =
                filtered[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(compact) =
                describe_reveal_top_then_if_put_into_hand(reveal_top, conditional)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(reveal_top) =
                filtered[idx].downcast_ref::<crate::effects::RevealTopEffect>()
            && let Some(with_id) = filtered[idx + 1].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = filtered[idx + 2].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) =
                describe_reveal_top_may_put_otherwise_hand(reveal_top, with_id, if_effect)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(deal) = filtered[idx + 1].downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(compact) = describe_tap_then_damage_for_tapped_this_way(with_id, deal)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(choose_new) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
            && let Some(compact) = describe_with_id_then_choose_new_targets(with_id, choose_new)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(may) = filtered[idx + 1].downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) = describe_with_id_then_may_choose_new_targets(with_id, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(for_players) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(compact) =
                describe_with_id_then_for_players_if_happened(with_id, for_players)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(for_players) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(compact) = describe_with_id_then_for_players_if_didnt(with_id, for_players)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && idx + 2 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(reflexive) =
                filtered[idx + 1].downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
            && let Some(grant) =
                filtered[idx + 2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) =
                describe_exile_play_then_reflexive_trigger(with_id, reflexive, grant)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(reflexive) =
                filtered[idx + 1].downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
            && let Some(compact) = describe_with_id_then_reflexive_trigger(with_id, reflexive)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = filtered[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && if_effect.predicate == EffectPredicate::Happened
            && if_effect.else_.is_empty()
            && if_effect.then.len() == 2
            && let Some(rendered) = render_consult_reveal_put_battlefield_rest_graveyard(&[
                &with_id.effect,
                &if_effect.then[0],
                &if_effect.then[1],
            ])
        {
            parts.push(rendered);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) =
                describe_wrapped_search_for_each_then_conditional_shuffle(&filtered[idx..])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
        {
            let mut branch_parts = Vec::new();
            let mut lookahead = idx + 1;
            while lookahead < filtered.len() {
                let if_effect = filtered[lookahead]
                    .downcast_ref::<crate::effects::IfEffect>()
                    .or_else(|| {
                        filtered[lookahead]
                            .downcast_ref::<crate::effects::WithIdEffect>()
                            .and_then(|nested| {
                                nested.effect.downcast_ref::<crate::effects::IfEffect>()
                            })
                    });
                let Some(if_effect) = if_effect else {
                    break;
                };
                if if_effect.condition != with_id.id {
                    break;
                }
                let Some(branch_text) = describe_with_id_if_clause(with_id, if_effect) else {
                    break;
                };
                branch_parts.push(branch_text);
                lookahead += 1;
            }
            if !branch_parts.is_empty() {
                let setup = describe_optional_setup_effect_for_if_happened(with_id)
                    .unwrap_or_else(|| describe_effect(&with_id.effect));
                parts.push(format!("{setup}. {}", branch_parts.join(". ")));
                idx = lookahead;
                continue;
            }
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(deal) = filtered[idx + 1]
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .or_else(|| {
                    filtered[idx + 1]
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
                        })
                        .and_then(|with_source| {
                            with_source
                                .effect
                                .downcast_ref::<crate::effects::DealDamageEffect>()
                        })
                })
            && let Some(compact) = describe_tagged_target_then_power_damage(tagged, deal)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) = describe_target_power_damage_to_other_and_self(
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(for_each_object) =
                filtered[idx].downcast_ref::<crate::effects::ForEachObject>()
            && let Some(shuffle) =
                filtered[idx + 2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(compact) = describe_for_each_same_name_search_to_battlefield(
                for_each_object,
                filtered[idx + 1],
                shuffle,
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_choose_then_for_each_copy(choose, for_each)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(shuffle) =
                filtered[idx + 3].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(compact) = describe_choose_then_for_each_same_name_search_to_battlefield(
                choose,
                for_each,
                filtered[idx + 2],
                shuffle,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && reveal.tag == choose.tag
            && let Some(battlefield_conditional) =
                filtered[idx + 2].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(hand_conditional) = filtered
                .get(idx + 3)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
            && let Some(shuffle) = filtered
                .get(idx + 4)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>())
            && let Some(compact) =
                describe_search_reveal_conditional_may_battlefield_else_hand_then_shuffle(
                    choose,
                    reveal,
                    battlefield_conditional,
                    hand_conditional,
                    shuffle,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && reveal.tag == choose.tag
            && let Some(conditional) =
                filtered[idx + 2].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(shuffle) = filtered
                .get(idx + 3)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>())
            && let Some(compact) = describe_search_reveal_named_conditional_move_then_shuffle(
                choose,
                reveal,
                conditional,
                shuffle,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && reveal.tag == choose.tag
            && let Some(conditional) =
                filtered[idx + 2].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(shuffle) = filtered
                .get(idx + 3)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>())
            && let Some(compact) = describe_search_reveal_conditional_move_then_shuffle(
                choose,
                reveal,
                conditional,
                shuffle,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && reveal.tag == choose.tag
            && let Some(move_to_zone) =
                filtered[idx + 2].downcast_ref::<crate::effects::MoveToZoneEffect>()
        {
            let shuffle = filtered
                .get(idx + 3)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(compact) =
                describe_search_choose_then_move(choose, Some(reveal), move_to_zone, shuffle)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 4 } else { 3 };
                continue;
            }
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && reveal.tag == choose.tag
            && let Some(for_each) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            let shuffle = filtered
                .get(idx + 3)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            let mut revealed_choose = choose.clone();
            revealed_choose.reveal = true;
            if let Some(compact) =
                describe_search_choose_for_each(&revealed_choose, for_each, shuffle, false)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 4 } else { 3 };
                continue;
            }
            if let Some(move_to_zone) =
                filtered[idx + 2].downcast_ref::<crate::effects::MoveToZoneEffect>()
            {
                let shuffle = filtered.get(idx + 3).and_then(|effect| {
                    effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                });
                if let Some(compact) =
                    describe_search_choose_then_move(choose, Some(reveal), move_to_zone, shuffle)
                {
                    parts.push(compact);
                    idx += if shuffle.is_some() { 4 } else { 3 };
                    continue;
                }
            }
            if let Some(exile) = filtered[idx + 2].downcast_ref::<crate::effects::ExileEffect>() {
                let shuffle = filtered.get(idx + 3).and_then(|effect| {
                    effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                });
                if let Some(compact) =
                    describe_search_choose_then_exile(choose, Some(reveal), exile, shuffle)
                {
                    parts.push(compact);
                    idx += if shuffle.is_some() { 4 } else { 3 };
                    continue;
                }
            }
            if let Some(return_to_hand) =
                filtered[idx + 2].downcast_ref::<crate::effects::ReturnToHandEffect>()
            {
                let shuffle = filtered.get(idx + 3).and_then(|effect| {
                    effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                });
                if let Some(compact) = describe_search_choose_then_return_to_hand(
                    choose,
                    Some(reveal),
                    return_to_hand,
                    shuffle,
                ) {
                    parts.push(compact);
                    idx += if shuffle.is_some() { 4 } else { 3 };
                    continue;
                }
            }
        }
        if idx + 3 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(shuffle) =
                filtered[idx + 2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        {
            if let Some(exile) =
                unwrap_tag_wrappers(filtered[idx + 1]).downcast_ref::<crate::effects::ExileEffect>()
                && let Some(conditional) =
                    filtered[idx + 3].downcast_ref::<crate::effects::ConditionalEffect>()
                && let Some(compact) =
                    describe_search_face_down_exile_shuffle_conditional_cast_else_hand(
                        choose,
                        exile,
                        shuffle,
                        conditional,
                    )
            {
                parts.push(compact);
                idx += 4;
                continue;
            }
            if let Some(compact) = describe_search_choose_then_exile_and_cast(
                choose,
                filtered[idx + 1],
                shuffle,
                filtered[idx + 3],
            ) {
                parts.push(compact);
                idx += 4;
                continue;
            }
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(shuffle) =
                filtered[idx + 2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(compact) = describe_search_choose_then_cast_then_shuffle(
                choose,
                filtered[idx + 1],
                shuffle,
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        {
            let next_effect = unwrap_tag_wrappers(filtered[idx + 1]);
            let shuffle = filtered
                .get(idx + 2)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(move_to_zone) =
                next_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
                && let Some(compact) =
                    describe_search_choose_then_move(choose, None, move_to_zone, shuffle)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
            if let Some(exile) = next_effect.downcast_ref::<crate::effects::ExileEffect>()
                && let Some(compact) =
                    describe_search_choose_then_exile(choose, None, exile, shuffle)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
            if let Some(return_to_hand) =
                next_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
                && let Some(compact) = describe_search_choose_then_return_to_hand(
                    choose,
                    None,
                    return_to_hand,
                    shuffle,
                )
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            let shuffle = filtered
                .get(idx + 2)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(compact) = describe_search_choose_for_each(choose, for_each, shuffle, false)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
            if let Some(move_to_zone) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            {
                let shuffle = filtered.get(idx + 2).and_then(|effect| {
                    effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                });
                if let Some(compact) =
                    describe_search_choose_then_move(choose, None, move_to_zone, shuffle)
                {
                    parts.push(compact);
                    idx += if shuffle.is_some() { 3 } else { 2 };
                    continue;
                }
            }
            if let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>() {
                let shuffle = filtered.get(idx + 2).and_then(|effect| {
                    effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                });
                if let Some(compact) =
                    describe_search_choose_then_exile(choose, None, exile, shuffle)
                {
                    parts.push(compact);
                    idx += if shuffle.is_some() { 3 } else { 2 };
                    continue;
                }
            }
            if let Some(return_to_hand) =
                filtered[idx + 1].downcast_ref::<crate::effects::ReturnToHandEffect>()
            {
                let shuffle = filtered.get(idx + 2).and_then(|effect| {
                    effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                });
                if let Some(compact) = describe_search_choose_then_return_to_hand(
                    choose,
                    None,
                    return_to_hand,
                    shuffle,
                ) {
                    parts.push(compact);
                    idx += if shuffle.is_some() { 3 } else { 2 };
                    continue;
                }
            }
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(shuffle) =
                filtered[idx + 1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(for_each) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_search_choose_for_each(choose, for_each, Some(shuffle), true)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(put) = filtered[idx + 2].downcast_ref::<crate::effects::PutCountersEffect>()
            && let Some(compact) = describe_choose_exile_then_put_counter(choose, exile, put)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) = describe_choose_then_exile(choose, exile)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(compact) =
                describe_for_players_choose_move_then_characteristics(&filtered[idx..idx + 5])
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(for_players) = with_id
                .effect
                .downcast_ref::<crate::effects::ForPlayersEffect>()
            && for_players.filter == PlayerFilter::Opponent
            && let [lose_effect] = for_players.effects.as_slice()
            && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
            && matches!(lose.player, ChooseSpec::Player(PlayerFilter::IteratedPlayer))
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
            && matches!(
                gain.amount.unhinted(),
                Value::EffectMetric {
                    effect_id,
                    metric: crate::effect::EffectMetric::LifeLost,
                    ..
                } if *effect_id == with_id.id
            )
            && let Some(where_x) = describe_where_x_basis(&lose.amount)
        {
            parts.push(format!(
                "Each opponent loses X life, where X is {where_x}. You gain life equal to the life lost this way"
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(for_players) =
                filtered[idx].downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(move_to_zone) = unwrap_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) =
                describe_for_players_choose_then_move_to_battlefield(for_players, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) = unwrap_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_choose_then_move_to_hand(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_choose_then_move_to_battlefield(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) = unwrap_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_choose_then_move_to_battlefield(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) = unwrap_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_choose_then_move_to_library(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(return_to_hand) =
                filtered[idx + 1].downcast_ref::<crate::effects::ReturnToHandEffect>()
            && let Some(compact) =
                describe_target_player_choose_half_then_return_to_hand(choose, return_to_hand)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(for_players) =
                filtered[idx].downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(look) =
                filtered[idx + 1].downcast_ref::<crate::effects::LookAtObjectsEffect>()
            && let Some(grant) =
                filtered[idx + 2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) =
                describe_for_players_bottom_library_exile_then_look_cast(for_players, look, grant)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(conditional) =
                filtered[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(bottom_conditional) =
                filtered[idx + 2].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(compact) =
                describe_look_top_card_if_matching_may_reveal_put_hand_else_bottom(
                    look_at_top,
                    conditional,
                    bottom_conditional,
                )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(conditional) =
                filtered[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(compact) =
                describe_look_top_card_if_matching_may_reveal_put_hand(look_at_top, conditional)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(move_to_zone) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) =
                describe_look_at_top_then_move_to_exile(look_at_top, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(exile_top) = unwrap_tag_wrappers(filtered[idx])
                .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            && let Some(grant_play) = unwrap_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(grant_free_cast) =
                unwrap_tag_wrappers(filtered[idx + 2])
                    .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
            && let Some(compact) = describe_exile_top_then_play_without_paying_mana(
                exile_top,
                grant_play,
                grant_free_cast,
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(exile_top) = unwrap_tag_wrappers(filtered[idx])
                .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            && let Some(may) = unwrap_basic_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) = describe_exile_top_then_may_cast(exile_top, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(shuffle) =
                filtered[idx].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(reveal_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTopEffect>()
            && let Some(reveal_permission) = unwrap_tag_wrappers(filtered[idx + 2])
                .downcast_ref::<crate::effects::ApplyContinuousEffect>(
            )
            && let Some(grant_play) =
                filtered[idx + 3].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(grant_free_cast) =
                filtered[idx + 4]
                    .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
            && let Some(compact) =
                describe_shuffle_then_reveal_top_then_temporarily_play_revealed_top_card(
                    shuffle,
                    reveal_top,
                    reveal_permission,
                    grant_play,
                    grant_free_cast,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(reveal_top) =
                filtered[idx].downcast_ref::<crate::effects::RevealTopEffect>()
            && let Some(reveal_permission) = unwrap_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::ApplyContinuousEffect>(
            )
            && let Some(grant_play) =
                filtered[idx + 2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(grant_free_cast) =
                filtered[idx + 3]
                    .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
            && let Some(compact) = describe_reveal_top_then_temporarily_play_revealed_top_card(
                reveal_top,
                reveal_permission,
                grant_play,
                grant_free_cast,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(exile_top) =
                filtered[idx].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(grant_play) =
                filtered[idx + 2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) =
                describe_exile_top_choose_one_then_play(exile_top, choose, grant_play)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(tag_triggering) =
                filtered[idx].downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            && let Some(exile_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            && let Some(grant_play) =
                filtered[idx + 2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) = describe_triggering_counter_count_exile_top_then_play(
                tag_triggering,
                exile_top,
                grant_play,
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(exile_top) =
                filtered[idx].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            && let Some(grant_play) =
                filtered[idx + 1].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) = describe_exile_top_then_play(
                exile_top,
                grant_play,
                idx.checked_sub(1)
                    .and_then(|previous_idx| {
                        filtered[previous_idx]
                            .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
                    })
                    .and_then(|reduction| reduction.generic_reduction.as_ref())
                    .is_some_and(|reduction| reduction == &exile_top.count),
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(grant) =
                filtered[idx + 2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) = describe_look_at_top_exile_face_down_then_play_while_exiled(
                look_at_top,
                exile,
                grant,
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = unwrap_basic_tag_wrappers(filtered[idx + 2])
                .downcast_ref::<crate::effects::ExileEffect>()
            && let Some(rest) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(play_grant) =
                filtered[idx + 4].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(any_mana_grant) =
                filtered[idx + 5].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) =
                describe_look_at_top_choose_exile_rest_bottom_play_grants_and_any_mana_while_exiled(
                    look_at_top,
                    choose,
                    exile,
                    rest,
                    play_grant,
                    any_mana_grant,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = unwrap_basic_tag_wrappers(filtered[idx + 2])
                .downcast_ref::<crate::effects::ExileEffect>()
            && let Some(rest) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(grant) =
                filtered[idx + 4].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) =
                describe_look_at_top_choose_exile_face_down_rest_bottom_then_play_while_exiled(
                    look_at_top,
                    choose,
                    exile,
                    rest,
                    grant,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = unwrap_basic_tag_wrappers(filtered[idx + 2])
                .downcast_ref::<crate::effects::ExileEffect>()
            && let Some(rest) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(may_play) = filtered[idx + 4].downcast_ref::<crate::effects::MayEffect>()
            && let Some(any_mana_grant) =
                filtered[idx + 5].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) =
                describe_look_at_top_choose_exile_rest_bottom_play_and_any_mana_while_exiled(
                    look_at_top,
                    choose,
                    exile,
                    rest,
                    may_play,
                    any_mana_grant,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 7 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(hand_choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(bottom_choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile_choose) =
                filtered[idx + 3].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(hand_move) = unwrap_tag_wrappers(filtered[idx + 4])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(bottom_move) = unwrap_tag_wrappers(filtered[idx + 5])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(exile_move) = unwrap_tag_wrappers(filtered[idx + 6])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(grant) =
                filtered[idx + 7].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            && let Some(compact) = describe_look_at_top_split_hand_bottom_exile_then_play_exiled(
                look_at_top,
                hand_choose,
                bottom_choose,
                exile_choose,
                hand_move,
                bottom_move,
                exile_move,
                grant,
            )
        {
            parts.push(compact);
            idx += 8;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(tag_matching) =
                filtered[idx + 2].downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
            && let Some((_, move_matching)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(remainder) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_look_at_top_then_reveal_put_matching_into_hand_rest_bottom(
                    look_at_top,
                    reveal_tagged,
                    tag_matching,
                    move_matching,
                    remainder,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(tag_matching) =
                filtered[idx + 2].downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
            && let Some((_, move_matching)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(remainder) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_look_at_top_then_reveal_put_all_matching_onto_battlefield_rest_bottom(
                    look_at_top,
                    reveal_tagged,
                    tag_matching,
                    move_matching,
                    remainder,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(choose_name) =
                filtered[idx].downcast_ref::<crate::effects::ChooseCardNameEffect>()
            && let Some(look_at_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 2].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some((_, distribute)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(compact) = describe_choose_name_then_reveal_matching_hand_rest_graveyard(
                choose_name,
                look_at_top,
                reveal_tagged,
                distribute,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some((_, distribute)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(compact) =
                describe_look_at_top_then_reveal_put_matching_onto_battlefield_rest_graveyard(
                    look_at_top,
                    reveal_tagged,
                    distribute,
                )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some((_, distribute)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(compact) =
                describe_look_at_top_then_reveal_put_matching_into_hand_rest_graveyard(
                    look_at_top,
                    reveal_tagged,
                    distribute,
                )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(remainder) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
                look_at_top,
                Some(reveal_tagged),
                choose,
                filtered[idx + 3],
                remainder,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(remainder) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
                look_at_top,
                None,
                choose,
                filtered[idx + 2],
                remainder,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(compact) = describe_choose_name_exile_top_consult_hand_rest_exile(&[
                filtered[idx],
                filtered[idx + 1],
                filtered[idx + 2],
                filtered[idx + 3],
                filtered[idx + 4],
            ])
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 3].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(filtered[idx + 4])
            && let Some(rest) =
                filtered[idx + 5].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
                look_at_top,
                Some(reveal_top),
                choose,
                Some(reveal),
                move_to_hand,
                rest,
            )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) =
                filtered[idx + 4].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
                look_at_top,
                Some(reveal_top),
                choose,
                None,
                move_to_hand,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 6 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(land_choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(hand_choose) =
                filtered[idx + 4].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(remainder) = filtered[idx + 6]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_look_at_top_then_put_onto_battlefield_and_into_hand_rest_bottom(
                    look_at_top,
                    Some(reveal_top),
                    land_choose,
                    filtered[idx + 3],
                    hand_choose,
                    filtered[idx + 5],
                    remainder,
                )
        {
            parts.push(compact);
            idx += 7;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(land_choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(hand_choose) =
                filtered[idx + 3].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(remainder) = filtered[idx + 5]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_look_at_top_then_put_onto_battlefield_and_into_hand_rest_bottom(
                    look_at_top,
                    None,
                    land_choose,
                    filtered[idx + 2],
                    hand_choose,
                    filtered[idx + 4],
                    remainder,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(battlefield_choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((battlefield_move_id, battlefield_move)) =
                for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(if_not_moved) = filtered[idx + 3].downcast_ref::<crate::effects::IfEffect>()
            && let Some(rest) =
                filtered[idx + 4].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_look_at_top_then_may_put_battlefield_else_hand_rest_bottom(
                    look_at_top,
                    battlefield_choose,
                    battlefield_move_id,
                    battlefield_move,
                    if_not_moved,
                    rest,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) =
                filtered[idx + 2].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) =
                describe_look_at_top_then_choose_move_to_exile(look_at_top, choose, move_to_zone)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 2].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) =
                describe_look_at_top_then_choose_exile(look_at_top, choose, exile)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_hand) = unwrap_tag_wrappers(filtered[idx + 2])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(rest) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_then_put_one_hand_other_bottom(
                look_at_top,
                choose,
                move_to_hand,
                rest,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(rest) =
                filtered[idx + 3].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_look_at_top_then_put_chosen_hand_rest_bottom_from_for_each(
                    look_at_top,
                    choose,
                    move_to_hand,
                    rest,
                )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) =
                filtered[idx + 4].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
                look_at_top,
                None,
                choose,
                Some(reveal),
                move_to_hand,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(conditional) =
                filtered[idx + 3].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(shuffle) =
                filtered[idx + 4].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(compact) =
                describe_look_at_top_reveal_matching_bargain_battlefield_else_hand(
                    look_at_top,
                    choose,
                    reveal,
                    conditional,
                    shuffle,
                )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_then_reveal_any_matching_to_hand_rest_bottom(
                look_at_top,
                choose,
                reveal,
                move_chosen,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(first_choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(second_choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 3].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 4])
            && let Some(rest) = filtered[idx + 5]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_look_at_top_then_reveal_split_matching_to_hand_rest_bottom(
                    look_at_top,
                    &[first_choose, second_choose],
                    reveal,
                    move_chosen,
                    rest,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some((_, rest)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
                look_at_top,
                None,
                choose,
                move_chosen,
                rest,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_tagged) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some((_, rest)) = for_each_tagged_for_compaction(filtered[idx + 4])
            && let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
                look_at_top,
                Some(reveal_tagged),
                choose,
                move_chosen,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(rest) =
                filtered[idx + 3].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
                look_at_top,
                None,
                choose,
                None,
                move_to_hand,
                rest,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(reveal_top) =
                filtered[idx + 1].downcast_ref::<crate::effects::RevealTaggedEffect>()
            && let Some(choose) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
                look_at_top,
                Some(reveal_top),
                choose,
                move_chosen,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(rest) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
                look_at_top,
                None,
                choose,
                move_chosen,
                rest,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(cast) = filtered[idx + 2].downcast_ref::<crate::effects::CastTaggedEffect>()
            && let Some(rest) = filtered[idx + 3]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) =
                describe_look_at_top_then_cast_matching_rest_bottom(look_at_top, choose, cast, rest)
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 5 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some((Some(move_to_hand_with_id), move_to_hand)) =
                for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(if_effect) = filtered[idx + 5].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) =
                describe_look_at_top_then_reveal_put_into_hand_rest_bottom_then_if_not_into_hand(
                    look_at_top,
                    choose,
                    reveal,
                    move_to_hand_with_id,
                    move_to_hand,
                    rest,
                    if_effect,
                )
        {
            parts.push(compact);
            idx += 6;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some((_, move_to_top)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_then_reveal_put_on_top_rest_bottom(
                look_at_top,
                choose,
                reveal,
                move_to_top,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        if idx + 4 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(reveal) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(rest) = filtered[idx + 4]
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
            )
            && let Some(compact) = describe_look_at_top_then_reveal_put_into_hand_rest_bottom(
                look_at_top,
                choose,
                Some(reveal),
                move_to_hand,
                rest,
            )
        {
            parts.push(compact);
            idx += 5;
            continue;
        }
        // "Target player reveals three cards from their hand and you choose
        // one of them. That player discards that card." — the revealer picks
        // the revealed pool, you pick from it, they discard your pick.
        if idx + 2 < filtered.len()
            && let Some(first_choose) = unwrap_basic_tag_wrappers(filtered[idx])
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(second_choose) = unwrap_basic_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(discard) = unwrap_basic_tag_wrappers(filtered[idx + 2])
                .downcast_ref::<crate::effects::DiscardEffect>()
            && matches!(&first_choose.chooser, PlayerFilter::Target(_))
            && choose_primary_zone(first_choose) == Some(Zone::Hand)
            && first_choose.filter.owner.as_ref() == Some(&first_choose.chooser)
            && second_choose.chooser == PlayerFilter::You
            && choose_exact_count(second_choose) == Some(1)
            && (choose_references_tag(second_choose, &first_choose.tag)
                || (choose_primary_zone(second_choose) == Some(Zone::Hand)
                    && second_choose.filter.owner.as_ref() == Some(&first_choose.chooser)))
            && discard.player == first_choose.chooser
            && !discard.random
            && !discard.any_number
            && discard
                .card_filter
                .as_ref()
                .is_some_and(|filter| object_filter_has_tag(filter, &second_choose.tag))
        {
            let revealer = describe_player_filter(&first_choose.chooser);
            let reveal_count = choose_exact_count(first_choose)
                .map(|count| number_word(count as i32).unwrap_or_else(|| count.to_string()))
                .unwrap_or_else(|| describe_choice_count(&first_choose.count));
            parts.push(format!(
                "{} reveals {reveal_count} cards from their hand and you choose one of them. That player discards that card",
                capitalize_first(&revealer)
            ));
            idx += 3;
            continue;
        }
        // "You and that player each gain that much life" — adjacent same-
        // amount life gains for you plus a back-referenced player compact to
        // the oracle's joint-subject sentence.
        if idx + 1 < filtered.len()
            && let Some(first_gain) = unwrap_basic_tag_wrappers(filtered[idx])
                .downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(second_gain) = unwrap_basic_tag_wrappers(filtered[idx + 1])
                .downcast_ref::<crate::effects::GainLifeEffect>()
            && first_gain.amount == second_gain.amount
            && matches!(&first_gain.player, ChooseSpec::Player(PlayerFilter::You))
            && let ChooseSpec::Player(second_player) = &second_gain.player
            && *second_player != PlayerFilter::You
        {
            let other = match second_player {
                PlayerFilter::DamagedPlayer | PlayerFilter::TaggedPlayer(_) => {
                    "that player".to_string()
                }
                other => describe_player_filter(other),
            };
            parts.push(format!(
                "You and {other} each gain {}",
                describe_life_amount_phrase(&first_gain.amount)
            ));
            idx += 2;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(compact) =
                describe_choose_sacrifice_then_return_from_graveyard(&filtered[idx..idx + 4])
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) =
                describe_choose_sacrifice_then_reflexive_trigger_refs(&filtered[idx..idx + 3])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some((compact, consumed)) =
                describe_choose_sacrifice_then_same_player_actions(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some((compact, consumed)) =
                describe_two_choose_sacrifices_same_player(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(with_id) = filtered[idx + 1].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(sacrifice) = sacrifice_view(&with_id.effect)
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) = sacrifice_view(filtered[idx + 1])
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(destroy) = destroy_effect_for_choose_compaction(filtered[idx + 1])
            && let Some(compact) = describe_choose_then_destroy(choose, destroy)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_tap_then_put_counters_same_target(
                filtered[idx],
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_put_counters_then_untap_them(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(deal) = filtered[idx].downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(tagged) = filtered[idx + 1].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(cant) = filtered[idx + 2].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_damage_then_self_skip_next_untap(deal, tagged, cant)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(deal) = filtered[idx].downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_damage_then_source_skip_next_untap(deal, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(deal) = tagged
                .effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_damage_then_source_skip_next_untap(deal, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(for_each) = filtered[idx].downcast_ref::<crate::effects::ForEachObject>()
            && let Some(compact) = describe_for_each_chosen_put_counters_then_gain_keywords(
                for_each,
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(additional_phases) =
                filtered[idx].downcast_ref::<crate::effects::AdditionalPhasesEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) =
                describe_additional_combat_then_chosen_attack_or_block_restriction(
                    additional_phases,
                    cant,
                )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_tagged_target_then_cant_restriction(tagged, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_choose_then_cant_pile_restriction(choose, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) =
                describe_choose_sacrifice_then_draw_for_sacrificed(&filtered[idx..idx + 3])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(compact) =
                describe_discard_hand_add_mana_draw_sequence(&filtered[idx..idx + 3])
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_discard_then_exile_same_player_graveyard(
                filtered[idx],
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some((compact, consumed)) =
                describe_same_referenced_player_action_sequence(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_action_and_get_energy_pair(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_same_actor_gain_then_draw(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_same_actor_draw_then_gain(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(discard) = filtered[idx + 1].downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(compact) = describe_draw_then_discard(draw, discard)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(first_draw) = draw_cards_view(filtered[idx])
            && let Some(second_draw) = draw_cards_view(filtered[idx + 1])
            && let Some(compact) =
                describe_fixed_draw_then_equal_to_draw(first_draw, second_draw)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(first_draw) = draw_cards_view(filtered[idx])
            && let Some(second_draw) = draw_cards_view(filtered[idx + 1])
            && let Some(compact) = describe_shared_draw(first_draw, second_draw)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(draw_you) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(draw_attacking) =
                filtered[idx + 1].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(lose_you) =
                filtered[idx + 2].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(lose_attacking) =
                filtered[idx + 3].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(compact) = describe_you_and_attacking_player_draw_and_lose(
                draw_you,
                draw_attacking,
                lose_you,
                lose_attacking,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        // Draw N cards, then you gain life equal to the number of [filter].
        if idx + 1 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && matches!(draw.player, PlayerFilter::You)
            && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
            && let Some(compact) = describe_draw_then_gain_life(draw, gain)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(lose) = filtered[idx + 1].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(energy) =
                filtered[idx + 2].downcast_ref::<crate::effects::EnergyCountersEffect>()
            && let Some(compact) = describe_draw_lose_life_get_energy(draw, lose, energy)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(target_only) =
                filtered[idx + 1].downcast_ref::<crate::effects::TargetOnlyEffect>()
            && let Some(lose) = filtered[idx + 2].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(compact) =
                describe_target_player_draw_then_lose_life(draw, target_only, lose)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(target_only) =
                filtered[idx].downcast_ref::<crate::effects::TargetOnlyEffect>()
            && let Some(lose) = filtered[idx + 1].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(gain) = filtered[idx + 2].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(compact) =
                describe_target_player_lose_then_you_gain_life(target_only, lose, gain)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(lose) = filtered[idx + 1].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(compact) = describe_draw_then_lose_life(draw, lose)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(mill) = filtered[idx].downcast_ref::<crate::effects::MillEffect>()
            && let Some(target_only) =
                filtered[idx + 1].downcast_ref::<crate::effects::TargetOnlyEffect>()
            && let Some(exile) = filtered[idx + 2].downcast_ref::<crate::effects::ExileEffect>()
            && mill.player == PlayerFilter::target_player()
            && target_only.target == ChooseSpec::target_player()
            && !exile.face_down
            && let ChooseSpec::All(filter) = &exile.spec
            && filter.zone == Some(Zone::Graveyard)
            && matches!(&filter.owner, Some(PlayerFilter::Target(inner)) if **inner == PlayerFilter::Any)
            && filter.card_types.is_empty()
            && filter.subtypes.is_empty()
        {
            parts.push(format!(
                "Target player mills {}, then exiles their graveyard",
                describe_card_count(&mill.count)
            ));
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(mill) = filtered[idx].downcast_ref::<crate::effects::MillEffect>()
            && let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>()
            && mill.player == PlayerFilter::target_player()
            && !exile.face_down
            && let ChooseSpec::All(filter) = &exile.spec
            && filter.zone == Some(Zone::Graveyard)
            && matches!(&filter.owner, Some(PlayerFilter::Target(inner)) if **inner == PlayerFilter::Any)
            && filter.card_types.is_empty()
            && filter.subtypes.is_empty()
        {
            parts.push(format!(
                "Target player mills {}, then exiles their graveyard",
                describe_card_count(&mill.count)
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(mill) = filtered[idx].downcast_ref::<crate::effects::MillEffect>()
            && let Some(may) = filtered[idx + 1].downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) = describe_mill_then_may_return(mill, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged_mill) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(mill) = tagged_mill
                .effect
                .downcast_ref::<crate::effects::MillEffect>()
            && let Some(may) = filtered[idx + 1].downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) =
                describe_tagged_mill_then_may_put_milled_card_into_hand(tagged_mill, mill, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) =
                filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) =
                filtered[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) =
                describe_may_tagged_mill_then_if_do_put_milled_cards(with_id, if_effect)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(tagged_mill) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(mill) = tagged_mill
                .effect
                .downcast_ref::<crate::effects::MillEffect>()
            && let Some(with_id) = filtered[idx + 1].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = filtered[idx + 2].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) =
                describe_tagged_mill_then_payment_if_you_do_put_milled_card_into_hand(
                    tagged_mill,
                    mill,
                    with_id,
                    if_effect,
                )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some(tagged_mill) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(mill) = tagged_mill
                .effect
                .downcast_ref::<crate::effects::MillEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((Some(move_to_hand_with_id), move_to_hand)) =
                for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(if_effect) = filtered[idx + 3].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) = describe_tagged_mill_then_put_milled_card_into_hand_with_fallback(
                tagged_mill,
                mill,
                choose,
                move_to_hand_with_id,
                move_to_hand,
                if_effect,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 3 < filtered.len()
            && let Some((source_tag, mill)) = mill_with_collection_tag(filtered[idx])
            && let Some(first_choice) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(second_choice) =
                filtered[idx + 2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_chosen)) =
                for_each_tagged_for_compaction(filtered[idx + 3])
            && let Some(compact) = describe_mill_then_put_milled_cards(
                source_tag.as_str(),
                mill,
                &[first_choice, second_choice],
                move_chosen,
            )
        {
            parts.push(compact);
            idx += 4;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some((source_tag, mill)) = mill_with_collection_tag(filtered[idx])
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some((_, move_chosen)) = for_each_tagged_for_compaction(filtered[idx + 2])
            && let Some(compact) = describe_mill_then_put_milled_cards(
                source_tag.as_str(),
                mill,
                &[choose],
                move_chosen,
            )
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(for_players) =
                filtered[idx].downcast_ref::<crate::effects::ForPlayersEffect>()
            && for_players.filter == PlayerFilter::Opponent
            && for_players.effects.len() == 1
            && let Some(deal) =
                for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()
            && matches!(
                deal.target,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer)
            )
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
            && gain.amount == deal.amount
            && !deal.source_is_combat
            && matches!(deal.amount, Value::Count(_))
        {
            let amount_text = describe_value(&deal.amount);
            parts.push(format!(
                "it deals X damage to each opponent and you gain X life, where X is {amount_text}"
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(deal) = deal_damage_effect_view(filtered[idx])
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(compact) = describe_deal_damage_then_gain_life(deal, gain)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(for_players) =
                filtered[idx].downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(compact) = describe_for_players_lose_life_then_gain_life(for_players, gain)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(lose) = filtered[idx].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(compact) = describe_lose_life_then_gain_life(lose, gain)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(gain) = filtered[idx].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(scry) = filtered[idx + 1].downcast_ref::<crate::effects::ScryEffect>()
            && let Some(compact) = describe_gain_life_then_scry(gain, scry)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(scry) = filtered[idx].downcast_ref::<crate::effects::ScryEffect>()
            && let Some(draw) = filtered[idx + 1].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(compact) = describe_scry_then_draw(scry, draw)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_may_search_basic_land_then_shuffle(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(may) = filtered[idx].downcast_ref::<crate::effects::MayEffect>()
            && may.decider.is_none()
            && let Some(shuffle) =
                filtered[idx + 1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(compact) = describe_may_search_choose_for_each_with_shuffle(may, shuffle)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(may) = filtered[idx].downcast_ref::<crate::effects::MayEffect>()
            && let Some(conditional) =
                filtered[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(shuffle) =
                filtered[idx + 2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(compact) =
                describe_may_search_reveal_conditional_move_then_shuffle(may, conditional, shuffle)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(may) = filtered[idx].downcast_ref::<crate::effects::MayEffect>()
            && let Some(conditional) =
                filtered[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(compact) =
                describe_may_search_reveal_shuffle_then_conditional_move(may, conditional)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(may) = filtered[idx].downcast_ref::<crate::effects::MayEffect>()
            && let Some(if_effect) = filtered[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition.0 == 0
            && matches!(if_effect.predicate, EffectPredicate::Happened)
            && if_effect.else_.is_empty()
        {
            let setup = describe_effect(filtered[idx]);
            let followup = lowercase_first(&describe_effect_list(&if_effect.then));
            if !setup.is_empty() && !followup.is_empty() {
                let condition = describe_may_have_source_deal_damage_condition(may, if_effect)
                    .unwrap_or_else(|| match may.decider.as_ref() {
                        None | Some(PlayerFilter::You) => "If you do".to_string(),
                        Some(PlayerFilter::Target(inner))
                            if matches!(inner.as_ref(), PlayerFilter::Opponent) =>
                        {
                            "If they do".to_string()
                        }
                        Some(player) => {
                            let player_text = describe_player_filter(player);
                            format!("If {player_text} does")
                        }
                    });
                parts.push(format!("{setup}. {condition}, {followup}"));
                idx += 2;
                continue;
            }
        }
        if let Some((compact, consumed)) =
            describe_longest_conjoined_counter_or_draw_sequence(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_result_producer_then_for_each_tagged(
                filtered[idx],
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        let mut rendered = describe_effect(filtered[idx]);
        if !rendered.is_empty() {
            let is_your_turn_followup = filtered[idx]
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .is_some_and(|conditional| conditional.condition == Condition::YourTurn);
            if !parts.is_empty() && rendered.starts_with("If ") && !is_your_turn_followup {
                rendered = format!("Then {}", lowercase_first(&rendered));
                if let Some(comma_idx) = rendered.find(", ") {
                    let tail = lowercase_first(&rendered[comma_idx + 2..]);
                    rendered = format!("{}, {tail}", &rendered[..comma_idx]);
                }
            }
            parts.push(rendered);
        }
        idx += 1;
}
