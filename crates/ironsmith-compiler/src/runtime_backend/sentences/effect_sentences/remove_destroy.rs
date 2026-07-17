use super::*;
use crate::runtime_backend::front_end::grammar::effects::remove_destroy_shapes as shapes;
use crate::runtime_backend::util::{
    parse_filter_counter_constraint_words, strip_leading_token_words_any,
};

pub(crate) fn parse_remove(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let shape = shapes::parse_remove_clause_shape(tokens).map_err(|error| match error {
        shapes::RemoveShapeError::MissingAmount => CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )),
        shapes::RemoveShapeError::MissingCounterKeyword => {
            CardTextError::ParseError("missing counter keyword".to_string())
        }
    })?;

    match shape {
        shapes::RemoveClauseShape::AllOfThem => {
            Ok(EffectAst::subject_verb_remove_all_of_them_counters_from_source())
        }
        shapes::RemoveClauseShape::FromCombat { target_tokens } => {
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing remove-from-combat target (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
            Ok(EffectAst::subject_verb_remove_from_combat(
                parse_target_phrase(target_tokens)?,
            ))
        }
        shapes::RemoveClauseShape::AllCounters {
            counter_descriptor,
            target_tokens,
            source_like_target,
            leave_one,
        } => {
            let counter_type = parse_counter_type_from_descriptor_tokens(counter_descriptor);
            let target_words = crate::runtime_backend::token_word_refs(target_tokens);
            if !leave_one && target_words.first().copied() == Some("all") {
                let filter_tokens = strip_leading_token_words_any(target_tokens, &["all"]);
                let filter = parse_object_filter(filter_tokens, false)?;
                return Ok(EffectAst::subject_verb_remove_counters_all(
                    Value::CountersOn(Box::new(ChooseSpec::Iterated), counter_type),
                    filter,
                    counter_type,
                    false,
                ));
            }
            let target = if source_like_target {
                TargetAst::Source(span_from_tokens(target_tokens))
            } else {
                parse_target_phrase(target_tokens)?
            };
            let amount = match (&target, counter_type) {
                (TargetAst::Source(_), Some(counter_type)) => Value::CountersOnSource(counter_type),
                (TargetAst::Source(_), None) => {
                    Value::CountersOn(Box::new(ChooseSpec::Source), None)
                }
                _ => Value::CountersOn(Box::new(ChooseSpec::Source), counter_type),
            };
            let amount = if leave_one {
                Value::Add(Box::new(amount), Box::new(Value::Fixed(-1)))
            } else {
                amount
            };
            Ok(EffectAst::subject_verb_remove_up_to_any_counters(
                amount,
                target,
                counter_type,
                false,
            ))
        }
        shapes::RemoveClauseShape::Counters {
            amount,
            up_to,
            counter_descriptor,
            destination,
        } => {
            let counter_type = parse_counter_type_from_descriptor_tokens(counter_descriptor);
            match destination {
                shapes::RemoveCounterDestination::All { filter_tokens } => {
                    let filter = parse_object_filter(filter_tokens, false)?;
                    Ok(EffectAst::subject_verb_remove_counters_all(
                        amount,
                        filter,
                        counter_type,
                        up_to,
                    ))
                }
                shapes::RemoveCounterDestination::ForEach {
                    target_tokens,
                    count_filter_tokens,
                    fallback_target_tokens,
                } => {
                    if let (Ok(target), Ok(count_filter)) = (
                        parse_target_phrase(target_tokens),
                        parse_object_filter(count_filter_tokens, false),
                    ) {
                        return Ok(EffectAst::ForEachObject {
                            filter: count_filter,
                            effects: vec![EffectAst::subject_verb_remove_up_to_any_counters(
                                amount,
                                target,
                                counter_type,
                                up_to,
                            )],
                        });
                    }
                    let target = parse_target_phrase(fallback_target_tokens)?;
                    Ok(EffectAst::subject_verb_remove_up_to_any_counters(
                        amount,
                        target,
                        counter_type,
                        up_to,
                    ))
                }
                shapes::RemoveCounterDestination::Single { target_tokens } => {
                    let target = parse_target_phrase(target_tokens)?;
                    Ok(EffectAst::subject_verb_remove_up_to_any_counters(
                        amount,
                        target,
                        counter_type,
                        up_to,
                    ))
                }
            }
        }
    }
}

fn wrap_destroy_with_delayed_timing(
    effect: EffectAst,
    timing: Option<shapes::DelayedDestroyTimingShape>,
) -> EffectAst {
    match timing {
        None => effect,
        Some(shapes::DelayedDestroyTimingShape::EndOfCombat) => {
            EffectAst::DelayedUntilEndOfCombat {
                effects: vec![effect],
            }
        }
        Some(shapes::DelayedDestroyTimingShape::NextEndStep) => {
            EffectAst::DelayedUntilNextEndStep {
                player: PlayerFilter::Any,
                effects: vec![effect],
            }
        }
    }
}

fn parse_destroy_all_filter(tokens: &[OwnedLexToken]) -> Result<ObjectFilter, CardTextError> {
    if let Some(shape) = shapes::parse_destroy_counter_constraint_shape(tokens) {
        let tail_words = crate::runtime_backend::token_word_refs(shape.constraint_tokens);
        if let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&tail_words)
            && consumed == tail_words.len()
        {
            let mut filter = parse_object_filter(shape.base_tokens, false)?;
            match shape.kind {
                shapes::DestroyCounterConstraintKind::With => {
                    filter.with_counter = Some(counter_constraint);
                }
                shapes::DestroyCounterConstraintKind::Without => {
                    filter.without_counter = Some(counter_constraint);
                }
            }
            return Ok(filter);
        }
    }
    parse_object_filter(tokens, false)
}

fn lower_combat_history_target(
    shape: shapes::DestroyCombatHistoryShape<'_>,
) -> Result<Option<TargetAst>, CardTextError> {
    match shape {
        shapes::DestroyCombatHistoryShape::DealtDamageThisTurn { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            let TargetAst::Object(mut filter, target_span, it_span) = target else {
                return Ok(None);
            };
            filter.was_dealt_damage_this_turn = true;
            Ok(Some(TargetAst::Object(filter, target_span, it_span)))
        }
        shapes::DestroyCombatHistoryShape::DealtDamageToPlayerThisTurn {
            target_tokens,
            player_tokens,
        } => {
            let TargetAst::Player(player, _) = parse_target_phrase(player_tokens)? else {
                return Ok(None);
            };
            let target = parse_target_phrase(target_tokens)?;
            let TargetAst::Object(mut filter, target_span, it_span) = target else {
                return Ok(None);
            };
            filter.dealt_damage_to_player_this_turn = Some(player);
            Ok(Some(TargetAst::Object(filter, target_span, it_span)))
        }
    }
}

fn lower_destroy_all_shape(shape: shapes::DestroyAllShape<'_>) -> Result<EffectAst, CardTextError> {
    match shape {
        shapes::DestroyAllShape::DealtDamageThisTurn { filter_tokens } => {
            let mut filter = parse_destroy_all_filter(filter_tokens)?;
            filter.was_dealt_damage_this_turn = true;
            Ok(EffectAst::subject_verb_destroy_all(filter))
        }
        shapes::DestroyAllShape::DealtDamageToPlayerThisTurn {
            filter_tokens,
            player_tokens,
        } => {
            let TargetAst::Player(player, _) = parse_target_phrase(player_tokens)? else {
                return Err(CardTextError::ParseError(
                    "combat-history destroy-all recipient must be a player".to_string(),
                ));
            };
            let mut filter = parse_destroy_all_filter(filter_tokens)?;
            filter.dealt_damage_to_player_this_turn = Some(player);
            Ok(EffectAst::subject_verb_destroy_all(filter))
        }
        shapes::DestroyAllShape::AttachedTo {
            filter_tokens,
            target_tokens,
        } => Ok(EffectAst::subject_verb_destroy_all_attached_to(
            parse_object_filter(filter_tokens, false)?,
            parse_target_phrase(target_tokens)?,
        )),
        shapes::DestroyAllShape::ExceptFor {
            filter_tokens,
            exception_tokens,
        } => {
            let mut filter = parse_object_filter(filter_tokens, false)?;
            let exception_filter = parse_object_filter(exception_tokens, false)?;
            apply_except_filter_exclusions(&mut filter, &exception_filter);
            Ok(EffectAst::subject_verb_destroy_all(filter))
        }
        shapes::DestroyAllShape::ChosenColor { filter_tokens } => {
            let filter = parse_object_filter(filter_tokens, false)?;
            Ok(EffectAst::subject_verb_destroy_all_of_chosen_color(
                filter, false,
            ))
        }
        shapes::DestroyAllShape::ChosenThisWay {
            filter_tokens,
            relation,
        } => {
            let relation = match relation {
                shapes::TaggedDestroyRelation::Matching => TaggedOpbjectRelation::IsTaggedObject,
                shapes::TaggedDestroyRelation::ExceptMatching => {
                    TaggedOpbjectRelation::IsNotTaggedObject
                }
            };
            let filter = parse_object_filter(filter_tokens, false)?
                .match_tagged(TagKey::from(IT_TAG), relation);
            Ok(EffectAst::subject_verb_destroy_all(filter))
        }
        shapes::DestroyAllShape::Plain { filter_tokens } => Ok(
            EffectAst::subject_verb_destroy_all(parse_destroy_all_filter(filter_tokens)?),
        ),
    }
}

pub(crate) fn parse_destroy(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let original_clause = crate::runtime_backend::token_word_refs(tokens).join(" ");
    let shape = shapes::parse_destroy_clause_shape(tokens);
    let timing = shape.timing;
    let effect = match shape.kind {
        shapes::DestroyClauseKind::Empty => {
            return Err(CardTextError::ParseError(format!(
                "missing destroy target before delayed timing clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::UnsupportedDelayedTiming => {
            return Err(CardTextError::ParseError(format!(
                "unsupported delayed destroy timing clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::CombatHistory(combat_shape) => {
            let Some(target) = lower_combat_history_target(combat_shape)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported combat-history destroy clause (clause: '{original_clause}')"
                )));
            };
            EffectAst::subject_verb_destroy(target)
        }
        shapes::DestroyClauseKind::UnsupportedCombatHistory => {
            return Err(CardTextError::ParseError(format!(
                "unsupported combat-history destroy clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::All(all_shape) => lower_destroy_all_shape(all_shape)?,
        shapes::DestroyClauseKind::UnlessTargetSetPredicate {
            target_tokens,
            predicate,
        } => {
            let predicate = match predicate {
                crate::runtime_backend::grammar::conditions::TargetSetPredicateAst::DifferentColorSets => {
                    PredicateAst::TargetObjectsHaveDifferentColorSets
                }
            };
            EffectAst::Conditional {
                predicate: PredicateAst::Not(Box::new(predicate)),
                if_true: vec![EffectAst::subject_verb_destroy(parse_target_phrase(
                    target_tokens,
                )?)],
                if_false: Vec::new(),
            }
        }
        shapes::DestroyClauseKind::UnlessPays {
            target_tokens,
            payment,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            let player = match crate::runtime_backend::util::parse_subject(payment.player_tokens) {
                SubjectAst::Player(player) => player,
                _ => PlayerAst::Implicit,
            };
            let cost = match payment.kind {
                crate::runtime_backend::grammar::effects::UnlessPaymentKind::LifeEqualToItsToughness => {
                    let value = Value::ToughnessOf(Box::new(
                        crate::runtime_backend::reference_helpers::choose_spec_for_target(&target),
                    ));
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::life(value))
                }
                crate::runtime_backend::grammar::effects::UnlessPaymentKind::Cost => {
                    crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(
                        payment.payment_tokens,
                    )?
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported destroy-unless payment (clause: '{original_clause}')"
                        ))
                    })?
                }
            };
            EffectAst::UnlessPays {
                effects: vec![EffectAst::subject_verb_destroy(target)],
                player,
                cost,
            }
        }
        shapes::DestroyClauseKind::UnsupportedUnless => {
            return Err(CardTextError::ParseError(format!(
                "unsupported destroy-unless clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::TrailingAttackOrBlockRestriction => {
            return Err(CardTextError::ParseError(format!(
                "compound destroy plus attack/block restriction should be parsed as an effect chain (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::Conditional {
            target_tokens,
            predicate_tokens,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            let predicate_tail = parse_conditional_predicate_tail_lexed(predicate_tokens)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported conditional destroy clause (clause: '{original_clause}')"
                    ))
                })?;
            match predicate_tail {
                ConditionalPredicateTailSpec::InsteadIf {
                    base_predicate,
                    outer_predicate,
                } => EffectAst::Conditional {
                    predicate: outer_predicate,
                    if_true: vec![EffectAst::Conditional {
                        predicate: base_predicate,
                        if_true: vec![EffectAst::subject_verb_destroy(target)],
                        if_false: Vec::new(),
                    }],
                    if_false: Vec::new(),
                },
                ConditionalPredicateTailSpec::Plain(predicate) => EffectAst::Conditional {
                    predicate,
                    if_true: vec![EffectAst::subject_verb_destroy(target)],
                    if_false: Vec::new(),
                },
            }
        }
        shapes::DestroyClauseKind::UnsupportedConditional => {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional destroy clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::MultiTarget => {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::Blocked { target_tokens } => EffectAst::Conditional {
            predicate: PredicateAst::TargetIsBlocked,
            if_true: vec![EffectAst::subject_verb_destroy(parse_target_phrase(
                &target_tokens,
            )?)],
            if_false: Vec::new(),
        },
        shapes::DestroyClauseKind::Plain { target_tokens } => {
            EffectAst::subject_verb_destroy(parse_target_phrase(target_tokens)?)
        }
    };
    Ok(wrap_destroy_with_delayed_timing(effect, timing))
}

pub(crate) fn apply_except_filter_exclusions(base: &mut ObjectFilter, exception: &ObjectFilter) {
    for card_type in exception
        .card_types
        .iter()
        .copied()
        .chain(exception.all_card_types.iter().copied())
    {
        if !base.excluded_card_types.contains(&card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in exception.subtypes.iter().copied() {
        if !base.excluded_subtypes.contains(&subtype) {
            base.excluded_subtypes.push(subtype);
        }
    }
}
