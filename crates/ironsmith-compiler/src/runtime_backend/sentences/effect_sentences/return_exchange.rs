use super::*;
use crate::runtime_backend::effect_sentences::SubjectVerbPrimitiveClause;
fn parse_return_back_reference_target(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if crate::runtime_backend::grammar::effects::is_return_back_reference_shape(tokens) {
        let span = span_from_tokens(tokens);
        let words = crate::runtime_backend::token_word_refs(tokens);
        if matches!(words.as_slice(), ["that" | "those", noun] if crate::runtime_backend::util::is_demonstrative_object_head(noun))
        {
            crate::runtime_backend::util::record_source_reference_surface(
                span,
                crate::target::SourceReferenceSurface::ThisPermanentType(words.join(" ")),
            );
        }
        Ok(TargetAst::Tagged(
            TagKey::from(IT_TAG),
            span,
        ))
    } else {
        parse_target_phrase(tokens)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DelayedReturnTimingAst {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

pub(crate) fn parse_delayed_return_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    crate::runtime_backend::grammar::effects::parse_return_timing_words_shape(words).map(|shape| {
        match shape {
            crate::runtime_backend::grammar::effects::ReturnTimingShape::NextEndStep(player) => {
                DelayedReturnTimingAst::NextEndStep(player)
            }
            crate::runtime_backend::grammar::effects::ReturnTimingShape::NextUpkeep(player) => {
                DelayedReturnTimingAst::NextUpkeep(player)
            }
            crate::runtime_backend::grammar::effects::ReturnTimingShape::EndOfCombat => {
                DelayedReturnTimingAst::EndOfCombat
            }
        }
    })
}
pub(crate) fn wrap_return_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedReturnTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedReturnTimingAst::NextEndStep(player) => EffectAst::DelayedUntilNextEndStep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::NextUpkeep(player) => EffectAst::DelayedUntilNextUpkeep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
    }
}

pub(crate) fn parse_return(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    let shape = crate::runtime_backend::grammar::effects::parse_return_clause_shape(tokens)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination (clause: '{clause_text}')"
            ))
        })?;
    if shape.has_unless {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-unless clause (clause: '{clause_text}')"
        )));
    }

    let destination = shape.destination;
    if destination.has_unparsed_timing_words {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{clause_text}')"
        )));
    }
    let delayed_timing = destination.timing.map(|timing| match timing {
        crate::runtime_backend::grammar::effects::ReturnTimingShape::NextEndStep(player) => {
            DelayedReturnTimingAst::NextEndStep(player)
        }
        crate::runtime_backend::grammar::effects::ReturnTimingShape::NextUpkeep(player) => {
            DelayedReturnTimingAst::NextUpkeep(player)
        }
        crate::runtime_backend::grammar::effects::ReturnTimingShape::EndOfCombat => {
            DelayedReturnTimingAst::EndOfCombat
        }
    });
    let return_controller = match destination.controller {
        crate::runtime_backend::grammar::effects::ReturnControllerShape::Preserve => {
            ReturnControllerAst::Preserve
        }
        crate::runtime_backend::grammar::effects::ReturnControllerShape::You => {
            ReturnControllerAst::You
        }
        crate::runtime_backend::grammar::effects::ReturnControllerShape::Owner => {
            ReturnControllerAst::Owner
        }
    };
    let attached_to_target = destination
        .attached_to_tokens
        .as_deref()
        .map(parse_return_back_reference_target)
        .transpose()?;

    let effect = match shape.target {
        crate::runtime_backend::grammar::effects::ReturnTargetShape::PairedSourceAndExiled {
            source_subtype,
        } => {
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
        crate::runtime_backend::grammar::effects::ReturnTargetShape::UntargetedExiledCards {
            filter_tokens,
            count,
        } => {
            let mut filter = parse_object_filter(&filter_tokens, false)?;
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
            match destination.zone {
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Battlefield => {
                    EffectAst::subject_verb_return_all_to_battlefield(
                        filter,
                        destination.tapped,
                        false,
                        return_controller,
                    )
                }
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Graveyard => {
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
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Hand => {
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
        crate::runtime_backend::grammar::effects::ReturnTargetShape::MultiTargetUnsupported => {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{clause_text}')"
            )));
        }
        crate::runtime_backend::grammar::effects::ReturnTargetShape::All {
            raw_filter_tokens,
            filter_tokens,
            chosen_this_way_excluded,
            chosen_creature_type,
            excluded_chosen_creature_type,
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
            if destination.zone == crate::runtime_backend::grammar::effects::ReturnZoneShape::Hand
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
            filter.chosen_creature_type |= chosen_creature_type;
            filter.excluded_chosen_creature_type |= excluded_chosen_creature_type;
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
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Battlefield => {
                    EffectAst::subject_verb_return_all_to_battlefield(
                        filter,
                        destination.tapped,
                        false,
                        return_controller,
                    )
                }
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Graveyard => {
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
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Hand => {
                    EffectAst::subject_verb_return_all_to_hand(filter)
                }
            }
        }
        crate::runtime_backend::grammar::effects::ReturnTargetShape::Singular {
            target_tokens,
            source_from_graveyard_tokens,
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
            let mut target = if let Some(target) = source_from_graveyard_target {
                target
            } else if back_reference {
                parse_return_back_reference_target(&target_tokens)?
            } else {
                parse_target_phrase(&target_tokens)?
            };
            let count_value = dynamic_count.then_some(crate::effect::Value::EventValue(
                crate::effect::EventValueSpec::Amount,
            ));
            if dynamic_count {
                target =
                    TargetAst::WithCount(Box::new(target), crate::effect::ChoiceCount::dynamic_x());
            }
            match destination.zone {
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Battlefield => {
                    if let Some(attached_to) = attached_to_target {
                        if destination.transformed || destination.converted || count_value.is_some()
                        {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported transformed/converted/dynamic return attached clause (clause: '{clause_text}')"
                            )));
                        }
                        EffectAst::subject_verb_move_to_zone(
                            target,
                            Zone::Battlefield,
                            false,
                            return_controller,
                            destination.tapped,
                            Some(attached_to),
                        )
                        .with_move_to_zone_verb_surface(
                            ironsmith_core::MoveToZoneVerbSurface::Return,
                        )
                    } else if destination.attacking {
                        EffectAst::subject_verb_move_to_zone_with_attacking(
                            target,
                            Zone::Battlefield,
                            false,
                            return_controller,
                            destination.tapped,
                            true,
                            false,
                            None,
                        )
                        .with_move_to_zone_verb_surface(
                            ironsmith_core::MoveToZoneVerbSurface::Return,
                        )
                    } else {
                        EffectAst::subject_verb_return_to_battlefield(
                            target,
                            destination.tapped,
                            destination.transformed,
                            destination.converted,
                            return_controller,
                            count_value,
                        )
                        .with_top_only_return_choice(top_only)
                    }
                }
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Graveyard => {
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
                crate::runtime_backend::grammar::effects::ReturnZoneShape::Hand => {
                    EffectAst::subject_verb_return_to_hand(target, shape.random)
                }
            }
        }
    };
    let effect =
        if destination.zone == crate::runtime_backend::grammar::effects::ReturnZoneShape::Hand {
            effect.with_return_destination_player_surface(destination.destination_player_surface)
        } else {
            effect
        };
    Ok(wrap_return_with_delayed_timing(effect, delayed_timing))
}
pub(crate) fn parse_exchange(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    use crate::runtime_backend::grammar::effects::{
        ExchangeClauseShape, ExchangeSharedTypeShape, ExchangeValueKindShape,
        ExchangeValueOperandShape,
    };

    fn shared_type(shape: Option<ExchangeSharedTypeShape>) -> Option<SharedTypeConstraintAst> {
        shape.map(|shape| match shape {
            ExchangeSharedTypeShape::PermanentType => SharedTypeConstraintAst::PermanentType,
            ExchangeSharedTypeShape::CardType => SharedTypeConstraintAst::CardType,
        })
    }

    fn value_operand(
        shape: ExchangeValueOperandShape<'_>,
    ) -> Result<ExchangeValueAst, CardTextError> {
        match shape {
            ExchangeValueOperandShape::LifeTotal(player) => Ok(ExchangeValueAst::LifeTotal(player)),
            ExchangeValueOperandShape::SourceStat {
                source_tokens,
                kind,
            } => Ok(ExchangeValueAst::Stat {
                target: TargetAst::Source(span_from_tokens(source_tokens)),
                kind: match kind {
                    ExchangeValueKindShape::Power => ExchangeValueKindAst::Power,
                    ExchangeValueKindShape::Toughness => ExchangeValueKindAst::Toughness,
                },
            }),
            ExchangeValueOperandShape::TargetStat {
                target_tokens,
                kind,
            } => Ok(ExchangeValueAst::Stat {
                target: parse_target_phrase(target_tokens)?,
                kind: match kind {
                    ExchangeValueKindShape::Power => ExchangeValueKindAst::Power,
                    ExchangeValueKindShape::Toughness => ExchangeValueKindAst::Toughness,
                },
            }),
        }
    }

    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    let shape = crate::runtime_backend::grammar::effects::parse_exchange_clause_shape(tokens)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported exchange clause (clause: '{clause_text}')"
            ))
        })?;
    match shape {
        ExchangeClauseShape::LifeTotalsOnly => match subject {
            Some(SubjectAst::Player(PlayerAst::Target)) => Ok(
                EffectAst::subject_verb_exchange_life_totals(PlayerAst::Target, PlayerAst::Target),
            ),
            _ => Err(CardTextError::ParseError(format!(
                "unsupported life-total exchange clause (clause: '{clause_text}')"
            ))),
        },
        ExchangeClauseShape::LifeTotalsWith(player2) => {
            let player1 = match subject {
                Some(SubjectAst::Player(player)) => player,
                _ => PlayerAst::You,
            };
            Ok(EffectAst::subject_verb_exchange_life_totals(
                player1, player2,
            ))
        }
        ExchangeClauseShape::TextBoxes { target_tokens } => {
            let target = parse_target_phrase(target_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported text-box exchange target (clause: '{clause_text}')"
                ))
            })?;
            Ok(EffectAst::subject_verb_exchange_text_boxes(target))
        }
        ExchangeClauseShape::Zones {
            player,
            zone1,
            zone2,
        } => Ok(EffectAst::subject_verb_exchange_zones(player, zone1, zone2)),
        ExchangeClauseShape::Values { tokens } => {
            let (duration, remainder) =
                if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
                    (duration, remainder)
                } else {
                    (Until::Forever, trim_commas(tokens).to_vec())
                };
            let (left, right) =
                crate::runtime_backend::grammar::effects::parse_exchange_value_operands(&remainder)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported exchange value operands (clause: '{clause_text}')"
                        ))
                    })?;
            Ok(EffectAst::subject_verb_exchange_values(
                value_operand(left)?,
                value_operand(right)?,
                duration,
            ))
        }
        ExchangeClauseShape::Control(control) => {
            if control.invalid_shared_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported exchange share-type clause (clause: '{clause_text}')"
                )));
            }
            let constraint = shared_type(control.shared_type);
            if let Some((left_tokens, right_tokens)) = control.heterogeneous {
                let left_target = parse_target_phrase(left_tokens).ok();
                let right_target = parse_target_phrase(right_tokens).ok();
                if let (Some(permanent1), Some(permanent2)) = (left_target, right_target) {
                    return Ok(EffectAst::subject_verb_exchange_control_heterogeneous(
                        permanent1, permanent2, constraint,
                    ));
                }
            }
            if control.filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "missing exchange target filter".to_string(),
                ));
            }
            let filter = parse_object_filter(control.filter_tokens, false)?;
            Ok(EffectAst::subject_verb_exchange_control(
                filter,
                control.count,
                constraint,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_top_graveyard_card_as_a_top_only_return_choice() {
        let tokens = lex_line(
            "the top creature card of your graveyard to the battlefield",
            0,
        )
        .expect("lex return clause");
        let effect = parse_return(&tokens).expect("parse return clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ReturnToBattlefield {
                    target, top_only, ..
                },
            ..
        }) = effect
        else {
            panic!("expected a singular battlefield return");
        };
        let TargetAst::Object(filter, None, _) = target else {
            panic!("expected an untargeted graveyard object filter");
        };

        assert!(top_only);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.card_types, [CardType::Creature]);
    }

    #[test]
    fn preserves_explicit_controller_and_source_link_for_exiled_card_returns() {
        let tokens = lex_line("the exiled cards to the battlefield under your control", 0)
            .expect("lex return clause");
        let effect = parse_return(&tokens).expect("parse return clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ReturnAllToBattlefield {
                    filter, controller, ..
                },
            ..
        }) = effect
        else {
            panic!("expected a bulk battlefield return");
        };

        assert_eq!(controller, ReturnControllerAst::You);
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| { constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG })
        );
    }
}
