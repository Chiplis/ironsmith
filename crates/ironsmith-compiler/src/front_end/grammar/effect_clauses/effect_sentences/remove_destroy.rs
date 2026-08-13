use super::*;
use crate::effect::ChoiceCount;
use crate::grammar::effects::remove_destroy_shapes as shapes;
use crate::util::{
    helper_tag_for_tokens, parse_filter_counter_constraint_words, strip_leading_token_words_any,
};

pub(crate) fn parse_remove(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let shape = shapes::parse_remove_clause_shape(tokens).map_err(|error| match error {
        shapes::RemoveShapeError::MissingAmount => CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
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
                    crate::token_word_refs(tokens).join(" ")
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
            let target_words = crate::token_word_refs(target_tokens);
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
                shapes::RemoveCounterDestination::EachOfAnyNumber { filter_tokens } => {
                    let filter = parse_object_filter(filter_tokens, false)?;
                    let selected_tag = helper_tag_for_tokens(tokens, "counter_removal_subset");
                    Ok(EffectAst::Sequence {
                        effects: vec![
                            EffectAst::ChooseObjects {
                                filter,
                                count: ChoiceCount::any_number(),
                                count_value: None,
                                player: PlayerAst::You,
                                tag: selected_tag.clone(),
                            },
                            EffectAst::ForEachTagged {
                                tag: selected_tag,
                                effects: vec![EffectAst::subject_verb_remove_up_to_any_counters(
                                    amount,
                                    TargetAst::Tagged(
                                        TagKey::from(IT_TAG),
                                        span_from_tokens(tokens),
                                    ),
                                    counter_type,
                                    up_to,
                                )],
                            },
                        ],
                    })
                }
                shapes::RemoveCounterDestination::All { filter_tokens } => {
                    let filter = parse_object_filter(filter_tokens, false)?;
                    Ok(EffectAst::subject_verb_remove_counters_all(
                        amount,
                        filter,
                        counter_type,
                        up_to,
                    ))
                }
                shapes::RemoveCounterDestination::Among { filter_tokens } => {
                    let filter = parse_object_filter(filter_tokens, false)?;
                    Ok(EffectAst::subject_verb_remove_up_to_counters_among(
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
        let tail_words = crate::token_word_refs(shape.constraint_tokens);
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
    let original_clause = crate::token_word_refs(tokens).join(" ");
    if crate::token_word_refs(tokens).as_slice() == ["both", "creatures"] {
        return Ok(EffectAst::Coordinated {
            effects: vec![
                EffectAst::subject_verb_destroy(TargetAst::Source(None)),
                EffectAst::subject_verb_destroy(TargetAst::Tagged(TagKey::from(IT_TAG), None)),
            ],
            leading_duration: false,
            result_conjunction: false,
        });
    }
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
                crate::grammar::conditions::TargetSetPredicateAst::DifferentColorSets => {
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
            let player = match crate::util::parse_subject(payment.player_tokens) {
                SubjectAst::Player(player) => player,
                _ => PlayerAst::Implicit,
            };
            let cost = match payment.kind {
                crate::grammar::effects::UnlessPaymentKind::LifeEqualToItsToughness => {
                    let value = Value::ToughnessOf(Box::new(
                        crate::reference_helpers::choose_spec_for_target(&target),
                    ));
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::life(value))
                }
                crate::grammar::effects::UnlessPaymentKind::Cost => {
                    crate::families::activation_and_restrictions::parse_payment_clause_as_total_cost(
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
                before_delayed_step: false,
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
        shapes::DestroyClauseKind::TargetAndAttached(shape) => {
            let target = parse_target_phrase(shape.target_tokens)?;
            let mut attachment_filter = parse_object_filter(shape.attachment_filter_tokens, false)?;
            attachment_filter.set_demonstrative_antecedent_surface(shape.demonstrative_antecedent);
            let target_tag = helper_tag_for_tokens(tokens, "destroy_attachment_target");
            let tagged_target =
                TargetAst::Tagged(target_tag.clone(), span_from_tokens(shape.target_tokens));

            EffectAst::Sequence {
                effects: vec![
                    EffectAst::TagAffected {
                        effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
                        tag: target_tag,
                    },
                    EffectAst::subject_verb_destroy_all_attached_to(
                        attachment_filter,
                        tagged_target.clone(),
                    ),
                    EffectAst::subject_verb_destroy(tagged_target),
                ],
            }
        }
        shapes::DestroyClauseKind::InlineNoRegeneration { target_tokens } => {
            EffectAst::subject_verb_destroy_no_regeneration(parse_target_phrase(target_tokens)?)
        }
        shapes::DestroyClauseKind::MultiTarget => {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{original_clause}')"
            )));
        }
        shapes::DestroyClauseKind::Blocked { target_tokens } => {
            EffectAst::subject_verb_destroy(parse_target_phrase(&target_tokens)?)
        }
        shapes::DestroyClauseKind::Plain { target_tokens } => {
            // "Destroy target creature of an opponent's choice" delegates the
            // target choice to an opponent: declare the target with that
            // chooser, then destroy the declared object.
            if let Some(choice_shape) =
                crate::grammar::choices::parse_possessive_object_choice_tokens(
                    target_tokens,
                )
                && choice_shape.actor
                    == crate::grammar::choices::PossessiveObjectChoiceActor::Opponent
            {
                let target = parse_target_phrase(&choice_shape.object_tokens)?;
                EffectAst::Sequence {
                    effects: vec![
                        EffectAst::subject_verb_explicit_target_only_for_chooser(
                            target,
                            PlayerAst::Opponent,
                        ),
                        EffectAst::subject_verb_destroy(TargetAst::Tagged(IT_TAG.into(), None)),
                    ],
                }
            } else {
                EffectAst::subject_verb_destroy(parse_target_phrase(target_tokens)?)
            }
        }
    };
    Ok(wrap_destroy_with_delayed_timing(effect, timing))
}

pub(crate) fn apply_except_filter_exclusions(base: &mut ObjectFilter, exception: &ObjectFilter) {
    for branch in &exception.any_of {
        apply_except_filter_exclusions(base, branch);
    }
    // A proper-name self-reference in an exception denotes the source object,
    // not every permanent that happens to share its name. Preserve that as the
    // ordinary source-identity exclusion (`other`) while retaining the authored
    // name solely as surface metadata.
    if exception.source {
        base.other = true;
        base.source_surface = exception.source_surface.clone();
    }
    // Literal named exceptions are name predicates rather than source
    // references. Carry those structurally instead of silently dropping them.
    if let Some(name) = &exception.name {
        base.excluded_name = Some(name.clone());
    }
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
    // "except for commanders" (Slash the Ranks) — previously dropped
    // silently, which destroyed commanders at runtime.
    if exception.is_commander {
        base.noncommander = true;
    }
    if exception.token {
        base.nontoken = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::runtime_backend::clause_support::parse_effect_sentences_lexed;
    use crate::{CardType, Subtype};

    #[test]
    fn destroy_all_except_lands_and_tokens_transports_both_union_exclusions() {
        let tokens = crate::runtime_backend::lex_line(
            "Destroy all other permanents except for lands and tokens.",
            0,
        )
        .expect("destroy exception should lex");
        let effect = parse_destroy(&tokens).expect("destroy exception should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Destroy { target, .. },
            ..
        }) = effect
        else {
            panic!("expected typed destroy-all action: {effect:#?}");
        };
        let TargetAst::Object(filter, ..) = target else {
            panic!("expected object filter: {target:#?}");
        };
        assert!(filter.other, "{filter:#?}");
        assert!(filter.nontoken, "{filter:#?}");
        assert!(filter.excluded_card_types.contains(&CardType::Land));
    }

    #[test]
    fn destroy_all_single_exception_does_not_synthesize_the_other_exception() {
        fn parsed_filter(text: &str) -> ObjectFilter {
            let tokens =
                crate::runtime_backend::lex_line(text, 0).expect("destroy exception should lex");
            let effect = parse_destroy(&tokens).expect("destroy exception should parse");
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Destroy { target, .. },
                ..
            }) = effect
            else {
                panic!("expected typed destroy-all action: {effect:#?}");
            };
            let TargetAst::Object(filter, ..) = target else {
                panic!("expected object filter: {target:#?}");
            };
            filter
        }

        let lands = parsed_filter("Destroy all other permanents except for lands.");
        assert_eq!(lands.excluded_card_types, [CardType::Land]);
        assert!(
            !lands.nontoken,
            "lands-only exception must still destroy tokens"
        );

        let tokens = parsed_filter("Destroy all other permanents except for tokens.");
        assert!(tokens.excluded_card_types.is_empty());
        assert!(
            tokens.nontoken,
            "tokens-only exception must still destroy lands"
        );
    }

    #[test]
    fn destroy_all_plural_shared_color_keeps_tagged_relation() {
        let tokens = crate::runtime_backend::lex_line(
            "Destroy all other creatures that share a color with it.",
            0,
        )
        .expect("shared-color destroy clause should lex");
        let effect = parse_destroy(&tokens).expect("shared-color destroy clause should parse");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("SharesColorWithTagged"), "{debug}");
        assert!(debug.contains(IT_TAG), "{debug}");
    }

    #[test]
    fn repeated_or_if_destroy_condition_is_one_disjunction() {
        let tokens = crate::runtime_backend::lex_line(
            "target nonland permanent if it's a creature or if {G}{W} was spent to cast this spell",
            0,
        )
        .expect("conditional destroy should lex");
        let effect = parse_destroy(&tokens).expect("conditional destroy should parse");
        let EffectAst::Conditional {
            predicate: PredicateAst::Or(left, right),
            if_true,
            if_false,
        } = effect
        else {
            panic!("expected one disjunctive condition: {effect:#?}");
        };
        assert!(if_false.is_empty());
        assert_eq!(if_true.len(), 1);
        assert!(matches!(*left, PredicateAst::ItMatches(_)));
        assert!(matches!(
            *right,
            PredicateAst::And(_, _) | PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
        ));
    }

    #[test]
    fn destroy_all_except_named_source_keeps_identity_exclusion_and_regeneration_rider() {
        let effects = crate::runtime_backend::util::with_card_source_reference_context(
            "Mageta the Lion",
            &[CardType::Creature],
            &[Subtype::Human, Subtype::Spellshaper],
            || {
                let tokens = crate::runtime_backend::lex_line(
                    "Destroy all creatures except for Mageta. Those creatures can't be regenerated.",
                    0,
                )
                .expect("Mageta destroy clause should lex");
                parse_effect_sentences_lexed(&tokens)
                    .expect("Mageta destroy and regeneration rider should parse together")
            },
        );

        let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
            panic!("expected one destroy-all effect, got {effects:#?}");
        };
        let SubjectVerbActionAst::DestroyAll {
            filter,
            no_regeneration,
            ..
        } = &subject_verb.action
        else {
            panic!("expected destroy-all action, got {subject_verb:#?}");
        };
        assert!(
            *no_regeneration,
            "the destroyed set must ignore regeneration"
        );
        assert!(
            filter.other,
            "the source object must be excluded by identity"
        );
        assert_eq!(
            filter
                .source_surface
                .as_ref()
                .map(|surface| surface.display_text())
                .as_deref(),
            Some("Mageta")
        );
    }

    #[test]
    fn inline_same_object_regeneration_rider_sets_destroy_semantics() {
        let tokens =
            crate::runtime_backend::lex_line("target Knight and it can't be regenerated", 0)
                .expect("destroy clause should lex");
        let effect = parse_destroy(&tokens).expect("inline rider should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            panic!("expected one destroy action, got {effect:#?}");
        };
        let SubjectVerbActionAst::Destroy {
            target,
            no_regeneration,
            ..
        } = action
        else {
            panic!("expected a destroy action, got {action:#?}");
        };
        assert!(no_regeneration);
        assert!(
            matches!(target, TargetAst::Object(filter, _, _) if filter.subtypes == [Subtype::Knight])
        );
    }
}
