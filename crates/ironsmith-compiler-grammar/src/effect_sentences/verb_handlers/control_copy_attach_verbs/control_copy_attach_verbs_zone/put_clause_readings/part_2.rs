//! Readers 2 of 2 of the registry in the parent module.

use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use super::*;

pub(super) fn read_tagged_into_hand(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    // "Put N of them into your hand and the rest on the bottom of your library in any order."
    // "Put N of them into your hand and the rest into your graveyard."
    // The chooser is typically the player whose hand is referenced.
    if let Some(put_shape) = cca_shapes::parse_tagged_into_hand_shape(tokens) {
        if put_shape.rest_destination == Some(cca_shapes::RestDestinationShape::BottomOfLibrary)
            && let Some(choice_count) = put_shape.count
            && let Some(bottom_order) = put_shape.bottom_order
        {
            let dest_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
            let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
            let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");

            return Ok(Some(EffectAst::Sequence {
                effects: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                    dest_player,
                    choice_count,
                    crate::tag::TagRef::of(looked_tag),
                    crate::tag::TagRef::of(chosen_tag),
                    bottom_order,
                ),
            }));
        }

        if put_shape.rest_destination == Some(cca_shapes::RestDestinationShape::Graveyard)
            && let Some(choice_count) = put_shape.count
        {
            let dest_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
            let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
            let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");

            return Ok(Some(EffectAst::Sequence {
                effects: EffectAst::compose_put_some_into_hand_rest_into_graveyard(
                    dest_player,
                    choice_count,
                    crate::tag::TagRef::of(looked_tag),
                    crate::tag::TagRef::of(chosen_tag),
                ),
            }));
        }

        let destination_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
        let tagged = TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(tokens),
        );
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
        return Ok(Some(wrap_return_with_delayed_timing(
            effect,
            parse_put_into_hand_delayed_timing(tokens),
        )));
    }
    Ok(None)
}
pub(super) fn read_destination_first_battlefield(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
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
                return Ok(Some(EffectAst::subject_verb_move_to_zone(
                    object_target,
                    Zone::Battlefield,
                    false,
                    battlefield_controller,
                    shape.tapped,
                    Some(attachment_target),
                )));
            }
            cca_shapes::DestinationFirstTargetShape::Objects(target_tokens) => {
                if cca_shapes::starts_with_all_or_each(target_tokens) {
                    let filter = parse_object_filter(&target_tokens[1..], false)?;
                    return Ok(Some(EffectAst::subject_verb_put_all_onto_battlefield(
                        filter,
                        shape.tapped,
                        shape.face_down,
                        battlefield_controller,
                    )));
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
                return parse_put_into_hand(&rewritten, subject).map(Some);
            }
        }
    }
    Ok(None)
}
pub(super) fn read_library_choice_destination(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = cca_shapes::parse_library_choice_destination_shape(tokens) {
        let target = if let Some(target) = parse_counted_card_target_prefix(shape.target_tokens)? {
            target
        } else {
            parse_target_phrase(shape.target_tokens)?
        };
        return Ok(Some(
            EffectAst::subject_verb_move_to_library_top_or_bottom_choice(target),
        ));
    }
    Ok(None)
}
pub(super) fn read_library_placement_destination(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let exiled_with_source_surface = input.exiled_with_source_surface.clone();
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
        return Ok(Some(
            effect
                .with_source_top_only(source_top_only)
                .with_library_order(order, player)
                .with_destination_player_surface(cca_shapes::parse_destination_player(
                    shape.destination_tokens,
                ))
                .with_destination_player_reference_surface(
                    cca_shapes::parse_destination_player_reference_surface(
                        shape.destination_tokens,
                    ),
                )
                .with_exiled_with_source_surface(exiled_with_source_surface.clone())
                .with_move_to_zone_actor_surface(player)
                .with_move_to_zone_plural_surface_if(
                    cca_shapes::is_plural_tagged_object_reference(target_tokens),
                ),
        ));
    }
    Ok(None)
}
pub(super) fn read_into_destination(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let exiled_with_source_surface = input.exiled_with_source_surface.clone();
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
            return Ok(Some(EffectAst::subject_verb_move_to_library_nth_from_top(
                target, position,
            )));
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
                return Ok(Some(
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Object(
                            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                            None,
                            None,
                        ),
                        zone,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )
                    .with_destination_player_surface(destination_player_surface)
                    .with_destination_player_reference_surface(destination_player_reference_surface)
                    .with_move_to_zone_actor_surface(player),
                ));
            }

            if zone == Zone::Hand {
                if let Some(count) = cca_shapes::parse_counted_those_cards(shape.target_tokens)
                    && cca_shapes::parse_rest_destination(shape.destination_tokens)
                        == Some(cca_shapes::RestDestinationShape::Graveyard)
                {
                    let dest_player =
                        cca_shapes::parse_destination_player(tokens).unwrap_or(player);
                    let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
                    let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");

                    return Ok(Some(EffectAst::Sequence {
                        effects: EffectAst::compose_put_some_into_hand_rest_into_graveyard(
                            dest_player,
                            crate::effect::ChoiceCount::exactly(count as usize),
                            crate::tag::TagRef::of(looked_tag),
                            crate::tag::TagRef::of(chosen_tag),
                        ),
                    }));
                }

                if cca_shapes::is_tagged_object_reference(shape.target_tokens) {
                    if cca_shapes::explicitly_names_object_owner(shape.destination_tokens) {
                        let effect = EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(
                                crate::tag::CompilerReferenceTag::It.bind(),
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
                        return Ok(Some(wrap_return_with_delayed_timing(
                            effect,
                            delayed_hand_timing,
                        )));
                    }
                    let destination_player = destination_player_surface.unwrap_or(player);
                    let effect = EffectAst::subject_verb_put_into_hand(
                        destination_player,
                        ObjectRefAst::Tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    )
                    .with_move_to_zone_actor_surface(player)
                    .with_move_to_zone_plural_surface_if(
                        cca_shapes::is_plural_tagged_object_reference(shape.target_tokens),
                    );
                    return Ok(Some(wrap_return_with_delayed_timing(
                        effect,
                        delayed_hand_timing,
                    )));
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
            return Ok(Some(if zone == Zone::Hand {
                wrap_return_with_delayed_timing(effect, delayed_hand_timing)
            } else {
                effect
            }));
        }
    }
    Ok(None)
}
pub(super) fn read_onto_clause(input: &PutClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let subject = input.subject;
    let clause_words = input.clause_words;
    let exiled_with_source_surface = input.exiled_with_source_surface.clone();
    // An object-controller phrase needs a previously established
    // object antecedent; leave it to the ordinary target path when
    // the selected object itself would be circular.
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
                TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    span_from_tokens(target_tokens),
                )
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
            return Ok(Some(if let Some(predicate) = trailing_predicate {
                EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                    predicate,
                    effects: vec![effect],
                })
            } else {
                effect
            }));
        }

        if !destination_shape.supported_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )))
            .map(Some);
        }
        let battlefield_controller = destination_shape
            .controller
            .map(cca_controller)
            .unwrap_or(ReturnControllerAst::Preserve);

        if let Some(choice_shape) =
            crate::grammar::choices::parse_possessive_object_choice_tokens(target_tokens)
        {
            use crate::grammar::choices::PossessiveObjectChoiceActor;

            let chooser = match choice_shape.actor {
                PossessiveObjectChoiceActor::You => Some(PlayerAst::You),
                PossessiveObjectChoiceActor::SubjectPlayer => extract_subject_player(subject),
                PossessiveObjectChoiceActor::Opponent => Some(PlayerAst::Opponent),
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
                            )))
                            .map(Some);
                        }
                    },
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "choice-owned battlefield move requires an object (clause: '{}')",
                            clause_words.join(" ")
                        )))
                        .map(Some);
                    }
                };
                if let Some(choice_owner) =
                    crate::activation_and_restrictions::controller_filter_for_token_player(chooser)
                {
                    if filter.owner == Some(PlayerFilter::IteratedPlayer) {
                        filter.owner = Some(choice_owner.clone());
                    }
                    if filter.controller == Some(PlayerFilter::IteratedPlayer) {
                        filter.controller = Some(choice_owner);
                    }
                }
                let tag = crate::util::helper_tag_for_tokens(target_tokens, "chosen");
                let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count,
                    count_value: None,
                    player: chooser,
                    tag: crate::tag::TagRef::of(tag.clone()),
                });
                let move_chosen = EffectAst::subject_verb_move_to_zone_with_attacking(
                    TargetAst::Tagged(crate::tag::TagRef::of(tag), span_from_tokens(target_tokens)),
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
                return Ok(Some(if let Some(predicate) = trailing_predicate {
                    EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                        predicate,
                        effects: vec![effect],
                    })
                } else {
                    effect
                }));
            }
        }

        if cca_shapes::starts_with_all_or_each(target_tokens) {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            if cca_shapes::contains_from_it(&target_tokens[1..]) {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter.tagged_constraints.retain(|constraint| {
                    constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
                });
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
            return Ok(Some(if let Some(predicate) = trailing_predicate {
                EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                    predicate,
                    effects: vec![effect],
                })
            } else {
                effect
            }));
        }

        let mut target = if cca_shapes::is_tagged_object_reference(target_tokens) {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(target_tokens),
            )
        } else {
            parse_target_phrase(target_tokens)?
        };
        target = expand_graveyard_or_hand_disjunction(target, target_tokens);
        apply_explicit_source_location(&mut target, target_tokens);
        if !cca_shapes::target_names_unowned_shared_zone(target_tokens)
            && let Some(filter) =
                crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
        {
            crate::effect_sentences::zone_counter_helpers::apply_exile_subject_owner_context(
                filter, subject,
            );
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
        return Ok(Some(if let Some(predicate) = trailing_predicate {
            EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                predicate,
                effects: vec![effect],
            })
        } else {
            effect
        }));
    }
    Ok(None)
}
