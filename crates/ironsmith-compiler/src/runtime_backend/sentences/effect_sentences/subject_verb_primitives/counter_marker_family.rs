use super::*;
use crate::runtime_backend::front_end::grammar::effects::counter_marker_shapes as counter_shapes;

fn subject_verb_put_counters_target(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::PutCounters { target, .. } => Some(target.clone()),
            SubjectVerbActionAst::PutCounterChoice { target, .. } => Some(target.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn retarget_it_target_for_counter_followup(target: &mut TargetAst, source_target: &TargetAst) {
    match target {
        TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG => {
            *target = source_target.clone();
        }
        TargetAst::Object(filter, _, _)
            if *filter == ObjectFilter::tagged(TagKey::from(IT_TAG)) =>
        {
            *target = source_target.clone();
        }
        TargetAst::WithCount(inner, _) => {
            retarget_it_target_for_counter_followup(inner, source_target);
        }
        _ => {}
    }
}

fn retarget_it_filter_for_counter_followup(
    filter: &mut ObjectFilter,
    source_filter: &ObjectFilter,
) {
    if *filter == ObjectFilter::tagged(TagKey::from(IT_TAG)) {
        *filter = source_filter.clone();
        return;
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        retarget_it_filter_for_counter_followup(targets, source_filter);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        retarget_it_filter_for_counter_followup(targets, source_filter);
    }
    for branch in &mut filter.any_of {
        retarget_it_filter_for_counter_followup(branch, source_filter);
    }
}

fn retarget_it_restriction_for_counter_followup(
    restriction: &mut crate::effect::Restriction,
    source_filter: &ObjectFilter,
) {
    use crate::effect::Restriction;

    match restriction {
        Restriction::Attack(filter)
        | Restriction::Block(filter)
        | Restriction::MustBeBlocked(filter)
        | Restriction::BlockAlone(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeRegenerated(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter)
        | Restriction::Transform(filter)
        | Restriction::PhaseOut(filter)
        | Restriction::AttackOrBlock(filter)
        | Restriction::AttackOrBlockAlone(filter)
        | Restriction::ActivateAbilitiesOf(filter)
        | Restriction::ActivateTapAbilitiesOf(filter)
        | Restriction::ActivateNonManaAbilitiesOf(filter) => {
            retarget_it_filter_for_counter_followup(filter, source_filter);
        }
        Restriction::BlockSpecificAttacker { blockers, attacker }
        | Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            retarget_it_filter_for_counter_followup(blockers, source_filter);
            retarget_it_filter_for_counter_followup(attacker, source_filter);
        }
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, .. }
        | Restriction::CastSpellsMatching(_, attackers)
        | Restriction::CastMoreThanOneSpellEachTurn(_, attackers) => {
            retarget_it_filter_for_counter_followup(attackers, source_filter);
        }
        Restriction::BeTargetedFrom(target, source) => {
            retarget_it_filter_for_counter_followup(target, source_filter);
            retarget_it_filter_for_counter_followup(source, source_filter);
        }
        Restriction::BeTargetedPlayerFrom(_, source) => {
            retarget_it_filter_for_counter_followup(source, source_filter);
        }
        _ => {}
    }
}

fn retarget_it_effect_for_counter_followup(effect: &mut EffectAst, source_target: &TargetAst) {
    let source_filter = target_ast_to_object_filter(source_target.clone());
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
            SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                retarget_it_target_for_counter_followup(target, source_target);
            }
            SubjectVerbActionAst::Cant { restriction, .. } => {
                if let Some(source_filter) = source_filter.as_ref() {
                    retarget_it_restriction_for_counter_followup(restriction, source_filter);
                }
            }
            _ => {}
        },
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            for nested in if_true.iter_mut().chain(if_false.iter_mut()) {
                retarget_it_effect_for_counter_followup(nested, source_target);
            }
        }
        _ => {}
    }
}

fn parse_put_counter_choice_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_counter_choice_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    let target_phrase =
        crate::runtime_backend::front_end::lexer::render_token_slice(shape.target_tokens);
    let mode_texts = shape
        .counter_types
        .iter()
        .map(|counter_type| {
            format!(
                "Put {} on {target_phrase}",
                super::super::zone_counter_helpers::describe_counter_phrase_for_mode(
                    1,
                    *counter_type,
                )
            )
        })
        .collect();

    Ok(Some(vec![EffectAst::subject_verb_put_counter_choice(
        shape.counter_types,
        Value::Fixed(1),
        mode_texts,
        target,
        None,
    )]))
}

pub(crate) fn parse_sentence_sacrifice_at_end_of_combat(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_sacrifice_at_end_of_combat_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let filter = if shape.tagged_object {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(shape.object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            filter,
            1,
            None,
        )],
    }]))
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_for_each_counter_kind_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;

    Ok(Some(vec![
        EffectAst::subject_verb_for_each_counter_kind_put_or_remove(target),
    ]))
}

fn lower_counter_placements(
    placements: Vec<counter_shapes::CounterPlacementShape<'_>>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::with_capacity(placements.len());
    for placement in placements {
        let target = parse_target_phrase(placement.target_tokens)?;
        effects.push(EffectAst::subject_verb_put_counters(
            placement.descriptor.counter_type,
            Value::Fixed(placement.descriptor.count as i32),
            target,
            None,
            false,
        ));
    }

    Ok(effects)
}

pub(crate) fn parse_sentence_put_counter_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_counter_sequence_tokens(clause.tokens()) else {
        return Ok(None);
    };
    if let counter_shapes::PutCounterSequenceShape::Then {
        head_tokens,
        tail_tokens,
    } = shape
    {
        let mut effects = parse_effect_chain(head_tokens)?;
        if effects.is_empty() {
            return Ok(None);
        }
        effects.extend(parse_effect_chain(tail_tokens)?);
        return Ok(Some(effects));
    }

    if let Some(placements) =
        counter_shapes::parse_counter_placement_sequence_tokens(clause.tokens())
    {
        return lower_counter_placements(placements).map(Some);
    }

    if let Some(effects) = parse_put_counter_choice_sequence(clause)? {
        return Ok(Some(effects));
    }

    if let Some(shape) = counter_shapes::parse_shared_counter_target_tokens(clause.tokens()) {
        let target = parse_target_phrase(shape.target_tokens)?;
        let effects = shape
            .descriptors
            .into_iter()
            .map(|descriptor| {
                EffectAst::subject_verb_put_counters(
                    descriptor.counter_type,
                    Value::Fixed(descriptor.count as i32),
                    target.clone(),
                    None,
                    false,
                )
            })
            .collect();
        return Ok(Some(effects));
    }

    if let Some(shape) = counter_shapes::parse_counter_followup_tokens(clause.tokens()) {
        if let Ok(first) = parse_put_counters(shape.counter_tokens)
            && let Ok(mut followup_effects) = parse_effect_chain(shape.followup_tokens)
            && !followup_effects.is_empty()
        {
            let source_target = match &first {
                effect if subject_verb_put_counters_target(effect).is_some() => {
                    subject_verb_put_counters_target(effect)
                }
                EffectAst::Conditional { if_true, .. } if if_true.len() == 1 => {
                    if_true.first().and_then(subject_verb_put_counters_target)
                }
                _ => None,
            };

            if let Some(source_target) = source_target {
                for effect in &mut followup_effects {
                    retarget_it_effect_for_counter_followup(effect, &source_target);
                }

                let mut effects = vec![first];
                effects.append(&mut followup_effects);
                return Ok(Some(vec![EffectAst::Coordinated {
                    effects,
                    leading_duration: false,
                    result_conjunction: false,
                }]));
            }
        }
    }

    if let Some(shape) = counter_shapes::parse_counter_pair_tokens(clause.tokens())
        && let (Ok(first), Ok(second)) = (
            parse_put_counters(shape.first_tokens),
            parse_put_counters(shape.second_tokens),
        )
    {
        return Ok(Some(vec![first, second]));
    }

    Ok(None)
}

pub(crate) fn is_pump_like_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Pump { .. }
                | SubjectVerbActionAst::PumpByLastEffect { .. }
                | SubjectVerbActionAst::SetBasePowerToughness { .. }
                | SubjectVerbActionAst::SetBasePower { .. },
            ..
        })
    )
}

pub(crate) fn parse_gets_then_fights_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_gets_then_fights_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let pump_effect = parse_effect_clause(shape.pump_tokens)?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let creature1 = parse_target_phrase(shape.first_target_tokens)?;
    let creature2 = parse_target_phrase(shape.second_target_tokens)?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::subject_verb_fight(creature1, creature2),
    ]))
}

pub(crate) fn parse_sentence_gets_then_fights(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(clause)
}

pub(crate) fn parse_return_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_return_with_counters_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    let return_effect = if shape.destination.attacking {
        if shape.destination.transformed {
            return Err(CardTextError::ParseError(format!(
                "unsupported transformed attacking return-with-counter destination (clause: '{}')",
                clause.text()
            )));
        }
        EffectAst::subject_verb_move_to_zone_with_attacking(
            target,
            Zone::Battlefield,
            false,
            shape.destination.controller,
            shape.destination.tapped,
            true,
            false,
            None,
        )
        .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
    } else {
        EffectAst::subject_verb_return_to_battlefield(
            target,
            shape.destination.tapped,
            shape.destination.transformed,
            false,
            shape.destination.controller,
            None,
        )
    };
    let mut effects = vec![return_effect];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());
    for descriptor in shape.descriptors {
        let count = Value::Fixed(descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter);
        let count = if descriptor.additional {
            count.with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
        } else {
            count
        };
        effects.push(EffectAst::subject_verb_put_counters(
            descriptor.counter_type,
            count,
            tagged_target.clone(),
            None,
            false,
        ));
    }

    let wrapped = if let Some(timing) = shape.timing {
        match timing {
            counter_shapes::CounterMarkerTimingShape::NextEndStep(player) => {
                vec![EffectAst::DelayedUntilNextEndStep { player, effects }]
            }
            counter_shapes::CounterMarkerTimingShape::NextUpkeep(player) => {
                vec![EffectAst::DelayedUntilNextUpkeep { player, effects }]
            }
            counter_shapes::CounterMarkerTimingShape::EndOfCombat => {
                vec![EffectAst::DelayedUntilEndOfCombat { effects }]
            }
        }
    } else {
        effects
    };

    Ok(Some(wrapped))
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        counter_shapes::parse_put_onto_battlefield_with_counters_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    if shape.destination.tapped || shape.destination.attacking {
        return Ok(None);
    }

    let target = parse_target_phrase(shape.target_tokens)?;
    let move_effect = if shape
        .target_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "all" | "each"))
    {
        EffectAst::subject_verb_move_all_to_zone(
            target,
            Zone::Battlefield,
            false,
            shape.destination.controller,
            false,
            None,
        )
    } else {
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Battlefield,
            false,
            shape.destination.controller,
            false,
            None,
        )
    }
    .with_exiled_with_source_surface(
        super::super::verb_handlers::parse_exiled_with_source_move_surface(clause.tokens()),
    );
    let move_effect = if shape.destination.transformed {
        move_effect.with_move_to_zone_transformed()
    } else {
        move_effect
    };
    let mut effects = vec![move_effect];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());
    for descriptor in shape.descriptors {
        let count = Value::Fixed(descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter);
        let count = if descriptor.additional {
            count.with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
        } else {
            count
        };
        effects.push(EffectAst::subject_verb_put_counters(
            descriptor.counter_type,
            count,
            tagged_target.clone(),
            None,
            false,
        ));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(clause)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(clause)
}

pub(crate) fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

pub(crate) fn clone_return_effect_with_subtype(
    base: &EffectAst,
    subtype: Subtype,
) -> Option<EffectAst> {
    match base {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ReturnToHand {
                target,
                random,
                destination_player_surface,
                exiled_with_source_surface,
                set_quantifier_surface,
                set_reference_surface,
            } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then_some(
                    EffectAst::subject_verb_return_to_hand(cloned_target, *random)
                        .with_return_destination_player_surface(*destination_player_surface)
                        .with_exiled_with_source_surface(exiled_with_source_surface.clone())
                        .with_return_set_quantifier_surface(*set_quantifier_surface)
                        .with_return_set_reference_surface(set_reference_surface.clone()),
                )
            }
            SubjectVerbActionAst::ReturnAllToHand {
                filter,
                destination_player_surface,
                exiled_with_source_surface,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(
                    EffectAst::subject_verb_return_all_to_hand(cloned_filter)
                        .with_return_destination_player_surface(*destination_player_surface)
                        .with_exiled_with_source_surface(exiled_with_source_surface.clone()),
                )
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
                top_only,
            } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then(|| {
                    let mut effect = EffectAst::subject_verb_return_to_battlefield(
                        cloned_target,
                        *tapped,
                        *transformed,
                        *converted,
                        *controller,
                        count_value.clone(),
                    )
                    .with_top_only_return_choice(*top_only);
                    if let EffectAst::SubjectVerb(subject_verb) = &mut effect
                        && let SubjectVerbActionAst::ReturnToBattlefield { as_aura: dst, .. } =
                            &mut subject_verb.action
                    {
                        *dst = as_aura.clone();
                    }
                    effect
                })
            }
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(
                    EffectAst::subject_verb_return_all_to_battlefield(
                        cloned_filter,
                        *tapped,
                        *face_down,
                        *controller,
                    )
                    .with_move_to_zone_verb_surface(*verb_surface),
                )
            }
            _ => None,
        },
        _ => None,
    }
}
pub(crate) fn parse_draw_then_connive_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_draw_then_connive_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.draw_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(shape.connive_tokens)? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_draw_then_connive(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(clause)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_if_enters_additional_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let put_counter = EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    );
    let apply_only_if_creature = EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(ObjectFilter::creature()),
        if_true: vec![put_counter],
        if_false: Vec::new(),
    };

    Ok(Some(vec![EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![apply_only_if_creature],
    }]))
}

pub(crate) fn parse_tagged_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_tagged_enters_additional_tokens(clause.tokens()) else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    )]))
}

pub(crate) fn parse_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_with_additional_tokens(clause.tokens()) else {
        return Ok(None);
    };
    lower_put_with_additional_counter(shape, clause.span()).map(Some)
}

fn lower_put_with_additional_counter(
    shape: counter_shapes::PutWithAdditionalCounterShape<'_>,
    span: Option<TextSpan>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = parse_effect_chain_inner(shape.move_tokens)?;
    if effects.is_empty()
        || !effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::MoveToZone {
                        zone: Zone::Battlefield,
                        ..
                    } | SubjectVerbActionAst::ReturnToBattlefield { .. }
                        | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                    ..
                })
            )
        })
    {
        return Ok(Vec::new());
    }

    effects.push(EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter),
        TargetAst::Tagged(TagKey::from(IT_TAG), span),
        None,
        false,
    ));

    Ok(effects)
}

pub(crate) fn parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_sacrifice_then_put_additional_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    lower_sacrifice_then_put_additional(shape, clause.span()).map(Some)
}

fn lower_sacrifice_then_put_additional(
    shape: counter_shapes::SacrificeThenPutAdditionalShape<'_>,
    span: Option<TextSpan>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut put_effects = lower_put_with_additional_counter(shape.put, span)?;
    if put_effects.is_empty() {
        return Ok(Vec::new());
    }
    let mut effects = if shape.plain_word_sacrifice {
        vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            ObjectFilter {
                source: true,
                ..Default::default()
            },
            1,
            None,
        )]
    } else {
        parse_effect_chain_inner(shape.sacrifice_tokens)?
    };
    if effects.is_empty() {
        return Ok(Vec::new());
    }
    effects.append(&mut put_effects);
    Ok(effects)
}

pub(crate) fn parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        counter_shapes::parse_if_sacrifice_then_put_additional_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let effects = lower_sacrifice_then_put_additional(shape.effect, clause.span())?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Conditional {
        predicate: parse_predicate_lexed(shape.predicate_tokens)?,
        if_true: effects,
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_each_player_return_additional_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let mut per_player_effects = parse_effect_chain_inner(shape.return_tokens)?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    ));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}
