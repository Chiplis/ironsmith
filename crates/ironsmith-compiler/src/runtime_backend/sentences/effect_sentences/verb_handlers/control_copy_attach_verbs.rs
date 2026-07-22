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
    let tokens = crate::runtime_backend::util::trim_edge_punctuation_tokens(tokens);
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
        if life_shape.remap_its_source_stat {
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
    let tokens = crate::runtime_backend::util::trim_edge_punctuation_tokens(tokens);
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let life_shape = cca_shapes::parse_life_surface_shape(tokens);

    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if life_shape.remap_its_source_stat {
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
            EffectAst::TrailingUnless {
                predicate,
                effects: vec![base_effect],
            }
        } else {
            EffectAst::TrailingIf {
                predicate,
                effects: vec![base_effect],
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

pub(crate) fn parse_exiled_with_source_move_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::ExiledWithSourceMoveSurface> {
    use ironsmith_core::{
        ExiledWithSourceDestinationSurface as DestinationSurface, ExiledWithSourceMoveSurface,
        ExiledWithSourceMoveVerbSurface as MoveVerbSurface,
        ExiledWithSourceReferenceSurface as ReferenceSurface,
        ExiledWithSourceSubjectSurface as SubjectSurface,
    };

    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let destination_marker = words
        .iter()
        .position(|word| matches!(*word, "into" | "onto" | "to"))?;
    let target_words = &words[..destination_marker];
    let verb = if target_words.first() == Some(&"return") {
        MoveVerbSurface::Return
    } else {
        MoveVerbSurface::Put
    };
    let target_words = target_words
        .strip_prefix(&["put"])
        .or_else(|| target_words.strip_prefix(&["return"]))
        .unwrap_or(target_words);
    let (subject, source) = if target_words == ["the", "exiled", "card"] {
        (SubjectSurface::TheExiledCard, ReferenceSurface::Omitted)
    } else if target_words == ["the", "exiled", "cards"] {
        (SubjectSurface::TheExiledCards, ReferenceSurface::Omitted)
    } else {
        let exiled = target_words.iter().position(|word| *word == "exiled")?;
        let subject_words = &target_words[..exiled];
        let subject = if subject_words == ["all", "cards"] {
            SubjectSurface::AllCards
        } else if subject_words == ["each", "card"] {
            SubjectSurface::EachCard
        } else if subject_words == ["one", "card"] || subject_words == ["a", "card"] {
            SubjectSurface::OneCard
        } else if subject_words == ["the", "exiled", "card"] {
            SubjectSurface::TheExiledCard
        } else if subject_words == ["the", "exiled", "cards"] {
            SubjectSurface::TheExiledCards
        } else if subject_words == ["the", "cards"] {
            SubjectSurface::TheCards
        } else {
            let exiled_token = tokens.iter().position(|token| token.is_word("exiled"))?;
            let rendered = crate::runtime_backend::front_end::lexer::render_token_slice(
                &tokens[..exiled_token],
            );
            let rendered = rendered.trim();
            let rendered = rendered
                .strip_prefix("Put ")
                .or_else(|| rendered.strip_prefix("put "))
                .or_else(|| rendered.strip_prefix("Return "))
                .or_else(|| rendered.strip_prefix("return "))
                .unwrap_or(rendered)
                .trim();
            if rendered.is_empty()
                || !subject_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
            {
                return None;
            }
            SubjectSurface::Custom(rendered.to_string())
        };

        let with_offset = target_words[exiled..]
            .iter()
            .position(|word| *word == "with")?;
        let source_words = &target_words[exiled + with_offset + 1..];
        let source = if source_words == ["it"] {
            ReferenceSurface::It
        } else {
            let surface = crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(source_words)
                .or_else(|| crate::runtime_backend::front_end::shared::util::this_source_surface_for_words(source_words))?;
            ReferenceSurface::Source(surface)
        };
        (subject, source)
    };

    let destination_words = &words[destination_marker + 1..];
    let destination_has = |phrase: &[&str]| {
        destination_words
            .windows(phrase.len())
            .any(|window| window == phrase)
    };
    let destination = if destination_has(&["its", "owner"])
        || destination_has(&["its", "owner's"])
        || destination_has(&["its", "owners"])
        || destination_has(&["its", "owners'"])
    {
        DestinationSurface::ItsOwner
    } else if destination_has(&["their", "owners"]) || destination_has(&["their", "owners'"]) {
        DestinationSurface::TheirOwners
    } else if destination_has(&["their", "owner"]) || destination_has(&["their", "owner's"]) {
        DestinationSurface::TheirOwner
    } else {
        DestinationSurface::ContextualPlayer
    };

    Some(ExiledWithSourceMoveSurface {
        verb,
        subject,
        source,
        destination,
    })
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

        // Parse the characteristic prefix independently from the zone
        // disjunction.  Otherwise the generic filter parser can put the
        // Aura/Equipment (or other type) union inside `any_of`, and clearing
        // that union while expanding the two zones silently drops it.
        if let Some(from_index) = target_tokens.iter().position(|token| {
            token
                .as_word()
                .is_some_and(|word| word.eq_ignore_ascii_case("from"))
        }) && from_index > 0
            && let Ok(base) = parse_target_phrase(&target_tokens[..from_index])
        {
            target = base;
        }

        let target_words = crate::runtime_backend::token_word_refs(target_tokens);
        let owner = target_words.windows(2).any(|window| {
            window[0].eq_ignore_ascii_case("your")
                && (window[1].eq_ignore_ascii_case("hand")
                    || window[1].eq_ignore_ascii_case("graveyard"))
        });

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

    fn apply_explicit_source_location(
        target: &mut TargetAst,
        tokens: &[OwnedLexToken],
    ) {
        let words = crate::runtime_backend::token_word_refs(tokens);
        let location = if words
            .windows(3)
            .any(|window| window == ["from", "your", "hand"])
        {
            Some((Zone::Hand, Some(PlayerFilter::You)))
        } else if words
            .windows(3)
            .any(|window| window == ["from", "your", "graveyard"])
        {
            Some((Zone::Graveyard, Some(PlayerFilter::You)))
        } else if words
            .windows(3)
            .any(|window| window == ["from", "your", "library"])
        {
            Some((Zone::Library, Some(PlayerFilter::You)))
        } else {
            None
        };
        let Some((zone, owner)) = location else {
            return;
        };

        apply_source_zone_constraint(target, zone);
        if let Some(owner) = owner
            && let Some(filter) = crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::target_object_filter_mut(target)
        {
            filter.owner = Some(owner);
        }
    }

    fn strip_source_top_only_prefix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
        use winnow::Parser as _;

        crate::runtime_backend::grammar::primitives::parse_prefix(
            tokens,
            crate::runtime_backend::grammar::primitives::phrase(&["the", "top"]).void(),
        )
        .map(|(_, rest)| (rest, true))
        .unwrap_or((tokens, false))
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let exiled_with_source_surface = parse_exiled_with_source_move_surface(tokens);

    if let Some(shape) = cca_shapes::parse_revealed_remainder_shape(tokens) {
        let order = if shape.random_order {
            crate::cards::builders::LibraryBottomOrderAst::Random
        } else {
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
        };
        return Ok(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                TagKey::from("__last_revealed__"),
                shape
                    .exclude_current_reference
                    .then(|| TagKey::from(IT_TAG)),
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

    if let Some(shape) = cca_shapes::parse_tagged_battlefield_partition_shape(tokens) {
        let collection_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
            tokens,
            "partition_pool",
        );
        let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
            tokens,
            "partition_chosen",
        );
        let owner = crate::runtime_backend::families::activation_and_restrictions::controller_filter_for_token_player(player)
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
            EffectAst::subject_verb_return_all_to_hand(filter)
                .with_exiled_with_source_surface(exiled_with_source_surface.clone()),
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // "Put one of those cards on top of your library and the rest on the bottom of your library"
    if let Some(shape) = cca_shapes::parse_tagged_on_top_library_shape(tokens) {
        let library_owner = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
        let looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
            tokens, "looked",
        );
        let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
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
        .with_move_to_zone_plural_surface_if(cca_shapes::is_plural_tagged_object_reference(tokens));
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
        if shape.placement == cca_shapes::LibraryPlacementShape::Bottom
            && cca_shapes::is_rest_reference(shape.target_tokens)
        {
            return Ok(EffectAst::subject_verb_put_rest_on_bottom_of_library());
        }
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
            let target = parse_target_phrase(target_tokens)?;
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
            )
            .with_exiled_with_source_surface(exiled_with_source_surface.clone());
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
            crate::runtime_backend::front_end::grammar::choices::parse_possessive_object_choice_tokens(
                target_tokens,
            )
        {
            use crate::runtime_backend::front_end::grammar::choices::PossessiveObjectChoiceActor;

            let chooser = match choice_shape.actor {
                PossessiveObjectChoiceActor::You => Some(PlayerAst::You),
                PossessiveObjectChoiceActor::SubjectPlayer => {
                    extract_subject_player(subject.clone())
                }
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
                if let Some(choice_owner) = crate::runtime_backend::families::activation_and_restrictions::controller_filter_for_token_player(
                    chooser.clone(),
                ) {
                    if filter.owner == Some(PlayerFilter::IteratedPlayer) {
                        filter.owner = Some(choice_owner.clone());
                    }
                    if filter.controller == Some(PlayerFilter::IteratedPlayer) {
                        filter.controller = Some(choice_owner);
                    }
                }
                let tag =
                    crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
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
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        target = expand_graveyard_or_hand_disjunction(target, target_tokens);
        apply_explicit_source_location(&mut target, target_tokens);
        if !cca_shapes::target_names_unowned_shared_zone(target_tokens)
            && let Some(filter) = crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
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

#[cfg(test)]
mod looked_card_count_tests {
    use super::*;

    #[test]
    fn source_exiled_move_surface_preserves_typed_subjects_and_onto_marker() {
        let tokens = crate::runtime_backend::front_end::lexer::lex_line(
            "Put target creature card with mana value X exiled with this creature onto the battlefield under your control.",
            0,
        )
        .expect("lex source-exiled move");
        let surface = parse_exiled_with_source_move_surface(&tokens)
            .expect("parse source-exiled move surface");

        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::Custom(
                "target creature card with mana value X".to_string()
            )
        );
        assert!(matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(
                crate::target::SourceReferenceSurface::ThisPermanentType(ref text)
            ) if text == "this creature"
        ));

        let effect = parse_put_into_hand(&tokens, None).expect("parse source-exiled move");
        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    exiled_with_source_surface: Some(
                        ironsmith_core::ExiledWithSourceMoveSurface {
                            subject: ironsmith_core::ExiledWithSourceSubjectSurface::Custom(ref text),
                            ..
                        }
                    ),
                    zone: Zone::Battlefield,
                    ..
                },
                ..
            }) if text == "target creature card with mana value X"
        ));

        let tokens = crate::runtime_backend::front_end::lexer::lex_line(
            "Return all cards you own exiled with this artifact to your hand.",
            0,
        )
        .expect("lex source-exiled return");
        let surface = parse_exiled_with_source_move_surface(&tokens)
            .expect("parse source-exiled return surface");
        assert_eq!(
            surface.verb,
            ironsmith_core::ExiledWithSourceMoveVerbSurface::Return
        );
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::Custom("all cards you own".to_string())
        );
    }

    #[test]
    fn standalone_tagged_hand_move_preserves_exact_choice_count() {
        let tokens = crate::runtime_backend::front_end::lexer::lex_line(
            "Put one of those cards into your hand.",
            0,
        )
        .expect("lex looked-card move");
        let effect = parse_put_into_hand(&tokens, None).expect("parse looked-card move");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    target: TargetAst::WithCount(inner, count),
                    zone,
                    destination_player_surface,
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected a counted tagged move, got {effect:#?}");
        };

        assert_eq!(count, ChoiceCount::exactly(1));
        assert!(matches!(
            inner.as_ref(),
            TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG
        ));
        assert_eq!(zone, Zone::Hand);
        assert_eq!(destination_player_surface, Some(PlayerAst::You));
    }
}
