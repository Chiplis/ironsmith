use super::*;

pub fn parse_deal_damage_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_damage_equal_shape_lexed(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::lexer::token_word_refs(tokens);
    let authored_difference = crate::word_primitives::sequence_occurs(
        &crate::lexer::token_word_refs(shape.amount_tokens),
        &["difference", "between"],
    )
    .then(|| parse_add_mana_equal_amount_value(shape.amount_tokens))
    .flatten();
    // Count expressions with a relative controller tail need the typed count
    // parser before the permissive value-expression fallback. Otherwise
    // `nonbasic lands that creature's controller controls` is accepted as a
    // single Land+Creature filter and loses the same-target controller link.
    let aggregate_amount = parse_equal_to_aggregate_filter_value(shape.amount_tokens);
    let relative_count = parse_equal_to_number_of_filter_value(shape.amount_tokens);
    let complete_value = aggregate_amount
        .or(authored_difference)
        .or(relative_count)
        .or_else(|| {
            parse_value(shape.amount_tokens)
                .and_then(|(value, used)| (used == shape.amount_tokens.len()).then_some(value))
                .map(preserve_equal_to_surface)
        });
    let amount = complete_value
        .or(parse_add_mana_equal_amount_value(shape.amount_tokens))
        .or(parse_devotion_value_from_add_clause(shape.amount_tokens)?)
        .or(parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
            shape.amount_tokens,
        ))
        .or(parse_equal_to_number_of_opponents_you_have_value(
            shape.amount_tokens,
        ))
        .or(parse_equal_to_number_of_counters_on_reference_value(
            shape.amount_tokens,
        ))
        .or(parse_dynamic_cost_modifier_value(shape.amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let amount = preserve_equal_to_surface(amount);
    if let Some(effect) = damage_to_embedded_target_controller(amount.clone(), shape.target_tokens)
    {
        return Ok(Some(effect));
    }
    if let Some(target) = parse_combat_player_damage_target(shape.target_tokens, true) {
        return Ok(Some(combat_player_damage_target_effect(
            amount.clone(),
            target,
        )));
    }
    if shape.target_is_each_or_all {
        if shape.target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_damage_each_filter(&shape.target_tokens[1..])?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = preserve_optional_single_damage_target(
        parse_target_phrase(shape.target_tokens)?,
        shape.target_tokens,
    );
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}

pub(super) fn parse_divided_damage_target(
    target_tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let clause = crate::lexer::token_word_refs(target_tokens).join(" ");
    let shape = combat_grammar::parse_combat_divided_target_shape_lexed(target_tokens).map_err(
        |error| {
            let message = match error {
                combat_grammar::CombatDividedTargetError::MissingTargetsAfterAmong => {
                    format!("missing divided-damage targets after 'among' (clause: '{clause}')")
                }
                combat_grammar::CombatDividedTargetError::MissingTargetPhrase => {
                    format!("missing divided-damage target phrase (clause: '{clause}')")
                }
                combat_grammar::CombatDividedTargetError::UnsupportedTargetCount => {
                    format!("unsupported divided-damage target count (clause: '{clause}')")
                }
                combat_grammar::CombatDividedTargetError::MissingTargetCount => {
                    format!("missing divided-damage target count (clause: '{clause}')")
                }
            };
            CardTextError::ParseError(message)
        },
    )?;
    let base_target = if shape.any_target {
        TargetAst::AnyTarget(span_from_tokens(shape.target_tokens))
    } else if shape
        .target_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("them"))
    {
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(shape.target_tokens),
        )
    } else if shape
        .target_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("those"))
    {
        let mut filter = parse_object_filter(&shape.target_tokens[1..], false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        TargetAst::Object(filter, None, span_from_tokens(shape.target_tokens))
    } else {
        parse_target_phrase(shape.target_tokens)?
    };
    Ok(TargetAst::WithCount(Box::new(base_target), shape.count))
}

pub(super) fn parse_divided_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let shape =
        combat_grammar::parse_combat_divided_amount_shape_lexed(tokens, used).map_err(|_| {
            CardTextError::ParseError(format!(
                "missing damage keyword in divided-damage clause (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            ))
        })?;
    match shape {
        combat_grammar::CombatDividedAmountShape::EvenlyEach { filter_tokens } => {
            let filter = parse_damage_each_filter(filter_tokens)?;
            Ok(EffectAst::subject_verb_damage_each(amount, filter))
        }
        combat_grammar::CombatDividedAmountShape::Distributed {
            target_tokens,
            evenly_rounded_down,
        } => {
            let target = parse_divided_damage_target(target_tokens)?;
            if evenly_rounded_down {
                Ok(EffectAst::subject_verb_evenly_distributed_damage(
                    amount, target,
                ))
            } else {
                Ok(EffectAst::subject_verb_distributed_damage(amount, target))
            }
        }
    }
}

pub fn parse_deal_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let clause = crate::lexer::token_word_refs(tokens).join(" ");
    let shape =
        combat_grammar::parse_combat_damage_target_shape_lexed(tokens, used).map_err(|error| {
            let message = match error {
                combat_grammar::CombatDamageTargetShapeError::MissingDamageKeyword => {
                    "missing damage keyword".to_string()
                }
                combat_grammar::CombatDamageTargetShapeError::UnsupportedTrailingIfClause
                | combat_grammar::CombatDamageTargetShapeError::UnsupportedEmbeddedIfClause => {
                    format!("unsupported trailing if clause in damage effect (clause: '{clause}')")
                }
                combat_grammar::CombatDamageTargetShapeError::MissingEachFilter => {
                    "missing damage target filter after 'each'".to_string()
                }
            };
            CardTextError::ParseError(message)
        })?;

    match shape {
        combat_grammar::CombatDamageTargetShape::InsteadIf {
            target_tokens,
            predicate_tokens,
            instead_tail_tokens,
        } => {
            let predicate = if let Some(predicate) =
                parse_instead_if_control_predicate(predicate_tokens)?
            {
                predicate
            } else {
                parse_trailing_instead_if_predicate_lexed(instead_tail_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported trailing instead-if clause in damage effect (clause: '{clause}')"
                    ))
                })?
            };
            let target = if target_tokens.is_empty() {
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
            } else {
                parse_target_phrase(target_tokens)?
            };
            Ok(EffectAst::TrailingIf {
                predicate,
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            })
        }
        combat_grammar::CombatDamageTargetShape::TrailingIf {
            target_tokens,
            predicate,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![EffectAst::subject_verb_damage(amount, target)],
                if_false: Vec::new(),
            })
        }
        combat_grammar::CombatDamageTargetShape::TrailingUnless {
            target_tokens,
            predicate,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::TrailingUnless {
                predicate,
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            })
        }
        combat_grammar::CombatDamageTargetShape::OmittedTargetIf { predicate } => {
            Ok(EffectAst::TrailingIf {
                predicate,
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
                )],
            })
        }
        combat_grammar::CombatDamageTargetShape::Simple {
            shape,
            target_tokens,
        } => {
            let amount = if shape == combat_grammar::CombatSimpleDamageTargetShape::IteratedPlayer
                && crate::word_primitives::parse_sequence_complete(
                    &crate::lexer::token_word_refs(target_tokens),
                    &["them"],
                ) {
                amount.with_surface_hint(ironsmith_core::ValueSurfaceHint::DamageRecipientPronoun)
            } else {
                amount
            };
            Ok(EffectAst::subject_verb_damage(
                amount,
                combat_simple_damage_target_ast(shape, target_tokens),
            ))
        }
        combat_grammar::CombatDamageTargetShape::EachOfCount { count, span_tokens } => {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(span_tokens))),
                count,
            );
            Ok(EffectAst::subject_verb_damage(amount, target))
        }
        combat_grammar::CombatDamageTargetShape::EachOfTarget { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::subject_verb_damage(amount, target))
        }
        combat_grammar::CombatDamageTargetShape::PlayerGroup(target) => {
            Ok(combat_player_damage_target_effect(amount, target))
        }
        combat_grammar::CombatDamageTargetShape::MaxSpeedPlayers { has_max_speed } => {
            let filter = if has_max_speed {
                PlayerFilter::with_max_speed(PlayerFilter::Any)
            } else {
                PlayerFilter::without_max_speed(PlayerFilter::Any)
            };
            Ok(EffectAst::ForEachPlayersFiltered {
                filter,
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            })
        }
        combat_grammar::CombatDamageTargetShape::OpponentWho { predicate_tokens } => {
            let predicate = parse_who_did_this_way_predicate(predicate_tokens)?;
            Ok(EffectAst::ForEachOpponentDid {
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
                predicate,
                result_predicate: IfResultPredicate::Did,
            })
        }
        combat_grammar::CombatDamageTargetShape::PlayerWho { predicate_tokens } => {
            let predicate = parse_who_did_this_way_predicate(predicate_tokens)?;
            Ok(EffectAst::ForEachPlayerDid {
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
                predicate,
                result_predicate: IfResultPredicate::Did,
            })
        }
        combat_grammar::CombatDamageTargetShape::PlayerAndObjects {
            player_filter,
            player_span,
            filter_tokens,
        } => {
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(player_filter.clone());
            }
            Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(player_filter, player_span),
                    ),
                    EffectAst::subject_verb_damage_each(amount, filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::EachObjectsAndPlayer { filter_tokens } => {
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::IteratedPlayer);
            }
            Ok(EffectAst::ForEachPlayer {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                    ),
                    EffectAst::subject_verb_damage_each(amount, filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::OpponentAndControlledCreaturePlaneswalker => {
            let mut filter = ObjectFilter::default();
            filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
            filter.controller = Some(PlayerFilter::IteratedPlayer);
            Ok(EffectAst::ForEachOpponent {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                    ),
                    EffectAst::subject_verb_damage_each(amount, filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::HistoricalDamageRecipients {
            players,
            filter_tokens,
        } => {
            let player_filter = PlayerFilter::was_dealt_damage_by_source_this_game(
                combat_player_damage_target_filter(players),
            );
            let mut object_filter = parse_object_filter(filter_tokens, false)?;
            object_filter.was_dealt_damage_by_source_this_game = true;
            Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::ForEachPlayersFiltered {
                        filter: player_filter,
                        effects: vec![EffectAst::subject_verb_damage(
                            amount.clone(),
                            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                        )],
                    },
                    EffectAst::subject_verb_damage_each(amount, object_filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::EachFilter { filter_tokens } => {
            let filter = parse_damage_each_filter(filter_tokens)?;
            Ok(EffectAst::subject_verb_damage_each(amount, filter))
        }
        combat_grammar::CombatDamageTargetShape::DelayedEndOfCombat { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::DelayedUntilEndOfCombat {
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            })
        }
        combat_grammar::CombatDamageTargetShape::General { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::subject_verb_damage(amount, target))
        }
    }
}
