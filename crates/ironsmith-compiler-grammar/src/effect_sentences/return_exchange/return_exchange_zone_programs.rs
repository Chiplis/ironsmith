use super::*;

pub fn parse_return(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(for_each_idx) = crate::slice_primitives::find_last_window_by(tokens, 2, |window| {
        window[0].is_word("for") && window[1].is_word("each")
    }) {
        let count_words = crate::lexer::token_word_refs(&tokens[for_each_idx..]);
        if let Some((count, used_words)) =
            crate::util::parse_for_each_count_value_words(&count_words)
            && used_words == count_words.len()
        {
            let base_tokens = trim_commas(&tokens[..for_each_idx]);
            if !base_tokens.is_empty() {
                return Ok(EffectAst::RepeatEffects {
                    count,
                    effects: vec![parse_return(&base_tokens)?],
                });
            }
        }
    }

    let clause_text = crate::lexer::token_word_refs(tokens).join(" ");
    if let Some(unless_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("unless"))
    {
        let return_effect = parse_return(&trim_commas(&tokens[..unless_idx]))?;
        return crate::effect_sentences::subject_verb_primitives::try_build_unless(
            vec![return_effect],
            SubjectVerbPrimitiveClause::new(tokens),
            unless_idx,
        )?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported return-unless clause (clause: '{clause_text}')"
            ))
        });
    }
    let mut exiled_with_source_surface =
        crate::effect_sentences::verb_handlers::parse_exiled_with_source_return_tail_surface(
            tokens,
        )
        .or_else(|| {
            crate::effect_sentences::verb_handlers::parse_exiled_with_source_move_surface(tokens)
        });
    if let Some(surface) = &mut exiled_with_source_surface {
        surface.verb = ironsmith_core::ExiledWithSourceMoveVerbSurface::Return;
    }
    let shape = crate::grammar::effects::parse_return_clause_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing return destination (clause: '{clause_text}')"
        ))
    })?;
    debug_assert!(!shape.has_unless);

    let destination_first = shape.destination_first;
    let destination = shape.destination;
    if destination.has_unparsed_timing_words {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{clause_text}')"
        )));
    }
    let delayed_timing = destination.timing.map(|timing| match timing {
        crate::grammar::effects::ReturnTimingShape::NextEndStep(player) => {
            DelayedReturnTimingAst::NextEndStep(player)
        }
        crate::grammar::effects::ReturnTimingShape::NextUpkeep(player) => {
            DelayedReturnTimingAst::NextUpkeep(player)
        }
        crate::grammar::effects::ReturnTimingShape::EndOfCombat => {
            DelayedReturnTimingAst::EndOfCombat
        }
    });
    let under_that_player_control =
        destination.controller == crate::grammar::effects::ReturnControllerShape::ThatPlayer;
    let return_controller = match destination.controller {
        crate::grammar::effects::ReturnControllerShape::Preserve => ReturnControllerAst::Preserve,
        crate::grammar::effects::ReturnControllerShape::You => ReturnControllerAst::You,
        crate::grammar::effects::ReturnControllerShape::Owner => ReturnControllerAst::Owner,
        crate::grammar::effects::ReturnControllerShape::ThatPlayer => {
            // The exact player is carried by the actor of the generic
            // PutOntoBattlefield action below, so no new controller model is
            // needed here.
            ReturnControllerAst::Preserve
        }
    };
    let attached_to_target = destination
        .attached_to_tokens
        .as_deref()
        .map(|tokens| {
            let mut target = parse_return_back_reference_target(tokens)?;
            if crate::grammar::filters::reference_tag_stage::has_plural_object_head_surface(tokens)
                && let Some(filter) =
                    crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(
                        &mut target,
                    )
            {
                filter.set_plural_object_noun_surface(true);
            }
            Ok::<_, CardTextError>(target)
        })
        .transpose()?;

    let effect = match shape.target {
        crate::grammar::effects::ReturnTargetShape::PairedSourceAndExiled { source_subtype } => {
            let mut source_filter = ObjectFilter::source();
            if let Some(subtype) = source_subtype {
                source_filter.subtypes.push(subtype);
            }
            let exiled_filter =
                ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);
            let mut filter = ObjectFilter::default();
            filter.any_of = vec![source_filter, exiled_filter];
            EffectAst::subject_verb_return_all_to_hand(filter)
        }
        crate::grammar::effects::ReturnTargetShape::UntargetedExiledCards {
            filter_tokens,
            count,
        } => {
            let has_explicit_source_link =
                exiled_with_source_surface.as_ref().is_some_and(|surface| {
                    !matches!(
                        surface.source,
                        ironsmith_core::ExiledWithSourceReferenceSurface::Omitted
                    )
                });
            let (filter_tokens, excludes_source) = strip_except_this_card_suffix(&filter_tokens);
            // The `exiled with this <source>` relative clause identifies the
            // source-linked set; it is not a characteristic restriction on
            // the returned cards. Parse only the authored noun phrase before
            // `exiled`, then represent the relationship with SOURCE_EXILED_TAG.
            // This prevents a source type such as "Vehicle" or "Saga" from
            // leaking into the selected-card filter.
            let source_linked_subject = exiled_with_source_surface
                .as_ref()
                .filter(|_| has_explicit_source_link)
                .and_then(|_| {
                    crate::slice_primitives::select_position(filter_tokens, |token| {
                        token.is_word("exiled")
                    })
                })
                .map(|exiled_idx| &filter_tokens[..exiled_idx])
                .unwrap_or(filter_tokens);
            let source_linked_excludes_current = has_explicit_source_link
                && source_linked_subject
                    .iter()
                    .any(|token| token.is_word("other"));
            let omitted_exiled_set = exiled_with_source_surface.as_ref().is_some_and(|surface| {
                matches!(
                    surface.subject,
                    ironsmith_core::ExiledWithSourceSubjectSurface::TheExiledCard
                        | ironsmith_core::ExiledWithSourceSubjectSurface::TheExiledCards
                )
            });
            let mut filter = if omitted_exiled_set {
                ObjectFilter::default()
            } else {
                parse_object_filter(source_linked_subject, false)?
            };
            if has_explicit_source_link {
                // The dedicated move surface owns the authored `card(s)` noun.
                // Keeping the same presentation bit on the executable filter
                // would make an otherwise identical source-linked set compare
                // differently in structural renderers.
                filter.set_explicit_card_noun(false);
                filter.zone = Some(Zone::Exile);
                // In "each other card exiled with this source", `other`
                // excludes the object produced by the immediately preceding
                // exile result. It is not the ordinary source-relative
                // `ObjectFilter::other` predicate.
                filter.other = false;
            }
            // "The exiled cards" can appear in a later ability of the same
            // source. Do not let its generic `it` placeholder bind to an
            // unrelated local action (for example, a sacrifice immediately
            // before the return). Exile execution already records the
            // source/object link represented by SOURCE_EXILED_TAG.
            filter
                .tagged_constraints
                .retain(|constraint| constraint.relation != TaggedOpbjectRelation::IsTaggedObject);
            filter = filter.match_tagged(
                TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                TaggedOpbjectRelation::IsTaggedObject,
            );
            if source_linked_excludes_current {
                filter = filter.not_tagged(TagKey::from(crate::cards::builders::IT_TAG));
            }
            filter.other |= excludes_source;
            match destination.zone {
                crate::grammar::effects::ReturnZoneShape::Battlefield => {
                    if let Some(attached_to) = attached_to_target.clone() {
                        if destination.face_down {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported face-down attached return-all clause (clause: '{clause_text}')"
                            )));
                        }
                        // A return-all instruction with an attachment
                        // destination must keep the returned collection and
                        // the preexisting attachment target as two distinct
                        // references. The generic move-all lowering already
                        // tags every returned object before emitting one
                        // AttachObjectsEffect, so use that typed path instead
                        // of dropping the attachment from ReturnAll.
                        let target = TargetAst::Object(filter, None, None);
                        let effect = if let Some(count) = count {
                            EffectAst::subject_verb_move_to_zone(
                                TargetAst::WithCount(Box::new(target), count),
                                Zone::Battlefield,
                                false,
                                return_controller,
                                destination.tapped,
                                Some(attached_to),
                            )
                        } else {
                            EffectAst::subject_verb_move_all_to_zone(
                                target,
                                Zone::Battlefield,
                                false,
                                return_controller,
                                destination.tapped,
                                Some(attached_to),
                            )
                        };
                        effect.with_move_to_zone_verb_surface(
                            ironsmith_core::MoveToZoneVerbSurface::Return,
                        )
                    } else if count.is_some() || has_explicit_source_link {
                        if destination.face_down {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported counted/source-linked face-down return clause (clause: '{clause_text}')"
                            )));
                        }
                        let target = TargetAst::Object(filter, None, None);
                        let effect = if let Some(count) = count {
                            EffectAst::subject_verb_move_to_zone(
                                TargetAst::WithCount(Box::new(target), count),
                                Zone::Battlefield,
                                false,
                                return_controller,
                                destination.tapped,
                                None,
                            )
                        } else {
                            EffectAst::subject_verb_move_all_to_zone(
                                target,
                                Zone::Battlefield,
                                false,
                                return_controller,
                                destination.tapped,
                                None,
                            )
                        };
                        effect.with_move_to_zone_verb_surface(
                            ironsmith_core::MoveToZoneVerbSurface::Return,
                        )
                    } else {
                        EffectAst::subject_verb_return_all_to_battlefield(
                            filter,
                            destination.tapped,
                            destination.face_down,
                            return_controller,
                        )
                    }
                }
                crate::grammar::effects::ReturnZoneShape::Graveyard => {
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Object(filter, None, None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )
                    .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
                }
                crate::grammar::effects::ReturnZoneShape::Hand => {
                    if let Some(count) = count {
                        EffectAst::subject_verb_return_to_hand(
                            TargetAst::WithCount(
                                Box::new(TargetAst::Object(filter, None, None)),
                                count,
                            ),
                            shape.random,
                        )
                    } else {
                        EffectAst::subject_verb_return_all_to_hand(filter)
                    }
                }
            }
        }
        crate::grammar::effects::ReturnTargetShape::MultiTargetUnsupported => {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{clause_text}')"
            )));
        }
        crate::grammar::effects::ReturnTargetShape::All {
            set_quantifier_surface,
            raw_filter_tokens,
            filter_tokens,
            chosen_this_way_excluded,
            chosen_creature_type,
            excluded_chosen_creature_type,
            chosen_type_this_way_surface,
            discarded_or_cycled_this_turn_by,
            unsupported_qualifier,
        } => {
            if unsupported_qualifier {
                return Err(CardTextError::ParseError(format!(
                    "unsupported qualified return-all filter (clause: '{clause_text}')"
                )));
            }
            if raw_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "missing return-all filter".to_string(),
                ));
            }
            if destination.zone == crate::grammar::effects::ReturnZoneShape::Hand
                && let Some((choice_idx, consumed)) =
                    find_color_choice_phrase(SubjectVerbPrimitiveClause::new(&raw_filter_tokens))
            {
                let base_filter_tokens = trim_commas(&raw_filter_tokens[..choice_idx]);
                let trailing = trim_commas(&raw_filter_tokens[choice_idx + consumed..]);
                if !trailing.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing color-choice return-all clause (clause: '{clause_text}')"
                    )));
                }
                if base_filter_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing return-all filter before color-choice clause (clause: '{clause_text}')"
                    )));
                }
                let mut filter = parse_object_filter(&base_filter_tokens, false)?;
                filter.set_return_destination_first_surface(destination_first);
                for subtype in &destination.excluded_subtypes {
                    if filter
                        .excluded_subtypes
                        .iter()
                        .all(|existing| existing != subtype)
                    {
                        filter.excluded_subtypes.push(*subtype);
                    }
                }
                return Ok(wrap_return_with_delayed_timing(
                    EffectAst::subject_verb_return_all_to_hand_of_chosen_color(filter),
                    delayed_timing,
                ));
            }
            let mut filter = parse_object_filter(&filter_tokens, false)?;
            filter.set_set_quantifier_surface(Some(set_quantifier_surface));
            filter.set_return_destination_first_surface(destination_first);
            filter.chosen_creature_type |= chosen_creature_type;
            filter.excluded_chosen_creature_type |= excluded_chosen_creature_type;
            if chosen_type_this_way_surface {
                filter.set_chosen_type_this_way_surface(true);
            }
            filter.discarded_or_cycled_this_turn_by = discarded_or_cycled_this_turn_by;
            for subtype in &destination.excluded_subtypes {
                if filter
                    .excluded_subtypes
                    .iter()
                    .all(|existing| existing != subtype)
                {
                    filter.excluded_subtypes.push(*subtype);
                }
            }
            if let Some(excluded) = chosen_this_way_excluded {
                filter = if excluded {
                    filter.not_tagged(TagKey::from(IT_TAG))
                } else {
                    filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject)
                };
            }
            match destination.zone {
                crate::grammar::effects::ReturnZoneShape::Battlefield => {
                    EffectAst::subject_verb_return_all_to_battlefield(
                        filter,
                        destination.tapped,
                        destination.face_down,
                        return_controller,
                    )
                }
                crate::grammar::effects::ReturnZoneShape::Graveyard => {
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Object(filter, None, None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )
                    .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
                }
                crate::grammar::effects::ReturnZoneShape::Hand => {
                    EffectAst::subject_verb_return_all_to_hand(filter)
                }
            }
        }
        crate::grammar::effects::ReturnTargetShape::Singular {
            target_tokens,
            source_from_graveyard_tokens,
            source_from_graveyard_or_exile_tokens,
            dynamic_count,
            back_reference,
            top_only,
        } => {
            if !destination.excluded_subtypes.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return exception on non-return-all clause (clause: '{clause_text}')"
                )));
            }
            let source_from_graveyard_target =
                source_from_graveyard_tokens
                    .as_deref()
                    .and_then(|prefix_tokens| match parse_target_phrase(prefix_tokens) {
                        Ok(TargetAst::Source(span)) => Some(TargetAst::Source(span)),
                        _ => None,
                    });
            let source_from_graveyard_or_exile_target = source_from_graveyard_or_exile_tokens
                .as_deref()
                .and_then(|prefix_tokens| match parse_target_phrase(prefix_tokens) {
                    Ok(TargetAst::Source(span)) => Some(TargetAst::Source(span)),
                    _ => None,
                });
            let graveyard_or_exile_source = source_from_graveyard_or_exile_target.is_some();
            let mut target = if let Some(target) = source_from_graveyard_or_exile_target {
                target
            } else if let Some(target) = source_from_graveyard_target {
                target
            } else if back_reference {
                parse_return_back_reference_target(&target_tokens)?
            } else {
                parse_target_phrase(&target_tokens)?
            };
            let words = crate::lexer::token_word_refs(tokens);
            if destination.zone == crate::grammar::effects::ReturnZoneShape::Battlefield
                && crate::word_primitives::sequence_occurs(&words, &["from", "your", "graveyard"])
                && let Some(filter) =
                    crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(
                        &mut target,
                    )
            {
                // A historical attachment predicate defaults to the
                // battlefield for ordinary "attached to" queries. In a
                // return instruction, an explicit later origin phrase owns
                // the selected-card zone and owner instead.
                filter.zone = Some(Zone::Graveyard);
                filter.owner = Some(PlayerFilter::You);
            }
            let count_value = dynamic_count.then_some(crate::effect::Value::EventValue(
                crate::effect::EventValueSpec::Amount,
            ));
            if dynamic_count {
                target =
                    TargetAst::WithCount(Box::new(target), crate::effect::ChoiceCount::dynamic_x());
            }
            set_return_destination_first_surface(&mut target, destination_first);
            match destination.zone {
                crate::grammar::effects::ReturnZoneShape::Battlefield => {
                    if destination.face_down && (destination.transformed || destination.converted) {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported face-down transformed/converted return clause (clause: '{clause_text}')"
                        )));
                    }
                    if under_that_player_control {
                        if destination.attacking
                            || destination.face_down
                            || destination.transformed
                            || destination.converted
                            || attached_to_target.is_some()
                        {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported modified return under that player's control (clause: '{clause_text}')"
                            )));
                        }
                        EffectAst::subject_verb_put_onto_battlefield(
                            PlayerAst::That,
                            target,
                            destination.tapped,
                            ReturnControllerAst::Preserve,
                        )
                    } else if let Some(attached_to) = attached_to_target {
                        if destination.transformed || destination.converted || count_value.is_some()
                        {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported transformed/converted/dynamic return attached clause (clause: '{clause_text}')"
                            )));
                        }
                        EffectAst::subject_verb_move_to_zone_with_attacking(
                            target,
                            Zone::Battlefield,
                            false,
                            return_controller,
                            destination.tapped,
                            false,
                            destination.face_down,
                            Some(attached_to),
                        )
                        .with_move_to_zone_verb_surface(
                            ironsmith_core::MoveToZoneVerbSurface::Return,
                        )
                    } else if destination.attacking || destination.face_down {
                        EffectAst::subject_verb_move_to_zone_with_attacking(
                            target,
                            Zone::Battlefield,
                            false,
                            return_controller,
                            destination.tapped,
                            destination.attacking,
                            destination.face_down,
                            None,
                        )
                        .with_move_to_zone_verb_surface(
                            ironsmith_core::MoveToZoneVerbSurface::Return,
                        )
                    } else {
                        let effect = EffectAst::subject_verb_return_to_battlefield(
                            target,
                            destination.tapped,
                            destination.transformed,
                            destination.converted,
                            return_controller,
                            count_value,
                        )
                        .with_top_only_return_choice(top_only);
                        if graveyard_or_exile_source {
                            effect.with_graveyard_or_exile_return_origin()
                        } else {
                            effect
                        }
                    }
                }
                crate::grammar::effects::ReturnZoneShape::Graveyard => {
                    EffectAst::subject_verb_move_to_zone(
                        target,
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )
                    .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
                }
                crate::grammar::effects::ReturnZoneShape::Hand => {
                    EffectAst::subject_verb_return_to_hand(target, shape.random)
                }
            }
        }
    };
    let retained_exiled_with_source_surface = exiled_with_source_surface.filter(|surface| {
        !matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Omitted
        )
    });
    let mut effect = effect.with_exiled_with_source_surface(retained_exiled_with_source_surface);
    effect = if destination.zone == crate::grammar::effects::ReturnZoneShape::Hand {
        effect.with_return_destination_player_surface(destination.destination_player_surface)
    } else {
        effect
    };
    if destination.zone == crate::grammar::effects::ReturnZoneShape::Battlefield
        && destination.destination_player_surface == Some(PlayerAst::That)
        && let EffectAst::SubjectVerb(subject_verb) = &mut effect
        && let SubjectVerbActionAst::ReturnToBattlefield { target, .. } = &mut subject_verb.action
        && let Some(filter) =
            crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(target)
    {
        // This surface fact distinguishes an authored "under their control"
        // destination from the ordinary rules-default owner controller. The
        // owner filter remains the executable identity of "their".
        filter.set_enters_under_controller_surface(true);
    }
    Ok(wrap_return_with_delayed_timing(effect, delayed_timing))
}
