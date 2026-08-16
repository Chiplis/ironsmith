use super::*;
use crate::cards::builders::SubjectVerbActionAst;
use crate::effect_sentences::SubjectVerbPrimitiveClause;
fn parse_return_back_reference_target(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if crate::grammar::effects::is_return_back_reference_shape(tokens) {
        let span = span_from_tokens(tokens);
        let words = crate::lexer::token_word_refs(tokens);
        if matches!(words.as_slice(), ["that" | "those", noun] if crate::util::is_demonstrative_object_head(noun))
        {
            crate::util::record_source_reference_surface(
                span,
                crate::target::SourceReferenceSurface::ThisPermanentType(words.join(" ")),
            );
        }
        Ok(TargetAst::Tagged(TagKey::from(IT_TAG), span))
    } else {
        parse_target_phrase(tokens)
    }
}

fn set_return_destination_first_surface(target: &mut TargetAst, destination_first: bool) {
    match target {
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            filter.set_return_destination_first_surface(destination_first);
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            set_return_destination_first_surface(inner, destination_first);
        }
        _ => {}
    }
}

fn strip_except_this_card_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    if tokens.len() >= 3
        && tokens[tokens.len() - 3].is_word("except")
        && tokens[tokens.len() - 2].is_word("this")
        && tokens[tokens.len() - 1].is_word("card")
    {
        (&tokens[..tokens.len() - 3], true)
    } else {
        (tokens, false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DelayedReturnTimingAst {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

pub(crate) fn parse_delayed_return_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    crate::grammar::effects::parse_return_timing_words_shape(words).map(|shape| match shape {
        crate::grammar::effects::ReturnTimingShape::NextEndStep(player) => {
            DelayedReturnTimingAst::NextEndStep(player)
        }
        crate::grammar::effects::ReturnTimingShape::NextUpkeep(player) => {
            DelayedReturnTimingAst::NextUpkeep(player)
        }
        crate::grammar::effects::ReturnTimingShape::EndOfCombat => {
            DelayedReturnTimingAst::EndOfCombat
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
    if let Some(for_each_idx) = (0..tokens.len().saturating_sub(1))
        .rev()
        .find(|&idx| tokens[idx].is_word("for") && tokens[idx + 1].is_word("each"))
    {
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
    if shape.has_unless {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-unless clause (clause: '{clause_text}')"
        )));
    }

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
            let (filter_tokens, excludes_source) = strip_except_this_card_suffix(&filter_tokens);
            // The `exiled with this <source>` relative clause identifies the
            // source-linked set; it is not a characteristic restriction on
            // the returned cards. Parse only the authored noun phrase before
            // `exiled`, then represent the relationship with SOURCE_EXILED_TAG.
            // This prevents a source type such as "Vehicle" or "Saga" from
            // leaking into the selected-card filter.
            let source_linked_subject = exiled_with_source_surface
                .as_ref()
                .and_then(|_| {
                    filter_tokens
                        .iter()
                        .position(|token| token.is_word("exiled"))
                })
                .map(|exiled_idx| &filter_tokens[..exiled_idx])
                .unwrap_or(filter_tokens);
            let source_linked_excludes_current = exiled_with_source_surface.is_some()
                && source_linked_subject
                    .iter()
                    .any(|token| token.is_word("other"));
            let mut filter = parse_object_filter(source_linked_subject, false)?;
            if exiled_with_source_surface.is_some() {
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
                    } else if count.is_some() || exiled_with_source_surface.is_some() {
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
                && words
                    .windows(3)
                    .any(|window| window == ["from", "your", "graveyard"])
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
    let mut effect = effect.with_exiled_with_source_surface(exiled_with_source_surface);
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
pub(crate) fn parse_exchange(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    use crate::grammar::effects::{
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

    let clause_text = crate::lexer::token_word_refs(tokens).join(" ");
    let shape = crate::grammar::effects::parse_exchange_clause_shape(tokens).ok_or_else(|| {
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
            let (left, right) = crate::grammar::effects::parse_exchange_value_operands(&remainder)
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
            let controller_set =
                crate::grammar::targets::parse_target_controller_set_suffix(control.filter_tokens);
            let mut filter = parse_object_filter(&controller_set.core_tokens, false)?;
            match controller_set.constraint {
                crate::grammar::targets::TargetControllerSetConstraint::None => {}
                crate::grammar::targets::TargetControllerSetConstraint::SameController => {
                    filter.target_set_same_controller = true;
                }
                crate::grammar::targets::TargetControllerSetConstraint::DifferentControllers => {
                    filter.target_set_different_controllers = true;
                }
            }
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
    use crate::effect_sentences::parse_effect_sentence_lexed;
    use crate::lexer::lex_line;
    use crate::model::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::types::CardType;

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

    #[test]
    fn source_linked_return_tail_excludes_only_the_current_exile_result() {
        let tokens = lex_line(
            "each other card exiled with this Vehicle to the battlefield under its owner's control",
            0,
        )
        .expect("lex source-linked return clause");
        let effect = parse_return(&tokens).expect("parse source-linked return clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    target: TargetAst::Object(filter, None, _),
                    zone: Zone::Battlefield,
                    all: true,
                    exiled_with_source_surface: Some(surface),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected a source-linked bulk move: {effect:#?}");
        };

        assert_eq!(filter.zone, Some(Zone::Exile));
        assert!(
            !filter.other,
            "`other` is result-relative, not source-relative"
        );
        assert!(filter.card_types.is_empty(), "{filter:#?}");
        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::cards::builders::IT_TAG
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::Custom("each other card".to_string())
        );
    }

    #[test]
    fn exchange_target_preserves_joint_negative_owner_and_controller_predicates() {
        let tokens = lex_line(
            "control of this enchantment and target permanent you neither own nor control",
            0,
        )
        .expect("lex heterogeneous exchange clause");
        let effect = parse_exchange(&tokens, None).expect("parse heterogeneous exchange clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ExchangeControlHeterogeneous {
                    permanent2: TargetAst::Object(filter, Some(_), _),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected heterogeneous source/target exchange: {effect:#?}");
        };
        assert_eq!(filter.owner, Some(PlayerFilter::NotYou));
        assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
        assert_eq!(
            filter.description(),
            "permanent you neither own nor control"
        );
    }

    #[test]
    fn exchange_target_preserves_different_controller_set_constraint() {
        let tokens = lex_line(
            "control of two target creatures controlled by different players",
            0,
        )
        .expect("lex homogeneous exchange clause");
        let effect = parse_exchange(&tokens, None).expect("parse homogeneous exchange clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ExchangeControl {
                    filter, count: 2, ..
                },
            ..
        }) = effect
        else {
            panic!("expected one counted exchange target set: {effect:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert!(filter.target_set_different_controllers, "{filter:#?}");
        assert!(!filter.target_set_same_controller, "{filter:#?}");
    }

    #[test]
    fn preserves_destination_first_surface_on_a_singular_graveyard_target() {
        let tokens = lex_line(
            "to your hand target artifact card in your graveyard with lesser mana value",
            0,
        )
        .expect("lex destination-first return clause");
        let effect = parse_return(&tokens).expect("parse destination-first return clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnToHand { target, .. },
            ..
        }) = effect
        else {
            panic!("expected a singular hand return");
        };
        let TargetAst::Object(filter, Some(_), _) = target else {
            panic!("expected a targeted graveyard object filter");
        };

        assert!(filter.has_return_destination_first_surface());
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.card_types, [CardType::Artifact]);
    }

    #[test]
    fn destination_first_return_preserves_branch_scoped_collection() {
        let tokens = lex_line(
            "to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control",
            0,
        )
        .expect("lex destination-first branch-scoped return clause");
        let effect = parse_return(&tokens).expect("parse branch-scoped return clause");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnAllToHand { filter, .. },
            ..
        }) = effect
        else {
            panic!("expected a bulk hand return");
        };

        assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
        assert!(filter.has_return_destination_first_surface(), "{filter:#?}");
    }

    #[test]
    fn full_return_sentence_preserves_branch_scoped_collection() {
        let tokens = lex_line(
            "Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control.",
            0,
        )
        .expect("lex full branch-scoped return sentence");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("parse full branch-scoped return sentence");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnAllToHand { filter, .. },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one bulk hand return, got {effects:#?}");
        };

        assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    }

    #[test]
    fn each_player_destination_first_return_keeps_graveyard_history() {
        let tokens = lex_line(
            "Each player returns to the battlefield all artifact, creature, enchantment, and land cards in their graveyard that were put there from the battlefield this turn.",
            0,
        )
        .expect("lex each-player historical return sentence");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("parse each-player historical return");
        let [EffectAst::ForEachPlayer { effects }] = effects.as_slice() else {
            panic!("expected an each-player return, got {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one return-all action, got {effects:#?}");
        };

        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(
            filter.owner,
            Some(PlayerFilter::IteratedPlayer),
            "{filter:#?}"
        );
        assert_eq!(
            filter.card_types,
            [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
            ],
            "{filter:#?}"
        );
        assert!(filter.entered_graveyard_this_turn, "{filter:#?}");
        assert!(
            filter.entered_graveyard_from_battlefield_this_turn,
            "{filter:#?}"
        );
        assert!(filter.has_return_destination_first_surface(), "{filter:#?}");
    }

    #[test]
    fn return_for_each_discarded_card_repeats_from_exact_prior_effect() {
        let tokens = lex_line(
            "a card from your graveyard to your hand for each card discarded this way",
            0,
        )
        .expect("lex return-for-each clause");
        let effect = parse_return(&tokens).expect("parse return-for-each clause");
        let EffectAst::RepeatEffects { count, effects } = effect else {
            panic!("expected repeated return effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            count.unhinted(),
            Value::PendingPriorEffectMetric(query)
                if query.action == Some(ironsmith_core::PriorEffectAction::Discarded)
        ));
    }
}
