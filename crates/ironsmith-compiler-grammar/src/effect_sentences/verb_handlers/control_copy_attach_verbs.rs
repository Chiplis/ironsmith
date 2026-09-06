use crate::cards::builders::DelayedEffectAst;
use crate::cards::builders::ControlActionAst;
use crate::cards::builders::TokenActionAst;
use crate::grammar::effects::control_copy_attach_shapes as cca_shapes;

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
        SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
            tag: crate::tag::TagRef::of(looked_tag),
            keep_tagged: crate::tag::TagRef::of(chosen_tag),
            zone: rest_zone,
            surface: ironsmith_core::LibraryRemainderSurface::Rest,
        }),
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
            into: crate::tag::TagRef::of(looked_tag.clone()),
        },
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: Zone::Library,
        }),
        EffectAst::MoveTaggedGroupToZone {
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: chosen_zone,
        },
    ]
}

pub fn parse_lose_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::lexer::token_word_refs(tokens);
    let life_shape = cca_shapes::parse_life_surface_shape(tokens);

    if let Some(cca_shapes::ExactLifeSurface::Fixed(amount)) = life_shape.exact {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                amount: Value::Fixed(amount as i32),
            }),
        ));
    }
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if life_shape.remap_its_source_stat {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount }),
        ));
    }
    if life_shape.exact == Some(cca_shapes::ExactLifeSurface::LoseGame) {
        return Ok(EffectAst::subject_verb_lose_game(player));
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount }),
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                amount: amount.clone(),
            }),
        );
        if let Some(delayed) =
            wrap_parsed_effect_in_delayed_next_step_unless_pays(&trailing, base_effect)?
        {
            return Ok(delayed);
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount }),
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount }),
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            }));
        }
        if let Some(unless_tail) = cca_shapes::parse_life_surface_shape(&trailing).unless_tail {
            let mut unless_as_if_tokens = Vec::with_capacity(unless_tail.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(unless_tail);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                }));
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
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount }),
    ))
}

pub fn parse_gain_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let life_shape = cca_shapes::parse_life_surface_shape(tokens);

    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if life_shape.remap_its_source_stat {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
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
                crate::lexer::token_word_refs(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            }));
        }
        if let Some(unless_tail) = cca_shapes::parse_life_surface_shape(&trailing).unless_tail {
            let mut unless_as_if_tokens = Vec::with_capacity(unless_tail.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(unless_tail);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                }));
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
    ))
}

pub fn parse_gain_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let explicit_triggering_source_controller =
        matches!(subject, Some(SubjectAst::TriggeringSourceController));
    let clause_words = crate::lexer::token_word_refs(tokens);
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
    let opponent_choice =
        crate::grammar::choices::parse_possessive_object_choice_tokens(
            shape.target_tokens,
        )
        .filter(|choice| {
            choice.actor
                == crate::grammar::choices::PossessiveObjectChoiceActor::Opponent
        });
    let target_tokens = opponent_choice
        .as_ref()
        .map_or(shape.target_tokens, |choice| {
            choice.object_tokens.as_slice()
        });
    let (target_ast, trailing_predicate, is_unless) =
        if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                false,
            )
        } else if crate::lexer::contains_token_word(target_tokens, "if") {
            return Err(invalid_conditional_error());
        } else if let Some(spec) = split_trailing_unless_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                true,
            )
        } else if crate::lexer::contains_token_word(target_tokens, "unless") {
            return Err(invalid_conditional_error());
        } else {
            (parse_target_phrase(target_tokens)?, None, false)
        };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let mut base_effect = match target_ast {
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
    if explicit_triggering_source_controller
        && let EffectAst::SubjectVerb(subject_verb) = &mut base_effect
        && let SubjectVerbActionAst::Control(ControlActionAst::GainControl {
            controller_reference,
            ..
        }) = &mut subject_verb.action
    {
        *controller_reference = Some(crate::target::ObjectRef::tagged(crate::tag::CompilerReferenceTag::TriggeringSource.bind()));
    }

    if opponent_choice.is_some()
        && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Control(ControlActionAst::GainControl { target, .. }),
            ..
        }) = &mut base_effect
    {
        let target_tag = crate::util::helper_tag_for_tokens(
            target_tokens,
            "opponent_chosen_target",
        );
        let declared = std::mem::replace(
            target,
            TargetAst::Tagged(crate::tag::TagRef::of(target_tag.clone()), span_from_tokens(target_tokens)),
        );
        base_effect = EffectAst::Sequence {
            effects: vec![
                EffectAst::TagAffected {
                    effect: Box::new(EffectAst::subject_verb_explicit_target_only_for_chooser(
                        declared,
                        PlayerAst::Opponent,
                    )),
                    tag: crate::tag::TagRef::of(target_tag),
                },
                base_effect,
            ],
        };
    }

    let effect = if let Some(predicate) = trailing_predicate {
        if is_unless {
            EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless {
                predicate,
                effects: vec![base_effect],
            })
        } else {
            EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                predicate,
                effects: vec![base_effect],
            })
        }
    } else {
        base_effect
    };

    if shape.delayed_until_end_of_combat {
        return Ok(EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        }));
    }

    Ok(effect)
}

pub fn parse_control_duration(
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
        Option<PredicateAst>,
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

pub fn parse_exiled_with_source_move_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::ExiledWithSourceMoveSurface> {
    parse_exiled_with_source_move_surface_inner(tokens, None)
}

/// Parse the noun-and-destination tail passed to `parse_return` after the
/// subject/verb dispatcher has already consumed the authored `return` verb.
/// Keep this separate from the full-clause entry point so arbitrary verbless
/// fragments cannot be mistaken for source-linked zone moves.
pub fn parse_exiled_with_source_return_tail_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::ExiledWithSourceMoveSurface> {
    parse_exiled_with_source_move_surface_inner(
        tokens,
        Some(ironsmith_core::ExiledWithSourceMoveVerbSurface::Return),
    )
}

fn parse_exiled_with_source_move_surface_inner(
    tokens: &[OwnedLexToken],
    assumed_verb: Option<ironsmith_core::ExiledWithSourceMoveVerbSurface>,
) -> Option<ironsmith_core::ExiledWithSourceMoveSurface> {
    use ironsmith_core::{
        ExiledWithSourceDestinationSurface as DestinationSurface, ExiledWithSourceMoveSurface,
        ExiledWithSourceMoveVerbSurface as MoveVerbSurface,
        ExiledWithSourceReferenceSurface as ReferenceSurface,
        ExiledWithSourceSubjectSurface as SubjectSurface,
    };

    let words = crate::lexer::parser_token_word_refs(tokens);
    let destination_marker = crate::slice_primitives::select_position(&words, |word| {
        matches!(*word, "into" | "onto" | "to" | "on")
    })?;
    let clause_target_words = &words[..destination_marker];
    // This helper is also called before the ordinary subject/verb split, so
    // retain the move surface when an explicit actor precedes the verb (`they
    // put`, `that player puts`, `each player returns`). The noun phrase begins
    // immediately after the first authored move verb.
    let (target_words, verb) = if let Some(verb) = assumed_verb {
        (clause_target_words, verb)
    } else {
        let verb_idx = crate::slice_primitives::select_position(clause_target_words, |word| {
            matches!(*word, "put" | "puts" | "return" | "returns")
        })?;
        let verb = match clause_target_words[verb_idx] {
            "put" | "puts" => MoveVerbSurface::Put,
            "return" | "returns" => MoveVerbSurface::Return,
            _ => return None,
        };
        (&clause_target_words[verb_idx + 1..], verb)
    };
    let (subject, source) = if crate::word_primitives::parse_sequence_complete(
        target_words,
        &["the", "exiled", "card"],
    ) {
        (SubjectSurface::TheExiledCard, ReferenceSurface::Omitted)
    } else if crate::word_primitives::parse_sequence_complete(
        target_words,
        &["the", "exiled", "cards"],
    ) {
        (SubjectSurface::TheExiledCards, ReferenceSurface::Omitted)
    } else {
        let exiled = crate::slice_primitives::select_position(target_words, |word| {
            *word == "exiled"
        })?;
        let subject_words = &target_words[..exiled];
        let subject = if crate::word_primitives::parse_sequence_complete(
            subject_words,
            &["all", "cards"],
        ) {
            SubjectSurface::AllCards
        } else if crate::word_primitives::parse_sequence_complete(
            subject_words,
            &["each", "card"],
        ) {
            SubjectSurface::EachCard
        } else if crate::word_primitives::parse_any_sequence_complete(
            subject_words,
            &[&["one", "card"], &["a", "card"]],
        ) {
            SubjectSurface::OneCard
        } else if crate::word_primitives::parse_sequence_complete(
            subject_words,
            &["the", "exiled", "card"],
        ) {
            SubjectSurface::TheExiledCard
        } else if crate::word_primitives::parse_sequence_complete(
            subject_words,
            &["the", "exiled", "cards"],
        ) {
            SubjectSurface::TheExiledCards
        } else if crate::word_primitives::parse_sequence_complete(
            subject_words,
            &["the", "cards"],
        ) {
            SubjectSurface::TheCards
        } else {
            let subject_token_start = if assumed_verb.is_some() {
                0
            } else {
                crate::slice_primitives::select_position(tokens, |token| {
                    token.is_word("put")
                        || token.is_word("puts")
                        || token.is_word("return")
                        || token.is_word("returns")
                })? + 1
            };
            let exiled_token = crate::slice_primitives::select_position(
                &tokens[subject_token_start..],
                |token| token.is_word("exiled"),
            )? + subject_token_start;
            let rendered = crate::lexer::render_token_slice(
                &tokens[subject_token_start..exiled_token],
            );
            let rendered = rendered.trim();
            if rendered.is_empty()
                || !subject_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
            {
                return None;
            }
            SubjectSurface::Custom(rendered.to_string())
        };

        let with_offset = crate::slice_primitives::select_position(
            &target_words[exiled..],
            |word| *word == "with",
        )?;
        let source_words = &target_words[exiled + with_offset + 1..];
        let source_words = crate::slice_primitives::strip_suffix(
            source_words,
            &["except", "this", "card"],
        )
            .unwrap_or(source_words);
        let source = if crate::word_primitives::parse_sequence_complete(source_words, &["it"]) {
            ReferenceSurface::It
        } else {
            let surface = crate::util::source_reference_surface_for_words(source_words)
                .or_else(|| crate::util::this_source_surface_for_words(source_words))?;
            ReferenceSurface::Source(surface)
        };
        (subject, source)
    };

    let destination_words = &words[destination_marker + 1..];
    let destination_has = |phrase: &[&str]| {
        crate::word_primitives::sequence_occurs(destination_words, phrase)
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

fn preserve_exiled_with_source_subject_cardinality(
    target: TargetAst,
    surface: Option<&ironsmith_core::ExiledWithSourceMoveSurface>,
) -> TargetAst {
    if surface.is_some_and(|surface| {
        surface.subject == ironsmith_core::ExiledWithSourceSubjectSurface::OneCard
    }) && !matches!(
        target,
        TargetAst::WithCount(..) | TargetAst::WithCountValue(..)
    ) {
        TargetAst::WithCount(Box::new(target), crate::effect::ChoiceCount::exactly(1))
    } else {
        target
    }
}

/// "Put it onto the battlefield or into your hand" — the mover picks one
/// destination, so lower one move mode per zone instead of failing the parse.
fn parse_put_destination_choice(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<Option<EffectAst>, CardTextError> {
    let or_split = crate::slice_primitives::find_window_by(tokens, 2, |pair| {
        pair[0].is_word("or")
            && pair[1]
                .as_word()
                .is_some_and(|next| matches!(next, "into" | "onto" | "both" | "on"))
    });
    let Some(or_idx) = or_split else {
        return Ok(None);
    };
    let left_tokens = &tokens[..or_idx];
    let right_tokens = &tokens[or_idx + 1..];
    let right_words = crate::lexer::token_word_refs(right_tokens);
    let right_words: Vec<&str> = right_words
        .iter()
        .copied()
        .filter(|word| {
            !matches!(
                *word,
                "your" | "the" | "their" | "its" | "owner's" | "both" | "of"
            )
        })
        .collect();
    let Some((right_zone, right_to_top)) = crate::word_primitives::matching_value(
        &right_words,
        &[
            (&["into", "hand"], (Zone::Hand, false)),
            (&["into", "graveyard"], (Zone::Graveyard, false)),
            (&["onto", "battlefield"], (Zone::Battlefield, false)),
            (&["into", "exile"], (Zone::Exile, false)),
            (&["on", "bottom", "library"], (Zone::Library, false)),
        ],
    ) else {
        return Ok(None);
    };
    let left_effect = parse_put_into_hand(left_tokens, subject)?;
    let EffectAst::SubjectVerb(left_subject_verb) = &left_effect else {
        return Ok(None);
    };
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { .. }) = &left_subject_verb.action else {
        return Ok(None);
    };
    let mut right_effect = left_effect.clone();
    if let EffectAst::SubjectVerb(subject_verb) = &mut right_effect
        && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { zone, to_top, .. }) = &mut subject_verb.action
    {
        *zone = right_zone;
        *to_top = right_to_top;
    }
    let left_display = format!(
        "Put {}",
        crate::lexer::render_token_slice(
            left_tokens.get(1..).unwrap_or(left_tokens),
        )
    );
    let right_display = format!(
        "Put it {}",
        crate::lexer::render_token_slice(right_tokens),
    );
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenChoice {
            options: vec![
                (left_display, Box::new(left_effect)),
                (right_display, Box::new(right_effect)),
            ],
        }),
    )))
}

#[cfg(test)]
#[path = "control_copy_attach_verbs_inline_looked_card_count_tests.rs"]
mod looked_card_count_tests;

#[path = "control_copy_attach_verbs/control_copy_attach_verbs_zone.rs"]
mod control_copy_attach_verbs_zone_programs;
pub use control_copy_attach_verbs_zone_programs::{parse_put_into_hand};
