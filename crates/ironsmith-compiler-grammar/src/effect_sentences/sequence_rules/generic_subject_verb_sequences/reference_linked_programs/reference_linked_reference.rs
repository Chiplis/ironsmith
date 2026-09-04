use super::*;

pub(super) fn parse_copy_for_each_candidate_filter(
    tokens: &[OwnedLexToken],
) -> Result<(Option<ObjectFilter>, Option<PlayerFilter>, bool), CardTextError> {
    let Some(shape) = effect_grammar::parse_copy_candidate_shape(tokens) else {
        return Ok((None, None, false));
    };
    let candidate_tokens = trim_commas(&tokens[shape.candidate]);
    if shape.kind == effect_grammar::CopyCandidateKind::PlayerOrPermanent {
        return Ok((
            Some(ObjectFilter::permanent()),
            Some(PlayerFilter::Any),
            shape.exclude_current_targets,
        ));
    }
    if shape.kind == effect_grammar::CopyCandidateKind::Player {
        return Ok((None, Some(PlayerFilter::Any), shape.exclude_current_targets));
    }

    let mut filter = parse_object_filter_lexed(&candidate_tokens, false)?;
    filter.other = false;
    filter.could_be_targeted_by = None;
    Ok((Some(filter), None, shape.exclude_current_targets))
}

pub(crate) fn parse_copy_for_each_target_sentence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_commas(tokens);
    let Some(shape) = effect_grammar::parse_copy_for_each_shape(&tokens) else {
        return Ok(None);
    };
    let wrap_if_result = shape.wrap_if_result;
    let (target, object_filter, player_filter, player, exclude_current_targets) = match shape.layout
    {
        effect_grammar::CopyForEachLayout::CopyThenForEach {
            subject,
            target,
            candidate,
        } => {
            let player = match parse_subject(&tokens[subject]) {
                SubjectAst::Player(player) => player,
                SubjectAst::This => PlayerAst::Implicit,
                SubjectAst::TriggeringSourceController => return Ok(None),
            };
            let target_tokens = trim_commas(&tokens[target]);
            let candidate_tokens = trim_commas(&tokens[candidate]);
            let (object_filter, player_filter, exclude_current_targets) =
                parse_copy_for_each_candidate_filter(&candidate_tokens)?;
            (
                target_for_referenced_stack_object(sentences, sentence_idx, &target_tokens),
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
            )
        }
        effect_grammar::CopyForEachLayout::ForEachThenPutCopy { target, candidate } => {
            let target_tokens = trim_commas(&tokens[target]);
            let candidate_tokens = trim_commas(&tokens[candidate]);
            let (object_filter, player_filter, exclude_current_targets) =
                parse_copy_for_each_candidate_filter(&candidate_tokens)?;
            (
                target_for_referenced_stack_object(sentences, sentence_idx, &target_tokens),
                object_filter,
                player_filter,
                PlayerAst::Implicit,
                exclude_current_targets,
            )
        }
    };
    let effect = EffectAst::subject_verb_copy_spell_for_each_target(
        target,
        object_filter,
        player_filter,
        player,
        exclude_current_targets,
        Vec::new(),
    );
    Ok(Some(if wrap_if_result {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![effect],
        }
    } else {
        effect
    }))
}

pub fn parse_for_each_tagged_copy_then_copy_targets_it(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some(shape) = effect_grammar::parse_tagged_copy_retarget_shape(
        &first_tokens,
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };
    let copy_target_tokens = trim_commas(&first_tokens[shape.copy_target]);
    let copy_effect = EffectAst::subject_verb_copy_spell(
        target_for_referenced_stack_object(sentences, sentence_idx, &copy_target_tokens),
        Value::Fixed(1),
        PlayerAst::You,
        false,
        false,
        Vec::new(),
    );

    let second_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())?;
    let [
        retarget @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RetargetStackObject { .. },
            ..
        }),
    ] = second_effects.as_slice()
    else {
        return Ok(None);
    };
    let for_each = EffectAst::ForEachTagged {
        tag: crate::tag::CompilerReferenceTag::It.key(),
        effects: vec![copy_effect, retarget.clone()],
    };

    Ok(Some(vec![if shape.wrap_if_result {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![for_each],
        }
    } else {
        for_each
    }]))
}

pub(crate) fn retarget_source_self_animate_effect(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    power,
                    toughness,
                    target,
                    card_types,
                    subtypes,
                    subtype_families,
                    colors,
                    abilities,
                    granted_abilities,
                    preserve_other_types,
                    type_retention_surface,
                    animation_pt_surface,
                    animation_duration_surface,
                    set_quantifier_surface,
                    duration,
                },
            ..
        }) => {
            let target = match target {
                TargetAst::Tagged(tag, span)
                    if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
                {
                    TargetAst::Source(span)
                }
                target => target,
            };
            EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                subtype_families,
                colors,
                abilities,
                granted_abilities,
                preserve_other_types,
                type_retention_surface,
                animation_pt_surface,
                animation_duration_surface,
                duration,
            )
            .with_set_quantifier_surface(set_quantifier_surface)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
        },
        EffectAst::IfResult { predicate, effects } => EffectAst::IfResult {
            predicate,
            effects: effects
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
        },
        other => other,
    }
}

pub(super) fn contains_tagged_source_animation(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    target, duration, ..
                },
            ..
        }) => {
            let self_animate_target = matches!(
                target,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ) || matches!(target, TargetAst::Source(_));
            *duration == crate::effect::Until::EndOfTurn && self_animate_target
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            if_true.iter().any(contains_tagged_source_animation)
                || if_false.iter().any(contains_tagged_source_animation)
        }
        EffectAst::IfResult { effects, .. } => effects.iter().any(contains_tagged_source_animation),
        _ => false,
    }
}

