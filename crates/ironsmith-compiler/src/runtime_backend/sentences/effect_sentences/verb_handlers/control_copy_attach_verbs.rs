use crate::runtime_backend::front_end::grammar::effects::control_copy_attach_shapes as cca_shapes;

fn cca_controller(shape: cca_shapes::BattlefieldControllerShape) -> ReturnControllerAst {
    match shape {
        cca_shapes::BattlefieldControllerShape::You => ReturnControllerAst::You,
        cca_shapes::BattlefieldControllerShape::Owner => ReturnControllerAst::Owner,
    }
}

fn parse_counted_card_target_prefix(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let Some(shape) = cca_shapes::parse_counted_card_target_shape(target_tokens) else {
        return Ok(None);
    };
    let inner = parse_target_phrase(shape.target_tokens)?;
    Ok(Some(TargetAst::WithCount(Box::new(inner), shape.count)))
}

fn compose_put_filtered_looked_cards_into_hand_rest_into_graveyard(
    player: PlayerAst,
    filter: ObjectFilter,
    count: ChoiceCount,
    looked_tag: TagKey,
    chosen_tag: TagKey,
) -> Vec<EffectAst> {
    compose_put_filtered_looked_cards_to_zone_rest_to_zone(
        player,
        filter,
        count,
        looked_tag,
        chosen_tag,
        Zone::Hand,
        Zone::Graveyard,
    )
}

fn compose_put_filtered_looked_cards_to_zone_rest_to_zone(
    player: PlayerAst,
    filter: ObjectFilter,
    count: ChoiceCount,
    looked_tag: TagKey,
    chosen_tag: TagKey,
    chosen_zone: Zone,
    rest_zone: Zone,
) -> Vec<EffectAst> {
    let mut effects = compose_put_filtered_looked_cards_to_zone(
        player,
        filter,
        count,
        looked_tag.clone(),
        chosen_tag.clone(),
        chosen_zone,
    );
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::PutTaggedRemainderInZone {
            tag: looked_tag,
            keep_tagged: chosen_tag,
            zone: rest_zone,
        },
    ));
    effects
}

fn compose_put_filtered_looked_cards_to_zone(
    player: PlayerAst,
    mut filter: ObjectFilter,
    count: ChoiceCount,
    looked_tag: TagKey,
    chosen_tag: TagKey,
    chosen_zone: Zone,
) -> Vec<EffectAst> {
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    vec![
        EffectAst::SnapshotLastObjectTag {
            into: looked_tag.clone(),
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::MoveTaggedGroupToZone {
            tag: chosen_tag.clone(),
            zone: chosen_zone,
        },
    ]
}

pub(crate) fn parse_lose_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let life_shape = cca_shapes::parse_life_surface_shape(tokens);

    if let Some(cca_shapes::ExactLifeSurface::Fixed(amount)) = life_shape.exact {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(amount as i32),
            },
        ));
    }
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && life_shape.remap_its_source_stat
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }
    if life_shape.exact == Some(cca_shapes::ExactLifeSurface::LoseGame) {
        return Ok(EffectAst::subject_verb_lose_game(player));
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::LoseLife { amount },
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            });
        }
        if let Some(unless_tail) = cca_shapes::parse_life_surface_shape(&trailing).unless_tail {
            let mut unless_as_if_tokens = Vec::with_capacity(unless_tail.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(unless_tail);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                });
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LoseLife { amount },
    ))
}

pub(crate) fn parse_gain_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let life_shape = cca_shapes::parse_life_surface_shape(tokens);

    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && life_shape.remap_its_source_stat
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainLife { amount },
        ));
    }

    // "gains no life [instead]" — a prevention rider ("If <player> would gain
    // life this turn, that player gains no life instead", Flames of the Blood
    // Hand). Model as a can't-gain-life window for the damaged player.
    if life_shape.exact == Some(cca_shapes::ExactLifeSurface::NoLifePrevention) {
        let restricted = match player {
            PlayerAst::You => crate::target::PlayerFilter::You,
            _ => crate::target::PlayerFilter::DamagedPlayer,
        };
        return Ok(EffectAst::subject_verb_cant(
            crate::effect::Restriction::gain_life(restricted),
            Until::EndOfTurn,
            None,
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life gain")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if life_shape.unsupported_shuffle_graveyard {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::GainLife { amount },
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainLife { amount },
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            });
        }
        if let Some(unless_tail) = cca_shapes::parse_life_surface_shape(&trailing).unless_tail {
            let mut unless_as_if_tokens = Vec::with_capacity(unless_tail.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(unless_tail);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                });
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::GainLife { amount },
    ))
}

pub(crate) fn parse_gain_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let shape = cca_shapes::parse_gain_control_clause_shape(tokens)
        .ok_or_else(|| CardTextError::ParseError("missing control keyword".to_string()))?;
    if shape.dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let invalid_conditional_error = || {
        CardTextError::ParseError(format!(
            "unsupported conditional gain-control clause (clause: '{}')",
            clause_words.join(" ")
        ))
    };
    let (target_ast, trailing_predicate, is_unless) = if let Some(spec) =
        split_trailing_if_clause_lexed(shape.target_tokens)
    {
        (
            parse_target_phrase(spec.leading_tokens)?,
            Some(spec.predicate),
            false,
        )
    } else if crate::runtime_backend::lexer::contains_token_word(shape.target_tokens, "if") {
        return Err(invalid_conditional_error());
    } else if let Some(spec) = split_trailing_unless_clause_lexed(shape.target_tokens) {
        (
            parse_target_phrase(spec.leading_tokens)?,
            Some(spec.predicate),
            true,
        )
    } else if crate::runtime_backend::lexer::contains_token_word(shape.target_tokens, "unless") {
        return Err(invalid_conditional_error());
    } else {
        (parse_target_phrase(shape.target_tokens)?, None, false)
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let base_effect = match target_ast {
        TargetAst::Player(filter, _) => {
            let duration = parse_control_duration(shape.duration_tokens)?;
            if matches!(duration, ControlDurationAst::UntilYourNextTurnEnd) {
                return Err(CardTextError::ParseError(
                    "unsupported player-control duration until the end of your next turn"
                        .to_string(),
                ));
            }
            EffectAst::subject_verb_control_player(
                player,
                PlayerFilter::Target(Box::new(filter)),
                duration,
            )
        }
        _ => {
            let (until, condition, source_reference_surface) =
                parse_permanent_gain_control_duration(shape.duration_tokens)?;
            EffectAst::subject_verb_gain_control_with_condition_and_source_surface(
                player,
                target_ast,
                until,
                condition,
                source_reference_surface,
            )
        }
    };

    let effect = if let Some(predicate) = trailing_predicate {
        if is_unless {
            EffectAst::Conditional {
                predicate,
                if_true: Vec::new(),
                if_false: vec![base_effect],
            }
        } else {
            EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            }
        }
    } else {
        base_effect
    };

    if shape.delayed_until_end_of_combat {
        return Ok(EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        });
    }

    Ok(effect)
}

pub(crate) fn parse_control_duration(
    tokens: &[OwnedLexToken],
) -> Result<ControlDurationAst, CardTextError> {
    cca_shapes::parse_control_duration_shape(tokens)
        .ok_or_else(|| CardTextError::ParseError("unsupported control duration".to_string()))
}

fn parse_permanent_gain_control_duration(
    tokens: &[OwnedLexToken],
) -> Result<
    (
        Until,
        Option<crate::ConditionExpr>,
        Option<crate::target::SourceReferenceSurface>,
    ),
    CardTextError,
> {
    let shape = cca_shapes::parse_permanent_control_duration_shape(tokens).ok_or_else(|| {
        let message = if cca_shapes::parse_control_duration_shape(tokens)
            == Some(ControlDurationAst::DuringNextTurn)
        {
            "unsupported control duration for permanents"
        } else {
            "unsupported control duration"
        };
        CardTextError::ParseError(message.to_string())
    })?;
    Ok((shape.until, shape.condition, shape.source_surface))
}

pub(crate) fn parse_put_into_hand(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_put_into_hand_delayed_timing(
        tokens: &[OwnedLexToken],
    ) -> Option<DelayedReturnTimingAst> {
        let tail_tokens = cca_shapes::parse_delayed_hand_tail(tokens)?;
        let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
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

        fn apply(filter: &ObjectFilter) -> ObjectFilter {
            let mut graveyard = filter.clone();
            graveyard.any_of.clear();
            graveyard.zone = Some(Zone::Graveyard);

            let mut hand = filter.clone();
            hand.any_of.clear();
            hand.zone = Some(Zone::Hand);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![graveyard, hand];
            disjunction
        }

        match &mut target {
            TargetAst::Object(filter, _, _) => {
                *filter = apply(filter);
            }
            TargetAst::WithCount(inner, _) => {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    *filter = apply(filter);
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

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    if let Some(shape) = cca_shapes::parse_revealed_remainder_shape(tokens) {
        let order = if shape.random_order {
            crate::cards::builders::LibraryBottomOrderAst::Random
        } else {
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
        };
        return Ok(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                TagKey::from("__last_revealed__"),
                Some(TagKey::from(IT_TAG)),
                order,
                cca_shapes::parse_destination_player(tokens).unwrap_or(player),
            ),
        );
    }

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if cca_shapes::is_reorder_tagged_cards(tokens) {
        return Ok(EffectAst::subject_verb_reorder_top_of_library(
            TagKey::from(IT_TAG),
        ));
    }

    let from_among_shape = cca_shapes::parse_from_among_them_shape(tokens);
    if let Some(shape) = from_among_shape
        && shape.destination == cca_shapes::FromAmongDestinationShape::Battlefield
    {
        let filter = crate::runtime_backend::effect_sentences::parse_looked_card_choice_filter(
            shape.filter_tokens,
        )
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to parse from-among hand filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
            tokens, "looked",
        );
        let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
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
        let looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
            tokens, "looked",
        );
        let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
            tokens, "chosen",
        );
        if let Some(shape) = from_among_shape {
            let filter = crate::runtime_backend::effect_sentences::parse_looked_card_choice_filter(
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
            EffectAst::subject_verb_return_all_to_hand(filter),
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // "Put one of those cards on top of your library and the rest on the bottom of your library"
    if let Some(choice_count) = cca_shapes::parse_tagged_on_top_library_shape(tokens) {
        let library_owner = cca_shapes::parse_destination_player(tokens).unwrap_or(player);

        return Ok(EffectAst::subject_verb_rearrange_looked_cards_in_library(
            library_owner,
            TagKey::from(IT_TAG),
            choice_count,
        ));
    }

    if let Some(put_shape) = cca_shapes::parse_tagged_into_hand_shape(tokens) {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if put_shape.rest_destination == Some(cca_shapes::RestDestinationShape::BottomOfLibrary)
            && let Some(choice_count) = put_shape.count
        {
            let dest_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
            let looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
                tokens, "looked",
            );
            let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
                tokens, "chosen",
            );

            return Ok(EffectAst::Sequence {
                effects: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                    dest_player,
                    choice_count,
                    looked_tag,
                    chosen_tag,
                ),
            });
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if put_shape.rest_destination == Some(cca_shapes::RestDestinationShape::Graveyard)
            && let Some(choice_count) = put_shape.count
        {
            // The chooser is typically the player whose hand is referenced.
            let dest_player = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
            let looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
                tokens, "looked",
            );
            let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
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

        let effect = EffectAst::subject_verb_put_into_hand(
            player,
            ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
        );
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
                    return Ok(EffectAst::subject_verb_return_all_to_battlefield(
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
        if shape.placement == cca_shapes::LibraryPlacementShape::Bottom
            && cca_shapes::is_rest_reference(shape.target_tokens)
        {
            return Ok(EffectAst::subject_verb_put_rest_on_bottom_of_library());
        }
        let target = if let Some(target) = parse_counted_card_target_prefix(shape.target_tokens)? {
            target
        } else {
            parse_target_phrase(shape.target_tokens)?
        };
        return Ok(EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Library,
            shape.placement == cca_shapes::LibraryPlacementShape::Top,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
    }

    if let Some(shape) = cca_shapes::parse_into_destination_shape(tokens) {
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
                ));
            }

            if zone == Zone::Hand {
                if let Some(count) = cca_shapes::parse_counted_those_cards(shape.target_tokens)
                    && cca_shapes::parse_rest_destination(shape.destination_tokens)
                        == Some(cca_shapes::RestDestinationShape::Graveyard)
                {
                    let dest_player =
                        cca_shapes::parse_destination_player(tokens).unwrap_or(player);
                    let looked_tag =
                        crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
                            tokens, "looked",
                        );
                    let chosen_tag =
                        crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
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
                    let effect = EffectAst::subject_verb_put_into_hand(
                        player,
                        ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    );
                    return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                }
            }

            let target = parse_target_phrase(shape.target_tokens)?;
            let effect = if cca_shapes::starts_with_all_or_each(shape.target_tokens) {
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
            };
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
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
            } else {
                parse_target_phrase(&target_tokens)?
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
            );
            let rest_target = parse_target_phrase(&rest_target_tokens)?;
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
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![effect],
                    if_false: Vec::new(),
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
            let effect = EffectAst::subject_verb_return_all_to_battlefield(
                filter,
                destination_shape.tapped,
                destination_shape.face_down,
                battlefield_controller,
            );
            return Ok(if let Some(predicate) = trailing_predicate {
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![effect],
                    if_false: Vec::new(),
                }
            } else {
                effect
            });
        }

        let mut target = if cca_shapes::is_tagged_object_reference(target_tokens) {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        if let Some(filter) = crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
        {
            crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::apply_exile_subject_owner_context(filter, subject);
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
        );
        return Ok(if let Some(predicate) = trailing_predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
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
