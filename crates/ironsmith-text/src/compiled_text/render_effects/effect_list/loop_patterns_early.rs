{
        if filtered[idx]
            .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
            .is_some_and(|tag| is_implicit_reference_tag(tag.tag.as_str()))
        {
            idx += 1;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                unwrap_basic_tag_wrappers(filtered[idx])
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                unwrap_basic_tag_wrappers(filtered[idx + 1])
                    .downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_choose_any_number_then_remove_counter_from_each(choose, for_each)
        {
            parts.push(compact);
            idx += 2;
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
            describe_revealed_top_choose_one_graveyard(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

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
            if let Some(next) = filtered.get(idx)
                && structural_unwrap_render_wrappers(next)
                    .downcast_ref::<crate::effects::DrawCardsEffect>()
                    .is_some_and(|draw| draw.player == PlayerFilter::You)
            {
                parts.push(normalize_imperative_you_clause(
                    describe_effect(next).trim().trim_end_matches('.'),
                ));
                idx += 1;
            }
            continue;
        }

        if let Some((rendered, consumed)) = describe_self_look_reorder(&filtered[idx..]) {
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

        if let [look_effect, may_effect, remainder_effect, ..] = &filtered[idx..]
            && let Some(look_at_top) =
                look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
            && let Some(remainder) = remainder_effect
                .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
            && let Some(rendered) =
                describe_looked_may_one_top_rest_bottom(look_at_top, may, remainder)
        {
            parts.push(rendered);
            idx += 3;
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

        // "Two target players each reveal the top card of their library. They
        // each lose life equal to the mana value of the card revealed by the
        // other player. Then they each put the card they revealed into their
        // hand." (Parker Luck): three loops over the same target players
        // reading one shared revealed pool.
        if let [reveal_effect, lose_effect, target_effect, return_effect, ..] = &filtered[idx..]
            && let Some(reveal_loop) = structural_unwrap_render_wrappers(reveal_effect)
                .downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(lose_loop) = structural_unwrap_render_wrappers(lose_effect)
                .downcast_ref::<crate::effects::ForPlayersEffect>()
            && let Some(target_only) = structural_unwrap_render_wrappers(target_effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
            && let Some(return_loop) = structural_unwrap_render_wrappers(return_effect)
                .downcast_ref::<crate::effects::ForPlayersEffect>()
            && [&reveal_loop.filter, &lose_loop.filter, &return_loop.filter]
                .iter()
                .all(|filter| matches!(filter, PlayerFilter::Target(inner) if **inner == PlayerFilter::Any))
            && let ChooseSpec::WithCount(inner_target, count) = &target_only.target
            && matches!(inner_target.base(), ChooseSpec::Player(PlayerFilter::Any))
            && count.min >= 2
            && count.max == Some(count.min)
            && let [reveal] = reveal_loop.effects.as_slice()
            && let Some(reveal) = structural_unwrap_render_wrappers(reveal)
                .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && reveal.reveal
            && reveal.player == PlayerFilter::IteratedPlayer
            && reveal.count == Value::Fixed(1)
            && let [lose] = lose_loop.effects.as_slice()
            && let Some(lose) = structural_unwrap_render_wrappers(lose)
                .downcast_ref::<crate::effects::LoseLifeEffect>()
            && matches!(lose.player, ChooseSpec::Player(PlayerFilter::IteratedPlayer))
            && let Value::TotalManaValue(others) = &lose.amount
            && matches!(
                others.owner.as_ref(),
                Some(PlayerFilter::Excluding { base, excluded })
                    if **base == PlayerFilter::Any && **excluded == PlayerFilter::IteratedPlayer
            )
            && others.tagged_constraints.iter().any(|constraint| constraint.tag == reveal.tag)
            && let [put] = return_loop.effects.as_slice()
            && let Some(put) = structural_unwrap_render_wrappers(put)
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && put.zone == Zone::Hand
            && let ChooseSpec::Object(own) = put.target.base()
            && own.owner == Some(PlayerFilter::IteratedPlayer)
            && own.tagged_constraints.iter().any(|constraint| constraint.tag == reveal.tag)
        {
            let count_text = small_number_word(count.min as u32)
                .unwrap_or_else(|| count.min.to_string());
            parts.push(format!(
                "{} target players each reveal the top card of their library. They each lose life equal to the mana value of the card revealed by the other player. Then they each put the card they revealed into their hand",
                capitalize_first(&count_text)
            ));
            idx += 4;
            continue;
        }

        // "Shuffle up to four cards you own from outside the game into your
        // library." (Research): an outside-the-game choice whose only consumer
        // is the following shuffle of exactly the chosen cards.
        if let [choose_effect, shuffle_effect, ..] = &filtered[idx..]
            && let Some(choose) = structural_unwrap_render_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(shuffle) = structural_unwrap_render_wrappers(shuffle_effect)
                .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()
            && choose.chooser == PlayerFilter::You
            && (choose.zone == Some(Zone::OutsideGame)
                || choose.filter.zone == Some(Zone::OutsideGame))
            && matches!(shuffle.target.base(), ChooseSpec::Tagged(_))
            && shuffle.player == PlayerFilter::You
            && let Some(selection) = describe_effect(choose_effect)
                .trim_end_matches('.')
                .strip_prefix("Choose ")
                .and_then(|rest| rest.strip_suffix(" outside the game"))
                .map(str::to_string)
        {
            parts.push(format!(
                "Shuffle {selection} from outside the game into your library"
            ));
            idx += 2;
            continue;
        }

        // "You may put a Human creature card from among them onto the
        // battlefield tapped and attacking." (Winota): an optional single (or
        // any-number) pick from the looked-at cards, each moved to the
        // battlefield.
        if let [choose_effect, move_each_effect, ..] = &filtered[idx..]
            && let Some(choose) = structural_unwrap_render_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_each) = structural_unwrap_render_wrappers(move_each_effect)
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && move_each.tag == choose.tag
            && choose.chooser == PlayerFilter::You
            && !choose.is_search
            && !choose.reveal
            && choose.count_value.is_none()
            && choose.aggregate_constraint.is_none()
            && choose.additional_zones.is_empty()
            && (choose.zone == Some(Zone::Library) || choose.filter.zone == Some(Zone::Library))
            && choose.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
            && let [move_effect] = move_each.effects.as_slice()
            && let Some(moved) = structural_unwrap_render_wrappers(move_effect)
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && moved.zone == Zone::Battlefield
            && (matches!(moved.target.base(), ChooseSpec::Iterated)
                || matches!(moved.target.base(), ChooseSpec::Tagged(t) if t.as_str() == "__it__"))
            && let Some(selection_count) = (if choose.count == crate::effect::ChoiceCount::up_to(1) {
                Some(false)
            } else if choose.count.is_any_number() {
                Some(true)
            } else {
                None
            })
            && let Some(placement) = describe_effect(move_effect)
                .trim_end_matches('.')
                .strip_prefix("Put it ")
                .map(str::to_string)
            && placement.starts_with("onto the battlefield")
        {
            let mut card = choose.filter.clone();
            card.zone = None;
            card.tagged_constraints.retain(|constraint| {
                constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            });
            let card_text = describe_revealed_selection_with_cards(&card.description());
            let selection = if selection_count {
                format!("any number of {}", pluralize_noun_phrase(&card_text))
            } else {
                with_indefinite_article(strip_indefinite_article(&card_text))
            };
            if !selection.contains("this way") {
                parts.push(format!("You may put {selection} from among them {placement}"));
                idx += 2;
                continue;
            }
        }

        // "Put all Goblin creature cards with mana value 5 or less from among
        // them onto the battlefield and the rest on the bottom of your library
        // in a random order." (Muxus): tag the matching looked-at cards, move
        // each, then bottom the complement.
        if let [matching_effect, move_each_effect, remainder_effect, ..] = &filtered[idx..]
            && let Some(matching) = structural_unwrap_render_wrappers(matching_effect)
                .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
            && let Some(move_each) = structural_unwrap_render_wrappers(move_each_effect)
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && matching.source_tags.is_empty()
            && matching.additional_zones.is_empty()
            && move_each.tag == matching.tag
            && matching.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
            && let Some(rest_text) = {
                let remainder_effect = structural_unwrap_render_wrappers(remainder_effect);
                if let Some(remainder) = remainder_effect
                    .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
                {
                    (remainder.keep_tagged.as_ref() == Some(&matching.tag)
                        && remainder.player == PlayerFilter::You)
                        .then(|| {
                            let order = match remainder.order {
                                crate::effects::consult_helpers::LibraryBottomOrder::Random => {
                                    "a random order"
                                }
                                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
                                    "any order"
                                }
                            };
                            format!("the rest on the bottom of your library in {order}")
                        })
                } else if let Some(rest_loop) =
                    remainder_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
                    && let [rest_conditional] = rest_loop.effects.as_slice()
                    && let Some(rest_conditional) = structural_unwrap_render_wrappers(rest_conditional)
                        .downcast_ref::<crate::effects::ConditionalEffect>()
                    && rest_conditional.if_true.is_empty()
                    && let [rest_move] = rest_conditional.if_false.as_slice()
                    && let Some(rest_move) = structural_unwrap_render_wrappers(rest_move)
                        .downcast_ref::<crate::effects::MoveToZoneEffect>()
                    && matches!(rest_move.zone, Zone::Graveyard | Zone::Hand)
                    && matches!(rest_move.target.base(), ChooseSpec::Iterated)
                    && let crate::effect::Condition::TaggedObjectMatches(it_tag, membership) =
                        &rest_conditional.condition
                    && it_tag.as_str() == "__it__"
                    && membership.tagged_constraints.iter().any(|constraint| {
                        constraint.tag == matching.tag
                    })
                {
                    // "and the rest into your hand" (Nissa, Nature's Artisan).
                    Some(if rest_move.zone == Zone::Hand {
                        "the rest into your hand".to_string()
                    } else {
                        "the rest into your graveyard".to_string()
                    })
                } else {
                    None
                }
            }
            && let [move_effect] = move_each.effects.as_slice()
            && let Some(moved) = structural_unwrap_render_wrappers(move_effect)
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && (matches!(moved.target.base(), ChooseSpec::Iterated)
                || matches!(moved.target.base(), ChooseSpec::Tagged(t) if t.as_str() == "__it__"))
            && !moved.to_top
            && moved.library_order.is_none()
            && (!moved.enters_tapped || moved.zone == Zone::Battlefield)
            && !moved.enters_attacking
            && !moved.enters_face_down
            && moved.enters_with_counters.is_empty()
            && let Some(destination) = match moved.zone {
                Zone::Hand => Some("into your hand"),
                Zone::Battlefield if moved.enters_tapped => Some("onto the battlefield tapped"),
                Zone::Battlefield => Some("onto the battlefield"),
                Zone::Graveyard => Some("into your graveyard"),
                _ => None,
            }
        {
            let mut card = matching.filter.clone();
            // The matched pool is cards in a library/graveyard ("Put all
            // cards with the chosen name from among them", Tamiyo, Collector
            // of Tales): clearing the zone must not turn an untyped card
            // filter into "permanent".
            if matches!(
                card.zone,
                Some(Zone::Library | Zone::Graveyard | Zone::Hand | Zone::Exile)
            ) && card.card_types.is_empty()
            {
                card.set_explicit_card_noun(true);
            }
            card.zone = None;
            card.tagged_constraints.retain(|constraint| {
                constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            });
            let selection = describe_revealed_selection_with_cards(&card.description());
            let plural = pluralize_noun_phrase(&selection);
            // The "revealed this way" (matched) variant is owned by the
            // grammar-side fix; only the "from among them" pool renders here.
            let pool = if crate::cards::is_sentence_helper_tag(matching.tag.as_str(), "chosen") {
                Some("from among them")
            } else {
                None
            };
            if let Some(pool) = pool
                && !plural.is_empty()
                && !plural.contains("this way")
            {
                parts.push(format!("Put all {plural} {pool} {destination} and {rest_text}"));
                idx += 3;
                continue;
            }
        }

        if let Some((rendered, consumed)) =
            describe_three_way_looked_card_partition(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_two_stage_looked_card_partition(&filtered[idx..])
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
            describe_looked_battlefield_then_conditional_remainder(&filtered[idx..])
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
            describe_look_exile_face_down_counter_rest_bottom(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_revealed_two_stage_graveyard_partition(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_look_reveal_your_turn_battlefield_else_hand_rest_bottom(&filtered[idx..])
        {
            parts.push(rendered);
            idx += consumed;
            continue;
        }

        if let Some((rendered, consumed)) =
            describe_looked_card_selected_hand_remainder_bottom(&filtered[idx..])
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
