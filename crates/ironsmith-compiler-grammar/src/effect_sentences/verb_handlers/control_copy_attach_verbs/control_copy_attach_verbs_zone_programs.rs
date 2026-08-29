use super::*;


pub fn parse_put_into_hand(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if let Some(choice) = parse_put_destination_choice(tokens, subject)? {
        return Ok(choice);
    }
    let authored_tokens = tokens;
    let tokens = if tokens
        .first()
        .is_some_and(|token| token.is_word("put") || token.is_word("puts"))
    {
        &tokens[1..]
    } else {
        tokens
    };
    fn parse_put_into_hand_delayed_timing(
        tokens: &[OwnedLexToken],
    ) -> Option<DelayedReturnTimingAst> {
        let tail_tokens = cca_shapes::parse_delayed_hand_tail(tokens)?;
        let tail_words = crate::lexer::token_word_refs(tail_tokens);
        parse_delayed_return_timing_words(&tail_words)
    }

    fn force_object_targeting(target: TargetAst, span: TextSpan) -> TargetAst {
        match target {
            TargetAst::Object(filter, explicit_span, fixed_span) => {
                TargetAst::Object(filter, explicit_span.or(Some(span)), fixed_span)
            }
            TargetAst::WithCount(inner, count) => {
                TargetAst::WithCount(Box::new(force_object_targeting(*inner, span)), count)
            }
            other => other,
        }
    }

    fn expand_graveyard_or_hand_disjunction(
        mut target: TargetAst,
        target_tokens: &[OwnedLexToken],
    ) -> TargetAst {
        if !cca_shapes::contains_graveyard_and_hand(target_tokens) {
            return target;
        }

        // Parse the characteristic prefix independently from the zone
        // disjunction.  Otherwise the generic filter parser can put the
        // Aura/Equipment (or other type) union inside `any_of`, and clearing
        // that union while expanding the two zones silently drops it.
        if let Some(from_index) = crate::slice_primitives::select_position(target_tokens, |token| {
            token
                .as_word()
                .is_some_and(|word| word.eq_ignore_ascii_case("from"))
        }) && from_index > 0
            && let Ok(base) = parse_target_phrase(&target_tokens[..from_index])
        {
            target = base;
        }

        let target_words = crate::lexer::token_word_refs(target_tokens);
        let owner = crate::slice_primitives::find_window_by(&target_words, 2, |pair| {
            pair[0].eq_ignore_ascii_case("your")
                && (pair[1].eq_ignore_ascii_case("hand")
                    || pair[1].eq_ignore_ascii_case("graveyard"))
        })
        .is_some();

        fn apply(filter: &ObjectFilter, owner: bool) -> ObjectFilter {
            let mut hand = filter.clone();
            hand.any_of.clear();
            hand.zone = Some(Zone::Hand);
            if owner {
                hand.owner = Some(PlayerFilter::You);
            }

            let mut graveyard = filter.clone();
            graveyard.any_of.clear();
            graveyard.zone = Some(Zone::Graveyard);
            if owner {
                graveyard.owner = Some(PlayerFilter::You);
            }

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![hand, graveyard];
            disjunction
        }

        match &mut target {
            TargetAst::Object(filter, _, _) => {
                *filter = apply(filter, owner);
            }
            TargetAst::WithCount(inner, _) => {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    *filter = apply(filter, owner);
                }
            }
            _ => {}
        }

        target
    }

    fn apply_source_zone_constraint(target: &mut TargetAst, zone: Zone) {
        match target {
            TargetAst::Source(span) => {
                *target = TargetAst::Object(ObjectFilter::source().in_zone(zone), *span, None);
            }
            TargetAst::Object(filter, _, _) => {
                filter.zone = Some(zone);
            }
            TargetAst::WithCount(inner, _) => apply_source_zone_constraint(inner, zone),
            _ => {}
        }
    }

    fn apply_explicit_source_location(target: &mut TargetAst, tokens: &[OwnedLexToken]) {
        let words = crate::lexer::token_word_refs(tokens);
        let location = if crate::word_primitives::sequence_occurs(
            &words,
            &["from", "your", "hand"],
        ) {
            Some((Zone::Hand, Some(PlayerFilter::You)))
        } else if crate::word_primitives::sequence_occurs(
            &words,
            &["from", "your", "graveyard"],
        ) {
            Some((Zone::Graveyard, Some(PlayerFilter::You)))
        } else if crate::word_primitives::sequence_occurs(
            &words,
            &["from", "your", "library"],
        ) {
            Some((Zone::Library, Some(PlayerFilter::You)))
        } else if crate::word_primitives::sequence_occurs(
            &words,
            &["from", "the", "command", "zone"],
        ) {
            Some((Zone::Command, Some(PlayerFilter::You)))
        } else {
            None
        };
        let Some((zone, owner)) = location else {
            return;
        };

        apply_source_zone_constraint(target, zone);
        if let Some(owner) = owner
            && let Some(filter) = crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(target)
        {
            filter.owner = Some(owner);
        }
    }

    fn strip_source_top_only_prefix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
        use winnow::Parser as _;

        crate::grammar::primitives::parse_prefix(
            tokens,
            crate::grammar::primitives::phrase(&["the", "top"]).void(),
        )
        .map(|(_, rest)| (rest, true))
        .unwrap_or((tokens, false))
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::lexer::token_word_refs(tokens);
    let exiled_with_source_surface = parse_exiled_with_source_move_surface(authored_tokens);

    if let Some(shape) = cca_shapes::parse_revealed_remainder_shape(tokens) {
        let order = if shape.random_order {
            crate::cards::builders::LibraryBottomOrderAst::Random
        } else {
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
        };
        return Ok(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
                crate::tag::CompilerReferenceTag::LastRevealed.key(),
                shape
                    .exclude_current_reference
                    .then(|| TagKey::from(IT_TAG)),
                order,
                cca_shapes::parse_destination_player(tokens).unwrap_or(player),
                shape.surface,
            ),
        );
    }

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if cca_shapes::is_reorder_tagged_cards(tokens) {
        return Ok(EffectAst::subject_verb_reorder_top_of_library(
            TagKey::from(IT_TAG),
        ));
    }

    if let Some(shape) = cca_shapes::parse_tagged_battlefield_partition_shape(tokens) {
        let collection_tag = crate::util::helper_tag_for_tokens(
            tokens,
            "partition_pool",
        );
        let chosen_tag = crate::util::helper_tag_for_tokens(
            tokens,
            "partition_chosen",
        );
        let owner = crate::activation_and_restrictions::controller_filter_for_token_player(player)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "battlefield collection partition has no resolvable player (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

        let mut collection_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        collection_filter.zone = Some(Zone::Library);
        collection_filter.owner = Some(owner.clone());
        let capture_collection = EffectAst::subject_verb_tag_matching_objects(
            collection_filter,
            vec![Zone::Library],
            collection_tag.clone(),
        );

        let mut choose_filter = ObjectFilter::tagged(collection_tag.clone());
        choose_filter.zone = Some(Zone::Library);
        choose_filter.owner = Some(owner.clone());
        let choose = EffectAst::ChooseTaggedObjectsInZone {
            filter: choose_filter,
            count: shape.count,
            player,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        };

        let mut chosen_filter = ObjectFilter::tagged(chosen_tag.clone());
        chosen_filter.zone = Some(Zone::Library);
        chosen_filter.owner = Some(owner.clone());
        let chosen_controller = match shape.chosen_controller {
            cca_shapes::PartitionBattlefieldControllerShape::You => ReturnControllerAst::You,
            cca_shapes::PartitionBattlefieldControllerShape::SubjectPlayer => {
                ReturnControllerAst::Owner
            }
        };
        let move_chosen = EffectAst::subject_verb_put_all_onto_battlefield(
            chosen_filter,
            shape.chosen_tapped,
            false,
            chosen_controller,
        );

        let mut remainder_filter = ObjectFilter::tagged(collection_tag);
        remainder_filter.zone = Some(Zone::Library);
        remainder_filter.owner = Some(owner);
        remainder_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: chosen_tag,
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
        let remainder_controller = match shape.remainder_controller {
            cca_shapes::PartitionBattlefieldControllerShape::You => ReturnControllerAst::You,
            cca_shapes::PartitionBattlefieldControllerShape::SubjectPlayer => {
                ReturnControllerAst::Owner
            }
        };
        let move_remainder = EffectAst::subject_verb_put_all_onto_battlefield(
            remainder_filter,
            shape.remainder_tapped,
            false,
            remainder_controller,
        );

        return Ok(EffectAst::Sequence {
            effects: vec![capture_collection, choose, move_chosen, move_remainder],
        });
    }

    let from_among_shape = cca_shapes::parse_from_among_them_shape(tokens);
    if let Some(shape) = from_among_shape
        && shape.destination == cca_shapes::FromAmongDestinationShape::Battlefield
    {
        let filter = crate::effect_sentences::parse_looked_card_choice_filter(
            shape.filter_tokens,
        )
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to parse from-among hand filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let looked_tag = crate::util::helper_tag_for_tokens(
            tokens, "looked",
        );
        let chosen_tag = crate::util::helper_tag_for_tokens(
            tokens, "chosen",
        );
        let effects = if shape.rest_destination == Some(cca_shapes::RestDestinationShape::Hand) {
            compose_put_filtered_looked_cards_to_zone_rest_to_zone(
                player,
                filter,
                shape.count,
                looked_tag,
                chosen_tag,
                Zone::Battlefield,
                Zone::Hand,
            )
        } else {
            compose_put_filtered_looked_cards_to_zone(
                player,
                filter,
                shape.count,
                looked_tag,
                chosen_tag,
                Zone::Battlefield,
            )
        };
        return Ok(EffectAst::Sequence { effects });
    }
    if cca_shapes::has_from_among_hand_surface(tokens) {
        let looked_tag = crate::util::helper_tag_for_tokens(
            tokens, "looked",
        );
        let chosen_tag = crate::util::helper_tag_for_tokens(
            tokens, "chosen",
        );
        if let Some(shape) = from_among_shape {
            let filter = crate::effect_sentences::parse_looked_card_choice_filter(
                shape.filter_tokens,
            )
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to parse from-among hand filter (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            return Ok(EffectAst::Sequence {
                effects: compose_put_filtered_looked_cards_into_hand_rest_into_graveyard(
                    player,
                    filter,
                    shape.count,
                    looked_tag,
                    chosen_tag,
                ),
            });
        }
        return Ok(EffectAst::Sequence {
            effects: EffectAst::compose_put_some_into_hand_rest_into_graveyard(
                player,
                crate::effect::ChoiceCount::exactly(1),
                looked_tag,
                chosen_tag,
            ),
        });
    }

    if let Some(filter_tokens) = cca_shapes::parse_all_exiled_into_hand_filter(tokens) {
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(wrap_return_with_delayed_timing(
            EffectAst::subject_verb_return_all_to_hand(filter)
                .with_exiled_with_source_surface(exiled_with_source_surface.clone()),
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // "Put one of those cards on top of your library and the rest on the bottom of your library"
    if let Some(shape) = cca_shapes::parse_tagged_on_top_library_shape(tokens) {
        let library_owner = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
        let looked_tag = crate::util::helper_tag_for_tokens(
            tokens, "looked",
        );
        let chosen_tag = crate::util::helper_tag_for_tokens(
            tokens, "chosen",
        );

        return Ok(EffectAst::Sequence {
            effects: EffectAst::compose_put_some_on_top_rest_on_bottom_of_library(
                library_owner,
                shape.count,
                looked_tag,
                chosen_tag,
                shape.bottom_order,
            ),
        });
    }

    if let Some(put_shape) = cca_shapes::parse_tagged_into_hand_shape(tokens) {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if put_shape.rest_destination == Some(cca_shapes::RestDestinationShape::BottomOfLibrary)
            && let Some(choice_count) = put_shape.count
            && let Some(bottom_order) = put_shape.bottom_order
        {
            let dest_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
            let looked_tag = crate::util::helper_tag_for_tokens(
                tokens, "looked",
            );
            let chosen_tag = crate::util::helper_tag_for_tokens(
                tokens, "chosen",
            );

            return Ok(EffectAst::Sequence {
                effects: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                    dest_player,
                    choice_count,
                    looked_tag,
                    chosen_tag,
                    bottom_order,
                ),
            });
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if put_shape.rest_destination == Some(cca_shapes::RestDestinationShape::Graveyard)
            && let Some(choice_count) = put_shape.count
        {
            // The chooser is typically the player whose hand is referenced.
            let dest_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
            let looked_tag = crate::util::helper_tag_for_tokens(
                tokens, "looked",
            );
            let chosen_tag = crate::util::helper_tag_for_tokens(
                tokens, "chosen",
            );

            return Ok(EffectAst::Sequence {
                effects: EffectAst::compose_put_some_into_hand_rest_into_graveyard(
                    dest_player,
                    choice_count,
                    looked_tag,
                    chosen_tag,
                ),
            });
        }

        let destination_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
        let tagged = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
        let target = put_shape
            .count
            .map(|count| TargetAst::WithCount(Box::new(tagged.clone()), count))
            .unwrap_or(tagged);
        let effect = EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_destination_player_surface(Some(destination_player))
        .with_move_to_zone_actor_surface(player)
        .with_move_to_zone_plural_surface_if(put_shape.plural_reference);
        return Ok(wrap_return_with_delayed_timing(
            effect,
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // Support destination-first wording:
    // "Put onto the battlefield under your control all creature cards ..."
    if let Some(shape) = cca_shapes::parse_destination_first_battlefield_shape(tokens) {
        let battlefield_controller = shape
            .controller
            .map(cca_controller)
            .unwrap_or(ReturnControllerAst::Preserve);
        match shape.target {
            cca_shapes::DestinationFirstTargetShape::Attached {
                attachment_target_tokens,
                object_tokens,
            } => {
                let attachment_target = parse_target_phrase(attachment_target_tokens)?;
                let mut object_target = parse_target_phrase(object_tokens)?;
                object_target = expand_graveyard_or_hand_disjunction(object_target, object_tokens);
                object_target = force_object_targeting(object_target, tokens[0].span());
                return Ok(EffectAst::subject_verb_move_to_zone(
                    object_target,
                    Zone::Battlefield,
                    false,
                    battlefield_controller,
                    shape.tapped,
                    Some(attachment_target),
                ));
            }
            cca_shapes::DestinationFirstTargetShape::Objects(target_tokens) => {
                if cca_shapes::starts_with_all_or_each(target_tokens) {
                    let filter = parse_object_filter(&target_tokens[1..], false)?;
                    return Ok(EffectAst::subject_verb_put_all_onto_battlefield(
                        filter,
                        shape.tapped,
                        shape.face_down,
                        battlefield_controller,
                    ));
                }
                let span = tokens[0].span();
                let mut rewritten = target_tokens.to_vec();
                rewritten.push(OwnedLexToken::word("onto".to_string(), span));
                rewritten.push(OwnedLexToken::word("battlefield".to_string(), span));
                if shape.tapped {
                    rewritten.push(OwnedLexToken::word("tapped".to_string(), span));
                }
                if shape.face_down {
                    rewritten.push(OwnedLexToken::word("face".to_string(), span));
                    rewritten.push(OwnedLexToken::word("down".to_string(), span));
                }
                match shape.controller {
                    Some(cca_shapes::BattlefieldControllerShape::You) => {
                        rewritten.push(OwnedLexToken::word("under".to_string(), span));
                        rewritten.push(OwnedLexToken::word("your".to_string(), span));
                        rewritten.push(OwnedLexToken::word("control".to_string(), span));
                    }
                    Some(cca_shapes::BattlefieldControllerShape::Owner) => {
                        rewritten.push(OwnedLexToken::word("under".to_string(), span));
                        rewritten.push(OwnedLexToken::word("its".to_string(), span));
                        rewritten.push(OwnedLexToken::word("owner".to_string(), span));
                        rewritten.push(OwnedLexToken::word("control".to_string(), span));
                    }
                    None => {}
                }
                return parse_put_into_hand(&rewritten, subject);
            }
        }
    }

    if let Some(shape) = cca_shapes::parse_library_choice_destination_shape(tokens) {
        let target = if let Some(target) = parse_counted_card_target_prefix(shape.target_tokens)? {
            target
        } else {
            parse_target_phrase(shape.target_tokens)?
        };
        return Ok(EffectAst::subject_verb_move_to_library_top_or_bottom_choice(target));
    }

    if let Some(shape) = cca_shapes::parse_library_placement_destination_shape(tokens) {
        let (target_tokens, source_top_only) = strip_source_top_only_prefix(shape.target_tokens);
        let target = if let Some(target) = parse_counted_card_target_prefix(target_tokens)? {
            target
        } else {
            parse_target_phrase(target_tokens)?
        };
        let moves_all = cca_shapes::starts_with_all_or_each(target_tokens)
            || cca_shapes::is_exhaustive_hand_collection(target_tokens);
        let order = shape.order.map(|order| match order {
            cca_shapes::LibraryPlacementOrderShape::Random => {
                crate::cards::builders::LibraryBottomOrderAst::Random
            }
            cca_shapes::LibraryPlacementOrderShape::ChooserChooses => {
                crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
            }
        });
        let effect = if moves_all {
            EffectAst::subject_verb_move_all_to_zone(
                target,
                Zone::Library,
                shape.placement == cca_shapes::LibraryPlacementShape::Top,
                ReturnControllerAst::Preserve,
                false,
                None,
            )
        } else {
            EffectAst::subject_verb_move_to_zone(
                target,
                Zone::Library,
                shape.placement == cca_shapes::LibraryPlacementShape::Top,
                ReturnControllerAst::Preserve,
                false,
                None,
            )
        };
        return Ok(effect
            .with_source_top_only(source_top_only)
            .with_library_order(order, player)
            .with_destination_player_surface(cca_shapes::parse_destination_player(
                shape.destination_tokens,
            ))
            .with_destination_player_reference_surface(
                cca_shapes::parse_destination_player_reference_surface(shape.destination_tokens),
            )
            .with_exiled_with_source_surface(exiled_with_source_surface.clone())
            .with_move_to_zone_actor_surface(player)
            .with_move_to_zone_plural_surface_if(cca_shapes::is_plural_tagged_object_reference(
                target_tokens,
            )));
    }

    if let Some(shape) = cca_shapes::parse_into_destination_shape(tokens) {
        let destination_player_surface =
            cca_shapes::parse_destination_player(shape.destination_tokens);
        let destination_player_reference_surface =
            cca_shapes::parse_destination_player_reference_surface(shape.destination_tokens);
        let zone = if let Some(zone) = shape.zone {
            Some(zone)
        } else if let Some(position) =
            parse_library_nth_from_top_destination(shape.destination_tokens)
        {
            let target = parse_target_phrase(shape.target_tokens)?;
            return Ok(EffectAst::subject_verb_move_to_library_nth_from_top(
                target, position,
            ));
        } else {
            None
        };

        if let Some(zone) = zone {
            let delayed_hand_timing = if zone == Zone::Hand {
                parse_put_into_hand_delayed_timing(tokens)
            } else {
                None
            };
            if zone == Zone::Graveyard && cca_shapes::is_rest_reference(shape.target_tokens) {
                return Ok(EffectAst::subject_verb_move_to_zone(
                    TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), None, None),
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
                .with_destination_player_surface(destination_player_surface)
                .with_destination_player_reference_surface(destination_player_reference_surface)
                .with_move_to_zone_actor_surface(player));
            }

            if zone == Zone::Hand {
                if let Some(count) = cca_shapes::parse_counted_those_cards(shape.target_tokens)
                    && cca_shapes::parse_rest_destination(shape.destination_tokens)
                        == Some(cca_shapes::RestDestinationShape::Graveyard)
                {
                    let dest_player =
                        cca_shapes::parse_destination_player(tokens).unwrap_or(player);
                    let looked_tag =
                        crate::util::helper_tag_for_tokens(
                            tokens, "looked",
                        );
                    let chosen_tag =
                        crate::util::helper_tag_for_tokens(
                            tokens, "chosen",
                        );

                    return Ok(EffectAst::Sequence {
                        effects: EffectAst::compose_put_some_into_hand_rest_into_graveyard(
                            dest_player,
                            crate::effect::ChoiceCount::exactly(count as usize),
                            looked_tag,
                            chosen_tag,
                        ),
                    });
                }

                if cca_shapes::is_tagged_object_reference(shape.target_tokens) {
                    if cca_shapes::explicitly_names_object_owner(shape.destination_tokens) {
                        let effect = EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(
                                TagKey::from(IT_TAG),
                                span_from_tokens(shape.target_tokens),
                            ),
                            Zone::Hand,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )
                        .with_move_to_zone_actor_surface(player)
                        .with_move_to_zone_plural_surface_if(
                            cca_shapes::is_plural_tagged_object_reference(shape.target_tokens),
                        );
                        return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                    }
                    let destination_player = destination_player_surface.unwrap_or(player);
                    let effect = EffectAst::subject_verb_put_into_hand(
                        destination_player,
                        ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    )
                    .with_move_to_zone_actor_surface(player)
                    .with_move_to_zone_plural_surface_if(
                        cca_shapes::is_plural_tagged_object_reference(shape.target_tokens),
                    );
                    return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                }
            }

            let (target_tokens, source_top_only) =
                strip_source_top_only_prefix(shape.target_tokens);
            let mut target = preserve_exiled_with_source_subject_cardinality(
                parse_target_phrase(target_tokens)?,
                exiled_with_source_surface.as_ref(),
            );
            apply_explicit_source_location(&mut target, tokens);
            let effect = if cca_shapes::starts_with_all_or_each(target_tokens) {
                EffectAst::subject_verb_move_all_to_zone(
                    target,
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            } else {
                EffectAst::subject_verb_move_to_zone(
                    target,
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            }
            .with_source_top_only(source_top_only)
            .with_destination_player_surface(destination_player_surface)
            .with_destination_player_reference_surface(destination_player_reference_surface)
            .with_exiled_with_source_surface(exiled_with_source_surface.clone())
            .with_move_to_zone_actor_surface(player)
            .with_move_to_zone_plural_surface_if(
                cca_shapes::is_plural_tagged_object_reference(target_tokens),
            );
            return Ok(if zone == Zone::Hand {
                wrap_return_with_delayed_timing(effect, delayed_hand_timing)
            } else {
                effect
            });
        }
    }

    if let Some(onto_shape) = cca_shapes::parse_onto_clause_shape(tokens) {
        let target_tokens = onto_shape.target_tokens;
        let (destination_slice, trailing_predicate) =
            if let Some(spec) = split_trailing_if_clause_lexed(onto_shape.destination_tokens) {
                (spec.leading_tokens, Some(spec.predicate))
            } else {
                (onto_shape.destination_tokens, None)
            };
        let destination_shape = cca_shapes::parse_onto_battlefield_destination_shape(
            destination_slice,
        )
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let attached_to_target = destination_shape
            .attached_to_tokens
            .as_deref()
            .map(parse_target_phrase)
            .transpose()?;

        if let Some(rest_target_tokens) = destination_shape.rest_graveyard_target.as_deref() {
            let primary_target = if cca_shapes::is_tagged_object_reference(target_tokens) {
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
            } else {
                parse_target_phrase(target_tokens)?
            };
            let primary_effect = EffectAst::subject_verb_move_to_zone_with_attacking(
                primary_target,
                Zone::Battlefield,
                false,
                ReturnControllerAst::Preserve,
                destination_shape.tapped,
                destination_shape.attacking,
                destination_shape.face_down,
                attached_to_target.clone(),
            )
            .with_exiled_with_source_surface(exiled_with_source_surface.clone());
            let rest_target = parse_target_phrase(rest_target_tokens)?;
            let rest_effect = if cca_shapes::starts_with_all_or_each(rest_target_tokens) {
                EffectAst::subject_verb_move_all_to_zone(
                    rest_target,
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            } else {
                EffectAst::subject_verb_move_to_zone(
                    rest_target,
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            };
            let effect = EffectAst::Sequence {
                effects: vec![primary_effect, rest_effect],
            };
            return Ok(if let Some(predicate) = trailing_predicate {
                EffectAst::TrailingIf {
                    predicate,
                    effects: vec![effect],
                }
            } else {
                effect
            });
        }

        if !destination_shape.supported_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = destination_shape
            .controller
            .map(cca_controller)
            .unwrap_or(ReturnControllerAst::Preserve);

        if let Some(choice_shape) =
            crate::grammar::choices::parse_possessive_object_choice_tokens(
                target_tokens,
            )
        {
            use crate::grammar::choices::PossessiveObjectChoiceActor;

            let chooser = match choice_shape.actor {
                PossessiveObjectChoiceActor::You => Some(PlayerAst::You),
                PossessiveObjectChoiceActor::SubjectPlayer => {
                    extract_subject_player(subject)
                }
                PossessiveObjectChoiceActor::Opponent => Some(PlayerAst::Opponent),
                // An object-controller phrase needs a previously established
                // object antecedent; leave it to the ordinary target path when
                // the selected object itself would be circular.
                PossessiveObjectChoiceActor::ObjectController => None,
            };
            if let Some(chooser) = chooser {
                let parsed_target = parse_target_phrase(&choice_shape.object_tokens)?;
                let (mut filter, count) = match parsed_target {
                    TargetAst::Object(filter, _, _) => {
                        (filter, crate::effect::ChoiceCount::exactly(1))
                    }
                    TargetAst::WithCount(inner, count) => match *inner {
                        TargetAst::Object(filter, _, _) => (filter, count),
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "choice-owned battlefield move requires an object (clause: '{}')",
                                clause_words.join(" ")
                            )));
                        }
                    },
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "choice-owned battlefield move requires an object (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                };
                if let Some(choice_owner) = crate::activation_and_restrictions::controller_filter_for_token_player(
                    chooser,
                ) {
                    if filter.owner == Some(PlayerFilter::IteratedPlayer) {
                        filter.owner = Some(choice_owner.clone());
                    }
                    if filter.controller == Some(PlayerFilter::IteratedPlayer) {
                        filter.controller = Some(choice_owner);
                    }
                }
                let tag =
                    crate::util::helper_tag_for_tokens(
                        target_tokens,
                        "chosen",
                    );
                let choose = EffectAst::ChooseObjects {
                    filter,
                    count,
                    count_value: None,
                    player: chooser,
                    tag: tag.clone(),
                };
                let move_chosen = EffectAst::subject_verb_move_to_zone_with_attacking(
                    TargetAst::Tagged(tag, span_from_tokens(target_tokens)),
                    Zone::Battlefield,
                    false,
                    battlefield_controller,
                    destination_shape.tapped,
                    destination_shape.attacking,
                    destination_shape.face_down,
                    attached_to_target.clone(),
                )
                .with_exiled_with_source_surface(exiled_with_source_surface.clone());
                let effect = EffectAst::Sequence {
                    effects: vec![choose, move_chosen],
                };
                return Ok(if let Some(predicate) = trailing_predicate {
                    EffectAst::TrailingIf {
                        predicate,
                        effects: vec![effect],
                    }
                } else {
                    effect
                });
            }
        }

        if cca_shapes::starts_with_all_or_each(target_tokens) {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            if cca_shapes::contains_from_it(&target_tokens[1..]) {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter
                    .tagged_constraints
                    .retain(|constraint| constraint.tag.as_str() != IT_TAG);
            }
            if cca_shapes::contains_among_them(tokens) {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if cca_shapes::contains_permanent(tokens) {
                    filter.card_types = vec![
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                        CardType::Battle,
                    ];
                }
            }
            let effect = EffectAst::subject_verb_put_all_onto_battlefield(
                filter,
                destination_shape.tapped,
                destination_shape.face_down,
                battlefield_controller,
            )
            .with_exiled_with_source_surface(exiled_with_source_surface.clone());
            return Ok(if let Some(predicate) = trailing_predicate {
                EffectAst::TrailingIf {
                    predicate,
                    effects: vec![effect],
                }
            } else {
                effect
            });
        }

        let mut target = if cca_shapes::is_tagged_object_reference(target_tokens) {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
        } else {
            parse_target_phrase(target_tokens)?
        };
        target = expand_graveyard_or_hand_disjunction(target, target_tokens);
        apply_explicit_source_location(&mut target, target_tokens);
        if !cca_shapes::target_names_unowned_shared_zone(target_tokens)
            && let Some(filter) = crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
        {
            crate::effect_sentences::zone_counter_helpers::apply_exile_subject_owner_context(filter, subject);
        }
        if destination_shape.source_from_command {
            apply_source_zone_constraint(&mut target, Zone::Command);
        }

        let effect = EffectAst::subject_verb_move_to_zone_with_attacking(
            target,
            Zone::Battlefield,
            false,
            battlefield_controller,
            destination_shape.tapped,
            destination_shape.attacking,
            destination_shape.face_down,
            attached_to_target,
        )
        .with_exiled_with_source_surface(exiled_with_source_surface)
        .with_move_to_zone_actor_surface(player)
        .with_move_to_zone_plural_surface_if(
            cca_shapes::is_plural_tagged_object_reference(target_tokens),
        );
        return Ok(if let Some(predicate) = trailing_predicate {
            EffectAst::TrailingIf {
                predicate,
                effects: vec![effect],
            }
        } else {
            effect
        });
    }

    if cca_shapes::contains_sticker(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported sticker clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}
